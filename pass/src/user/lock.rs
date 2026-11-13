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

use crate::common::CodeResponse;
use crate::error::ProtonApiErrorCode;
use crate::utils::extract_proton_code;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::transport::http::Status;
use muon::{DELETE, GET, POST};
use pass_derive::sdk_export;

#[derive(serde::Serialize)]
struct LockSessionRequest {
    #[serde(rename = "LockCode")]
    lock_code: String,
    #[serde(rename = "UnlockedSecs")]
    unlocked_secs: u32,
}

#[derive(serde::Serialize)]
struct UnlockSessionRequest {
    #[serde(rename = "LockCode")]
    lock_code: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionLockState {
    NoLock,
    Unlocked { unlocked_secs: Option<u32> },
    Locked,
}

impl SessionLockState {
    pub fn has_lock(&self) -> bool {
        !matches!(self, SessionLockState::NoLock)
    }

    pub fn unlocked_secs(&self) -> Option<u32> {
        match self {
            SessionLockState::Unlocked { unlocked_secs } => *unlocked_secs,
            _ => None,
        }
    }
}

#[derive(serde::Deserialize)]
struct LockInfo {
    #[serde(rename = "Exists")]
    exists: bool,
    #[serde(rename = "UnlockedSecs")]
    unlocked_secs: Option<u32>,
}

#[derive(serde::Deserialize)]
struct CheckSessionLockResponse {
    #[serde(rename = "LockInfo")]
    lock_info: LockInfo,
}

impl From<LockInfo> for SessionLockState {
    fn from(info: LockInfo) -> Self {
        if info.exists {
            SessionLockState::Unlocked {
                unlocked_secs: info.unlocked_secs,
            }
        } else {
            SessionLockState::NoLock
        }
    }
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn get_session_lock_state(&self) -> Result<SessionLockState> {
        let res = self
            .send(GET!("/pass/v1/user/session/lock/check"))
            .await
            .context("Error sending check session lock request")?;

        if res.status() == Status::UNPROCESSABLE_ENTITY
            && matches!(
                extract_proton_code(&res),
                Some(ProtonApiErrorCode::SessionLocked)
            )
        {
            return Ok(SessionLockState::Locked);
        }

        let response: CheckSessionLockResponse = assert_response!(res);
        Ok(response.lock_info.into())
    }
}

#[sdk_export]
impl<C: PassClientContext> PassClient<C> {
    #[sdk_export]
    pub async fn lock_session(&self, lock_code: &str, unlocked_secs: u32) -> Result<()> {
        let request = LockSessionRequest {
            lock_code: lock_code.to_string(),
            unlocked_secs,
        };

        let req = POST!("/pass/v1/user/session/lock")
            .body_json(request)
            .context("Error creating lock session request")?;

        let res = self
            .send(req)
            .await
            .context("Error sending lock session request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }

    #[sdk_export]
    pub async fn force_lock_session(&self) -> Result<()> {
        let req = POST!("/pass/v1/user/session/lock/force_lock");

        let res = self
            .send(req)
            .await
            .context("Error sending force lock session request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }

    #[sdk_export]
    pub async fn unlock_session(&self, lock_code: &str) -> Result<()> {
        let request = UnlockSessionRequest {
            lock_code: lock_code.to_string(),
        };

        let req = POST!("/pass/v1/user/session/lock/unlock")
            .body_json(request)
            .context("Error creating unlock session request")?;

        let res = self
            .send(req)
            .await
            .context("Error sending unlock session request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }

    #[sdk_export]
    pub async fn remove_session_lock(&self, lock_code: &str) -> Result<()> {
        let request = UnlockSessionRequest {
            lock_code: lock_code.to_string(),
        };

        let req = DELETE!("/pass/v1/user/session/lock")
            .body_json(request)
            .context("Error creating remove session lock request")?;

        let res = self
            .send(req)
            .await
            .context("Error sending remove session lock request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::CodeResponse;
    use crate::test_tools::*;

    #[derive(serde::Serialize)]
    struct LockInfoPayload {
        #[serde(rename = "Exists")]
        exists: bool,
        #[serde(rename = "UnlockedSecs")]
        unlocked_secs: Option<u32>,
    }

    #[derive(serde::Serialize)]
    struct CheckPayload {
        #[serde(rename = "Code")]
        code: u32,
        #[serde(rename = "LockInfo")]
        lock_info: LockInfoPayload,
    }

    fn check_payload(exists: bool, unlocked_secs: Option<u32>) -> CheckPayload {
        CheckPayload {
            code: 1000,
            lock_info: LockInfoPayload {
                exists,
                unlocked_secs,
            },
        }
    }

    #[muon_test::test]
    async fn check_without_lock_returns_no_lock(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client(raw_client, &api).await;
        api.handler("/pass/v1/user/session/lock/check", |_| {
            response_from_status_and_payload(200, check_payload(false, None))
        });

        let state = client.get_session_lock_state().await.unwrap();

        assert_eq!(state, SessionLockState::NoLock);
    }

    #[muon_test::test]
    async fn check_with_unlocked_lock_returns_unlocked_secs(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client(raw_client, &api).await;
        api.handler("/pass/v1/user/session/lock/check", |_| {
            response_from_status_and_payload(200, check_payload(true, Some(600)))
        });

        let state = client.get_session_lock_state().await.unwrap();

        assert_eq!(
            state,
            SessionLockState::Unlocked {
                unlocked_secs: Some(600)
            }
        );
    }

    #[muon_test::test]
    async fn check_on_locked_session_returns_locked(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client(raw_client, &api).await;
        api.handler("/pass/v1/user/session/lock/check", |_| {
            response_from_status_and_payload(
                422,
                &CodeResponse {
                    code: ProtonApiErrorCode::SessionLocked as u32,
                },
            )
        });

        let state = client.get_session_lock_state().await.unwrap();

        assert_eq!(state, SessionLockState::Locked);
    }

    #[muon_test::test]
    async fn check_with_other_error_fails(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client(raw_client, &api).await;
        api.handler("/pass/v1/user/session/lock/check", |_| {
            response_from_status_and_payload(
                400,
                &CodeResponse {
                    code: ProtonApiErrorCode::SessionLocked as u32,
                },
            )
        });

        assert!(client.get_session_lock_state().await.is_err());
    }
}
