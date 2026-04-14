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

use pass_domain::PlainText;

#[async_trait::async_trait(?Send)]
pub trait ClientTestExt {
    async fn encrypt_for_user_key(&self, data: Vec<u8>) -> Vec<u8>;
}

#[async_trait::async_trait(?Send)]
impl ClientTestExt for super::muon_ext::TestPassClient {
    async fn encrypt_for_user_key(&self, data: Vec<u8>) -> Vec<u8> {
        let user_key = self
            .get_primary_user_key()
            .await
            .expect("Error getting user key");
        let (private, public) = user_key.into_keys();
        let crypto = self.client_features.get_pgp_crypto().await;
        crypto
            .encrypt_and_sign(PlainText::new(data), public, private, None)
            .await
            .expect("Error encrypting data")
    }
}
