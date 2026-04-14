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

use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use pass_domain::{AddressKey, UnlockedAddressKeys};

impl<C: PassClientContext> PassClient<C> {
    pub async fn open_address_keys(
        &self,
        address_keys: Vec<AddressKey>,
    ) -> Result<UnlockedAddressKeys> {
        let user_keys = self
            .get_user_keys()
            .await
            .context("Error getting user keys")?;

        let account_crypto = self.client_features.get_account_crypto().await;
        account_crypto
            .open_address_keys(user_keys, address_keys)
            .await
            .context("Error opening address keys")
    }
}
