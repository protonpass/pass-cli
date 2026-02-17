use crate::callbacks::{AuthEventHandler, CredentialProvider};
use crate::error::AuthError;
use anyhow::{Context, anyhow};
use muon::{GET, POST, Session, Status};
use pass::PassSessionKeyType;
use proton_crypto::srp::SRPProvider;
use std::sync::Arc;

async fn perform_extra_password_auth(
    session: &Session<PassSessionKeyType>,
    password: String,
) -> Result<(), AuthError> {
    let srp_info = get_srp_info(session).await?;

    let provider = proton_crypto::new_srp_provider();
    let proof = provider
        .generate_client_proof(
            "", // username: not used
            &password,
            srp_info.version,
            &srp_info.srp_salt,
            &srp_info.modulus,
            &srp_info.server_ephemeral,
        )
        .context("Error generating client proof")?;

    let proofs = ExtraPasswordProofs {
        client_ephemeral: proof.ephemeral,
        client_proof: proof.proof,
        srp_session_id: srp_info.srp_session_id,
    };
    send_srp_proofs(session, proofs).await?;
    session
        .refresh_auth()
        .await
        .context("Error refreshing session")?;

    Ok(())
}

#[derive(serde::Deserialize)]
struct ExtraPasswordSrpResponse {
    #[serde(rename = "SRPData")]
    data: ExtraPasswordSrpInfo,
}

#[derive(serde::Deserialize)]
struct ExtraPasswordSrpInfo {
    #[serde(rename = "Modulus")]
    modulus: String,
    #[serde(rename = "ServerEphemeral")]
    server_ephemeral: String,
    #[serde(rename = "SrpSessionID")]
    srp_session_id: String,
    #[serde(rename = "Version")]
    version: u8,
    #[serde(rename = "SrpSalt")]
    srp_salt: String,
}

async fn get_srp_info(
    session: &Session<PassSessionKeyType>,
) -> anyhow::Result<ExtraPasswordSrpInfo> {
    let res = session
        .send(GET!("/pass/v1/user/srp/info"))
        .await
        .context("Error requesting SRP info for extra password")?;
    if res.status() != Status::OK {
        return Err(anyhow!("Invalid status code received: {:?}", res.status()));
    }
    let response: ExtraPasswordSrpResponse = res.body_json().context("Error decoding SRP info")?;

    Ok(response.data)
}

#[derive(serde::Serialize)]
struct ExtraPasswordProofs {
    #[serde(rename = "ClientEphemeral")]
    client_ephemeral: String,
    #[serde(rename = "ClientProof")]
    client_proof: String,
    #[serde(rename = "SrpSessionID")]
    srp_session_id: String,
}

async fn send_srp_proofs(
    session: &Session<PassSessionKeyType>,
    proofs: ExtraPasswordProofs,
) -> Result<(), AuthError> {
    let req = POST!("/pass/v1/user/srp/auth")
        .body_json(proofs)
        .context("Error creating SRP request")?;
    let res = session
        .send(req)
        .await
        .context("Error sending SRP proofs")?;
    match res.status() {
        Status::OK => Ok(()),
        Status::BAD_REQUEST => Err(AuthError::BadExtraPassword),
        _ => Err(AuthError::Other(anyhow!(
            "Invalid status code received: {:?}",
            res.status()
        ))),
    }
}

pub async fn handle_extra_password(
    session: &Session<PassSessionKeyType>,
    credential_provider: Arc<dyn CredentialProvider>,
    event_handler: Arc<dyn AuthEventHandler>,
) -> Result<(), anyhow::Error> {
    event_handler.on_extra_password_required().await?;

    let mut attempts = 3;
    loop {
        if attempts == 0 {
            event_handler
                .on_error("Too many incorrect extra password attempts")
                .await?;
            session.logout().await;
            return Err(anyhow!("Error in extra password flow"));
        }

        let extra_password = credential_provider.get_extra_password().await?;
        match perform_extra_password_auth(session, extra_password).await {
            Ok(()) => {
                // Initialize session to verify it works
                session
                    .send(GET!("/tests/ping"))
                    .await
                    .context("Error initializing session")?;
                return Ok(());
            }
            Err(e) => match e {
                AuthError::Other(e) => {
                    return Err(anyhow!("Error in extra password flow: {e:#}"));
                }
                AuthError::BadExtraPassword => {
                    event_handler.on_warning("Incorrect extra password").await?;
                    attempts -= 1;
                }
                AuthError::CannotDecrypt(e) => {
                    return Err(anyhow!("Cannot decrypt: {e:#}"));
                }
            },
        }
    }
}
