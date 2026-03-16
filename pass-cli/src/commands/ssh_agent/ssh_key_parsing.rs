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
