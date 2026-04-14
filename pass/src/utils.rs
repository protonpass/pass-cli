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

use crate::error::{ProtonApiError, ProtonApiErrorCode};
use anyhow::Context;
use base64::Engine;

pub fn b64_encode<T: AsRef<[u8]>>(data: T) -> String {
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
            error!("Cannot get HttpRes body_str: {e:#}");
        }
    }
}

pub fn is_id(value: &str) -> bool {
    value.len() == 88 && value.ends_with("==")
}

pub(crate) fn extract_proton_code(res: &muon::http::HttpRes) -> Option<ProtonApiErrorCode> {
    res.body_json::<ProtonApiError>().ok().map(|c| c.code)
}
