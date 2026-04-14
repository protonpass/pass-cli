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

use anyhow::{Context, Result, anyhow};
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::DecodePrivateKey;
use ssh_key::private::PrivateKey as SshPrivateKey;
use ssh_key::private::RsaKeypair as SshRsaKeypair;

pub fn parse_private_key_with_rsa_pem_fallback(private_key_content: &str) -> Result<SshPrivateKey> {
    match SshPrivateKey::from_openssh(private_key_content) {
        Ok(private_key) => Ok(private_key),
        Err(openssh_error) => {
            // Accept RSA keys serialized as PEM PKCS#8 / PKCS#1.
            if let Ok(rsa_key) = rsa::RsaPrivateKey::from_pkcs8_pem(private_key_content)
                .or_else(|_| rsa::RsaPrivateKey::from_pkcs1_pem(private_key_content))
            {
                let rsa_keypair = SshRsaKeypair::try_from(&rsa_key)
                    .context("Failed to convert RSA PEM key to SSH key format")?;
                return Ok(SshPrivateKey::from(rsa_keypair));
            }

            Err(anyhow!(openssh_error)).context("Failed to parse SSH private key")
        }
    }
}
