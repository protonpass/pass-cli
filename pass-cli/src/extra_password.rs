use crate::client::{get_extra_password, init_session};
use anyhow::{Context, anyhow};
use muon::{GET, POST, Session, Status};
use pass::PassSessionKeyType;
use proton_crypto::srp::SRPProvider;

pub enum ExtraPasswordError {
    BadPassword,
    Other(anyhow::Error),
}

impl From<anyhow::Error> for ExtraPasswordError {
    fn from(e: anyhow::Error) -> Self {
        ExtraPasswordError::Other(e)
    }
}

async fn perform_extra_password_auth(
    client: &Session<PassSessionKeyType>,
    password: String,
) -> Result<(), ExtraPasswordError> {
    let srp_info = get_srp_info(client).await?;

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
    send_srp_proofs(client, proofs).await?;

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
) -> Result<(), ExtraPasswordError> {
    let req = POST!("/pass/v1/user/srp/auth")
        .body_json(proofs)
        .context("Error creating SRP request")?;
    let res = session
        .send(req)
        .await
        .context("Error sending SRP proofs")?;
    match res.status() {
        Status::OK => Ok(()),
        Status::BAD_REQUEST => Err(ExtraPasswordError::BadPassword),
        _ => Err(ExtraPasswordError::Other(anyhow!(
            "Invalid status code received: {:?}",
            res.status()
        ))),
    }
}

pub async fn handle_extra_password(
    session: &Session<PassSessionKeyType>,
) -> Result<(), anyhow::Error> {
    let mut attempts = 3;
    loop {
        if attempts == 0 {
            println!("Too many incorrect extra password attempts, logging out");
            session.logout().await;
            return Err(anyhow!("Error in extra password flow"));
        }

        let extra_password = get_extra_password()?;
        match perform_extra_password_auth(session, extra_password).await {
            Ok(()) => {
                init_session(session)
                    .await
                    .context("Error initializing session")?;
                return Ok(());
            }
            Err(e) => match e {
                ExtraPasswordError::Other(e) => {
                    return Err(anyhow!("Error in extra password flow: {e:#}"));
                }
                ExtraPasswordError::BadPassword => {
                    println!("Incorrect extra password");
                    attempts -= 1;
                }
            },
        }
    }
}
