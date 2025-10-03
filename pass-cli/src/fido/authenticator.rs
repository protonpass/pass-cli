use super::error::AuthError;
use crate::utils::ask_for_input;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64_ENGINE;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64_URLSAFE_ENGINE;
use ctap_hid_fido2::{Cfg, FidoKeyHid, FidoKeyHidFactory, get_fidokey_devices};
use muon::rest::auth::v4::fido2;
use serde_json::json;

pub struct YubiKeyAuthenticator {
    device: FidoKeyHid,
}

impl YubiKeyAuthenticator {
    /// Create a new authenticator instance by connecting to the first available FIDO2 device
    pub fn new() -> Result<Self, AuthError> {
        info!("Searching for FIDO2 authenticators...");

        if get_fidokey_devices().is_empty() {
            return Err(AuthError::NoAuthenticator);
        }

        let mut cfg = Cfg::init();
        cfg.keep_alive_msg = "Touch the sensor on your 2FA device".to_string();

        // Connect to the first available device
        let device = FidoKeyHidFactory::create(&cfg).map_err(|e| {
            if e.to_string().contains("No device found") {
                AuthError::NoAuthenticator
            } else {
                AuthError::CtapError(e.to_string())
            }
        })?;

        info!("Connected to FIDO2 device");

        Ok(Self { device })
    }

    #[allow(dead_code)]
    pub fn get_device_info(&self) -> Result<String, AuthError> {
        let info = self.device.get_info()?;
        Ok(format!("{:?}", info))
    }

    pub fn authenticate_with_origin(
        &self,
        response: fido2::Response,
        origin: Option<&str>,
        pin: Option<&str>,
    ) -> Result<fido2::Request, AuthError> {
        let options = match response.authentication_options {
            Some(options) => options,
            None => {
                return Err(AuthError::Other(
                    "Authenticator options not found".to_string(),
                ));
            }
        };

        let pk = &options.public_key;
        let rp_id = match pk.rp_id {
            Some(ref rp_id) => rp_id,
            None => return Err(AuthError::Other("Missing rp_id".to_string())),
        };

        // Use provided origin or default to https://{rp_id}
        let origin_url = match origin {
            Some(origin) => origin.to_string(),
            None => format!("https://{}", rp_id),
        };

        info!("Starting authentication process");

        let pin_required = self.device_needs_pin()?;

        if pin_required && pin.is_none() {
            warn!("Device requires PIN but none provided");
            return Err(AuthError::PinRequired);
        }

        let credential_ids: Vec<Vec<u8>> = pk
            .allow_credentials
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|k| k.id)
            .collect();

        let client_data_json = json!({
            "type": "webauthn.get",
            "challenge": B64_URLSAFE_ENGINE.encode(&pk.challenge),
            "origin": origin_url,
            "crossOrigin": false
        });

        let client_data_bytes = serde_json::to_vec(&client_data_json)?;

        info!("Sending GetAssertion request...");
        let assertion = self
            .device
            .get_assertion(rp_id, &client_data_bytes, &credential_ids, pin)
            .map_err(|e| {
                error!("Authentication failed: {e:?}");
                if e.to_string().contains("0x31 CTAP2_ERR_PIN_INVALID") {
                    AuthError::InvalidPin
                } else {
                    AuthError::AuthenticationFailed
                }
            })?;

        info!("Authentication successful!");

        let client_data_b64 = B64_ENGINE.encode(&client_data_bytes);
        let authenticator_data_b64 = B64_ENGINE.encode(&assertion.auth_data);
        let signature_b64 = B64_ENGINE.encode(&assertion.signature);

        let final_credential_id = if !assertion.credential_id.is_empty() {
            assertion.credential_id.clone()
        } else {
            return Err(AuthError::Other(
                "Assertion credential_id not found".to_string(),
            ));
        };

        let request = fido2::Request {
            authentication_options: options,
            client_data: client_data_b64,
            authenticator_data: authenticator_data_b64,
            signature: signature_b64,
            credential_id: final_credential_id,
        };

        Ok(request)
    }

    /// Interactive authentication - prompts for PIN if needed
    pub fn authenticate_interactive(
        &self,
        response: fido2::Response,
    ) -> Result<fido2::Request, AuthError> {
        self.authenticate_interactive_with_origin(response, None)
    }

    /// Interactive authentication with custom origin - prompts for PIN if needed
    pub fn authenticate_interactive_with_origin(
        &self,
        response: fido2::Response,
        origin: Option<&str>,
    ) -> Result<fido2::Request, AuthError> {
        let pin_required = self.device_needs_pin()?;

        let pin = if pin_required {
            let pin = ask_for_input("Enter your 2FA device PIN: ", true)
                .map_err(|e| AuthError::Other(e.to_string()))?;
            Some(pin.trim().to_string())
        } else {
            None
        };

        self.authenticate_with_origin(response, origin, pin.as_deref())
    }

    fn device_needs_pin(&self) -> Result<bool, AuthError> {
        let info = self.device.get_info()?;
        let pin_required = info
            .options
            .iter()
            .find(|(k, _)| k == "clientPin")
            .map(|(_, v)| *v)
            .unwrap_or(false);
        Ok(pin_required)
    }
}
