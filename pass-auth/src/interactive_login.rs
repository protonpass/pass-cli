/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use crate::callbacks::{AuthEventHandler, CredentialProvider};
use crate::extra_password;
use crate::os::{ProdClient, ProdContext};
use crate::store::PassSessionStore;
use anyhow::{Context, bail};
use muon::auth::LoginFlow;
use muon::{GET, Session};
use std::sync::{Arc, RwLock};

pub struct AuthenticationResult {
    pub client: ProdClient,
    pub password: String,
}

pub async fn perform_interactive_login(
    client: ProdClient,
    username: &str,
    store: Arc<RwLock<PassSessionStore>>,
    credential_provider: Arc<dyn CredentialProvider>,
    event_handler: Arc<dyn AuthEventHandler>,
) -> anyhow::Result<AuthenticationResult> {
    let session = client
        .new_session_without_credentials(())
        .await
        .context("Error creating session")?;
    let auth = session.auth();
    let password = credential_provider.get_password().await?;
    let session = match auth.login(username, &password).await {
        LoginFlow::Ok(session, _) => session,

        LoginFlow::TwoFactor(session, _) => {
            let has_totp = session.has_totp();
            let has_fido = session.fido_details().is_some();

            match (has_totp, has_fido) {
                (true, _) => {
                    if has_fido {
                        event_handler
                            .on_warning(
                                "Your account has many 2FA methods available. Using TOTP. If you want to use others, use web login"
                            )
                            .await?;
                    }
                    let totp = credential_provider.get_totp().await?;
                    session.totp(&totp).await?
                }
                (false, true) => {
                    event_handler
                        .on_error("Your account cannot login interactively. Use web login instead")
                        .await?;
                    bail!("FIDO-only accounts must use web login");
                }
                (false, false) => bail!("no 2FA available"),
            }
        }

        LoginFlow::Failed { reason, .. } => {
            let msg = format!("Authentication failed: {reason}");
            event_handler.on_error(&msg).await?;
            bail!("Authentication failed");
        }
    };

    // Check if it needs extra password
    let needs_extra_password = {
        let store_guard = store.read().expect("store rwlock poisoned");
        store_guard.needs_extra_password()
    };

    if needs_extra_password {
        info!("Account needs Pass extra password");
        extra_password::handle_extra_password(&session, credential_provider, event_handler).await?;
    }

    // Initialize session to verify it works
    init_session(&session)
        .await
        .context("Error initializing session")?;

    Ok(AuthenticationResult { client, password })
}

async fn init_session(session: &Session<ProdContext>) -> anyhow::Result<()> {
    session
        .send(GET!("/tests/ping"))
        .await
        .context("Error initializing session")?;
    Ok(())
}
