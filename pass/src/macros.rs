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

#[macro_export]
macro_rules! map (
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     };
);

#[macro_export]
macro_rules! display_for_basic {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}
#[macro_export]
macro_rules! display_for_enum {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{self:?}")
            }
        }
    };
}

#[macro_export]
macro_rules! assert_response {
    ($res:expr) => {{
        if !$res.status().is_success() {
            $crate::utils::debug_response(&$res);
            return if let Some(c) = $crate::utils::extract_proton_code(&$res) {
                Err(anyhow::anyhow!(
                    "Could not perform operation. Reason: {}",
                    c.name()
                ))
            } else {
                Err(anyhow::anyhow!("Invalid status code: {}", $res.status()))
            };
        }

        match $res.body_json() {
            Ok(v) => v,
            Err(e) => {
                $crate::utils::debug_response(&$res);
                return Err(anyhow::anyhow!("Error parsing response body: {}", e));
            }
        }
    }};
}
