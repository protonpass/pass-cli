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
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::{DELETE, POST};

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

impl<C: PassClientContext> PassClient<C> {
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

    pub async fn unlock_session(&self, lock_code: &str) -> Result<()> {
        let request = UnlockSessionRequest {
            lock_code: lock_code.to_string(),
        };

        let req = POST!("/pass/v1/user/session/unlock")
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
