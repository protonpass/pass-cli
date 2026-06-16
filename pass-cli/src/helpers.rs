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

use anyhow::{Result, anyhow};
use parking_lot::RwLock;
use pass::{PassClient, PassClientContext};
use pass_auth::os::ProdContext;
use std::sync::Arc;

use crate::features::CliClientFeatures;
use pass_auth::PassSessionStore;

#[async_trait::async_trait]
pub trait SessionExt {
    async fn get_user_id(&self) -> Result<String>;
}

#[async_trait::async_trait]
impl SessionExt for Arc<RwLock<PassSessionStore>> {
    async fn get_user_id(&self) -> Result<String> {
        let store_guard = self.read();
        let auth = store_guard.auth.lock();
        let user_id = auth
            .as_ref()
            .and_then(|a| a.user_id().map(|u| u.to_string()));
        match user_id {
            Some(user_id) => Ok(user_id),
            None => Err(anyhow!("Invalid current session: Does not have a UserID")),
        }
    }
}

pub trait PassClientExt {
    fn get_cli_client_features(&self) -> Result<CliClientFeatures>;
}

impl<C: PassClientContext> PassClientExt for PassClient<C> {
    fn get_cli_client_features(&self) -> Result<CliClientFeatures> {
        let features = self.get_client_features();

        // HACK: Convert to &dyn Any so we can downcast it
        let any_ref = features.as_ref() as &dyn std::any::Any;

        any_ref
            .downcast_ref::<CliClientFeatures>()
            .cloned()
            .ok_or_else(|| anyhow!("Failed to downcast ClientFeatures to CliClientFeatures"))
    }
}

/// Type alias for the concrete PassClient used in the CLI
pub type CliPassClient = PassClient<ProdContext>;
