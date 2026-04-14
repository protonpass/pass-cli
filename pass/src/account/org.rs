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
use anyhow::{Context, Result, anyhow};
use muon::GET;
use pass_domain::{
    AddressKey, DataToDecrypt, Passphrase, PrivateKey, PublicKey, Signature, UnlockedAddressKeys,
    UserKeyExt,
};

struct OrgKeyCacheType;

const ORG_KEY_TOKEN_SIGNATURE_CONTEXT: &str = "account.key-token.organization";

#[derive(Clone, serde::Deserialize)]
pub(crate) struct OrganizationKey {
    #[serde(rename = "PrivateKey")]
    pub private_key: Option<String>,
    #[serde(rename = "Token")]
    pub token: Option<String>,
    #[serde(rename = "Signature")]
    pub signature: Option<String>,
    #[serde(rename = "Passwordless")]
    pub passwordless: bool,
}

impl OrganizationKey {
    pub fn is_passwordless(&self) -> bool {
        self.passwordless
            || (self.private_key.is_some() && self.token.is_some() && self.signature.is_some())
    }
}

impl<C: PassClientContext> PassClient<C> {
    pub(crate) async fn get_organization_key(&self) -> Result<OrganizationKey> {
        {
            let cached = self.cache.get(OrgKeyCacheType).await;
            if let Some(cache) = cached {
                return Ok(cache);
            }
        }
        let res = self
            .send(GET!("/core/v4/organizations/keys"))
            .await
            .context("Error fetching organization key")?;
        let response: OrganizationKey = assert_response!(res);

        self.cache.store(OrgKeyCacheType, response.clone()).await;

        Ok(response)
    }

    pub(crate) async fn open_org_key(
        &self,
        org_key: &OrganizationKey,
    ) -> Result<(PrivateKey, PublicKey)> {
        if org_key.is_passwordless() {
            let crypto = self.client_features.get_pgp_crypto().await;
            let private_key = match org_key.private_key {
                Some(ref private_key) => crypto
                    .unarmor(private_key.to_string())
                    .await
                    .context("Error unarmoring org key")?,
                None => return Err(anyhow!("Could not access private key for org key")),
            };
            let decrypted_token = self
                .decrypt_address_key_token(
                    &org_key.token,
                    &org_key.signature,
                    ORG_KEY_TOKEN_SIGNATURE_CONTEXT,
                )
                .await
                .context("Error decrypting address_key_token for organization key")?;

            let opened_key = crypto
                .open_private_key(
                    PrivateKey::new(private_key),
                    Passphrase::new(decrypted_token),
                )
                .await
                .context("Error opening org private key")?;

            let public_key = crypto
                .get_public_key(opened_key.clone())
                .await
                .context("Error getting public org key")?;

            Ok((opened_key, public_key))
        } else {
            Err(anyhow!("Cannot open OrganizationKey"))
        }
    }

    pub(crate) async fn open_group_keys(
        &self,
        group_keys: Vec<AddressKey>,
    ) -> Result<UnlockedAddressKeys> {
        let org_key = self
            .get_organization_key()
            .await
            .context("Error getting organization key")?;
        let (private_org_key, public_org_key) = self
            .open_org_key(&org_key)
            .await
            .context("Error opening org key")?;

        let account_crypto = self.client_features.get_account_crypto().await;
        account_crypto
            .open_address_keys_with_keys(vec![private_org_key], vec![public_org_key], group_keys)
            .await
            .context("Error opening address keys")
    }

    async fn decrypt_address_key_token(
        &self,
        token: &Option<String>,
        signature: &Option<String>,
        context: &str,
    ) -> Result<Vec<u8>> {
        let (token_data, signature_data) = match (token, signature) {
            (Some(token), Some(signature)) => (token, signature),
            _ => return Err(anyhow!("Missing token or signature for Organization Key")),
        };

        let user_keys = self
            .get_user_keys()
            .await
            .context("Error getting user keys")?;
        let (private, public) = user_keys.split_keys();

        let crypto = self.client_features.get_pgp_crypto().await;
        let decrypted = crypto
            .decrypt_and_verify_data(
                DataToDecrypt::DataWithSignature {
                    data: token_data.as_bytes().to_vec(),
                    signature: Signature::Armored(signature_data.to_string()),
                },
                private,
                public,
                Some(context.to_string()),
            )
            .await
            .context("Error decrypting token")?;

        Ok(decrypted)
    }
}
