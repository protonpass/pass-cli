use anyhow::Context;
use base64::Engine;

pub fn b64_encode(data: Vec<u8>) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

pub fn b64_decode(data: &str) -> anyhow::Result<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .context("Error decoding base64 data")
}

pub(crate) fn debug_response(res: &muon::http::HttpRes) {
    match res.body_str() {
        Ok(body) => {
            debug!("{body}");
        }
        Err(e) => {
            error!("Cannot get HttpRes body_str: {e}");
        }
    }
}
