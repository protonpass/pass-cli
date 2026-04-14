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

use crate::account::key_salts::{GetKeySaltsResponse, KeySaltResponse};
use crate::test_tools::{Empty, MuonServerExt, success};
use crate::user::access::{GetUserInfoResponse, MonitorStatus};
use crate::{PassPlan, PlanType, UserDataSettings, UserInfo};
use muon::common::Context;
use muon::rest::core::v4;
use muon::{GET, Session};
use muon_test::server::ProtonAPI;

pub const TEST_ADDRESS_EMAIL: &str = "passclitestuser@proton.black";
pub const TEST_ADDRESS_ID: &str =
    "XasbMWxj7eq6mZR1u0nH4sNqiE0SZtqCBvWquypSO-BwhkjVlqJ3ttcbiR55BM0fkDk9CkO-9lcUhDgg5cZqAg==";
pub const TEST_ADDRESS_KEY_ID: &str =
    "_h-tHP7dtSPhKkj81wPXudfIolbC6kVNDu5mM7vrp5-_pdIxCiZ2IgBf3AQIh7KGGSQWgnyHaHGwGxauxX0SQw==";

pub const TEST_USER_ID: &str =
    "sMXH3WRflhDfwvU0GsWvctl0wR3NJWEqtiRs8cf2NeMdBAk8e_MTJkQtS704RfhgVRuxJ7xVV49ta-pMHXbNDg==";
pub const TEST_USER_PRIVATE_KEY_ID: &str =
    "sMXH3WRflhDfwvU0GsWvctl0wR3NJWEqtiRs8cf2NeMdBAk8e_MTJkQtS704RfhgVRuxJ7xVV49ta-pMHXbNDg==";

pub const TEST_USER_PRIVATE_KEY: &str = r#"-----BEGIN PGP PRIVATE KEY BLOCK-----
Version: ProtonMail

xYYEaKctzhYJKwYBBAHaRw8BAQdAdrk+vfzbYPOas799KMPEej5qCY+cW8lP
rQ5MlWgToRj+CQMIbAvPVjxLketgAAAAAAAAAAAAAAAAAAAAAF4ECGsKje4D
ByzsJc7LesJ9z2fl7HGXRLixmE2R9WO1VOFH5fe3uZghhriMvUm7IeJcKcDG
bM07bm90X2Zvcl9lbWFpbF91c2VAZG9tYWluLnRsZCA8bm90X2Zvcl9lbWFp
bF91c2VAZG9tYWluLnRsZD7CwBEEExYKAIMFgminLc4DCwkHCZDNbMLYFQWa
+UUUAAAAAAAcACBzYWx0QG5vdGF0aW9ucy5vcGVucGdwanMub3JnguoKKSCv
r401eqLSB27J3CcPj4BupjfQInQdhANYfsUDFQoIBBYAAgECGQECmwMCHgEW
IQTzQgwE47WWctwq8pzNbMLYFQWa+QAAdJAA/1Q2nwQTPA5YpeCHvb7e8td/
OVU5MAZT8jA84f2JoG+jAQDhPEzR9pUDQ54AN3t8+j1/HQb/brHcCs4+kQ/Q
0cMGC8eLBGinLc4SCisGAQQBl1UBBQEBB0CKkVYUBcMFDKS34MGCstfm9wYI
6HI7HrmLm99L7rTPIgMBCAf+CQMIUYTGfZflpMFgAAAAAAAAAAAAAAAAAAAA
AI+voubzvqEV0TqLAOzCAfGX6onsFbfW9Km+zKv4+xPXL1iY+4/ZYMGr0Jq0
kHinER6pFKQZAMK+BBgWCgBwBYJopy3OCZDNbMLYFQWa+UUUAAAAAAAcACBz
YWx0QG5vdGF0aW9ucy5vcGVucGdwanMub3JnSBm1qOAMF+wI0oRQDpfgMhT4
PtjNT5d/3SzaH0NRPKECmwwWIQTzQgwE47WWctwq8pzNbMLYFQWa+QAA8UUA
/0BNvFOZ1OV+1v4j2QgXaZ3x2RnVxfybP+qlcx/zObmpAQCqSrCQjR1ygH0/
EuDfh9ybHGfj0377XIrW0uanwcfDDQ==
=2S1m
-----END PGP PRIVATE KEY BLOCK-----
"#;

pub const TEST_ADDRESS_KEY_PRIVATE_KEY: &str = r#"-----BEGIN PGP PRIVATE KEY BLOCK-----
Version: ProtonMail

xYYEaKctzhYJKwYBBAHaRw8BAQdAJc+pG6m3YFxhEuLAoDxSXBUy0SbsbE5e
x8eZVBpnXk/+CQMICq3bE1tkVy9gAAAAAAAAAAAAAAAAAAAAAJfFonHPifHc
lA04wRTBhALFYjf1zkFfKhWOwthf2Y5xv/j1JDFmOtv4dyCC5+dMBM/UIdsl
HM07cGFzc2NsaXRlc3R1c2VyQHByb3Rvbi5ibGFjayA8cGFzc2NsaXRlc3R1
c2VyQHByb3Rvbi5ibGFjaz7CwBEEExYKAIMFgminLc4DCwkHCZAy5zi+IWdF
dEUUAAAAAAAcACBzYWx0QG5vdGF0aW9ucy5vcGVucGdwanMub3JnouQAv/Xd
Fw9RlJfWGA5nxgeB9qJfWyUtZl57whx2yEYDFQoIBBYAAgECGQECmwMCHgEW
IQQ+LrBo8m3AqWHzPAgy5zi+IWdFdAAAqFcBALi8NV2F/vjKB9NOdS4Lb7T6
whIBHUW2pw1MeMvk3w2KAQDb9aSPRC2aW70ZcmkW+9lh8yUDvGwj61ry7bEr
GFFCDceLBGinLc4SCisGAQQBl1UBBQEBB0BVtVkqodOsZLHqEl0NIB8uYOBR
MBzu39ncic0BxTTeCQMBCAf+CQMI7C6bI2uAI+ZgAAAAAAAAAAAAAAAAAAAA
AMepexTBlnUSHeQpRfuocurKOLxK6bz3w+yU5Qamy2YRailubbdN9WJhHG+B
nat8rppHKn+9A8K+BBgWCgBwBYJopy3OCZAy5zi+IWdFdEUUAAAAAAAcACBz
YWx0QG5vdGF0aW9ucy5vcGVucGdwanMub3JngB3hRLgfDFRGL8KyiX9W3PI8
kdoE/jty8ktd1eku3Z8CmwwWIQQ+LrBo8m3AqWHzPAgy5zi+IWdFdAAAKgAB
AKsLoQTLDzbmKDbV396inJ1PB05l2YtPeAbHhp7dZ673AQCKM7R8/GVSVtjY
NJZIw3A4KuoCpttkwXh2d0Q3JmmvAg==
=wzSx
-----END PGP PRIVATE KEY BLOCK-----
"#;

#[allow(dead_code)]
pub const TEST_ADDRESS_KEY_PUBLIC_KEY: &str = r#"-----BEGIN PGP PUBLIC KEY BLOCK-----
Version: ProtonMail

xjMEaKctzhYJKwYBBAHaRw8BAQdAJc+pG6m3YFxhEuLAoDxSXBUy0SbsbE5e
x8eZVBpnXk/NO3Bhc3NjbGl0ZXN0dXNlckBwcm90b24uYmxhY2sgPHBhc3Nj
bGl0ZXN0dXNlckBwcm90b24uYmxhY2s+wsARBBMWCgCDBYJopy3OAwsJBwmQ
Muc4viFnRXRFFAAAAAAAHAAgc2FsdEBub3RhdGlvbnMub3BlbnBncGpzLm9y
Z6LkAL/13RcPUZSX1hgOZ8YHgfaiX1slLWZee8IcdshGAxUKCAQWAAIBAhkB
ApsDAh4BFiEEPi6waPJtwKlh8zwIMuc4viFnRXQAAKhXAQC4vDVdhf74ygfT
TnUuC2+0+sISAR1FtqcNTHjL5N8NigEA2/Wkj0Qtmlu9GXJpFvvZYfMlA7xs
I+ta8u2xKxhRQg3CwB4EEBYIAJAFgminLgoFgwDtTgAJENTSGgVRp7LsNRQA
AAAAABwAEHNhbHRAbm90YXRpb25zLm9wZW5wZ3Bqcy5vcmfG+p/lypX41Gd+
lyYrii+2LBxUZXN0IE9wZW5QR1AgQ0EgPHRlc3Qtb3BlbnBncC1jYUBwcm90
b24ubWU+FiEENhVDvw2lbaYMs9qU1NIaBVGnsuwAAKG7AQCVLpEk1DmJwTXm
wU5qeNrtXavYag6AEbjr+qchGoWFIAEAzwASA0nrtCYBA2dPa6jwU2NHYkzi
lpQAkz/s6WT85gfOOARopy3OEgorBgEEAZdVAQUBAQdAVbVZKqHTrGSx6hJd
DSAfLmDgUTAc7t/Z3InNAcU03gkDAQgHwr4EGBYKAHAFgminLc4JkDLnOL4h
Z0V0RRQAAAAAABwAIHNhbHRAbm90YXRpb25zLm9wZW5wZ3Bqcy5vcmeAHeFE
uB8MVEYvwrKJf1bc8jyR2gT+O3LyS13V6S7dnwKbDBYhBD4usGjybcCpYfM8
CDLnOL4hZ0V0AAAqAAEAqwuhBMsPNuYoNtXf3qKcnU8HTmXZi094BseGnt1n
rvcBAIoztHz8ZVJW2Ng0lkjDcDgq6gKm22TBeHZ3RDcmaa8C
=iWq/
-----END PGP PUBLIC KEY BLOCK-----
"#;

pub const TEST_ADDRESS_KEY_SIGNATURE: &str = r#"-----BEGIN PGP SIGNATURE-----
Version: ProtonMail

wrsEARYKAG0FgminLgoJkM1swtgVBZr5RRQAAAAAABwAIHNhbHRAbm90YXRp
b25zLm9wZW5wZ3Bqcy5vcmcfkR0KOkwLrpaHZBCBU/lSm1U35PLA5uYZXbY8
pLpY3xYhBPNCDATjtZZy3CrynM1swtgVBZr5AABWuQD8CI4JZHwuismKMZ0Y
F1kCiGaZ/wqA+V9CGtfvu35L9fcBANhGIJ9vgZwNpJ1IYKwE+l77MS4arI65
RHNes8QzQHcI
=TrfE
-----END PGP SIGNATURE-----
"#;

pub const TEST_ADDRESS_KEY_TOKEN: &str = r#"-----BEGIN PGP MESSAGE-----
Version: ProtonMail

wV4D9Cy1x6t9568SAQdA5o3TJrMtJEKZEU58IbsqthXSvRGyKIs+WdFZyGES
kGcwaucARflIQUpcBDAw2Bae42UJXH4MqGxPwSiikNvM599kw9fHjxDSxIIn
7AekC2nU0nEBesryI2fOfK+WkQeoq27F4M0ppdLSHLWwIeLoyb8vKDSnoOCz
1po3LZq6ac9TG1fTw1Bv89ttLE/vmMpFl/sW0Wleqyzz8mFxdlubAESjNu/d
ZUhh/Lg+SH6IQi81AzKXsiwsy7esQEgZuwC/K6htiw==
=wZeW
-----END PGP MESSAGE-----
"#;

pub const TEST_PASSPHRASE: &str = "passclitestuser";

pub const TEST_SALT_ID: &str =
    "sMXH3WRflhDfwvU0GsWvctl0wR3NJWEqtiRs8cf2NeMdBAk8e_MTJkQtS704RfhgVRuxJ7xVV49ta-pMHXbNDg==";
pub const TEST_SALT_VALUE: &str = "cHQscoez6Cx3YeVBbnKcwg==";

pub const TEST_PLAN_NAME: &str = "testplan123";

pub fn setup_user_access(api: &ProtonAPI, plan_type: PlanType) {
    setup_user_access_with_limits(api, None, None, None, plan_type)
}

pub async fn init_session<C: Context>(api: &ProtonAPI, session: Session<C>) {
    api.handler("/tests/ping", move |_| success(Empty));
    session
        .send(GET!("/tests/ping"))
        .await
        .expect("Error setting up test session");
}

pub fn setup_user_access_with_limits(
    api: &ProtonAPI,
    vault_limit: Option<u16>,
    alias_limit: Option<u16>,
    totp_limit: Option<u16>,
    plan_type: PlanType,
) {
    api.handler("/pass/v1/user/access", move |_| {
        success(GetUserInfoResponse {
            access: UserInfo {
                plan: PassPlan {
                    type_: plan_type,
                    internal_name: TEST_PLAN_NAME.to_string(),
                    display_name: "Test Plan".to_string(),
                    manage_subscription: false,
                    subscription_end: None,
                    subscription_renewal: false,
                    subscription_coupon: None,
                    subscription_offer: None,
                    hide_upgrade: false,
                    trial_end: None,
                    vault_limit,
                    alias_limit,
                    totp_limit,
                    manage_alias: false,
                    storage_allowed: false,
                    storage_max_file_size: 0,
                    storage_used: 0,
                    storage_quota: 0,
                    cli_allowed: Some(true),
                },
                monitor: MonitorStatus {
                    proton_address: false,
                    aliases: false,
                },
                pending_invites: 0,
                waiting_new_user_invites: 0,
                user_data: UserDataSettings {
                    default_share_id: None,
                    alias_sync_enabled: false,
                    pending_alias_to_sync: 0,
                },
            },
        })
    });
}

pub fn setup(api: &ProtonAPI) {
    api.handler("/addresses", move |_| {
        success(v4::addresses::GetRes {
            addresses: vec![v4::addresses::Address {
                id: TEST_ADDRESS_ID.to_string(),
                email: TEST_ADDRESS_EMAIL.to_string(),
                keys: vec![v4::keys::Key {
                    id: TEST_ADDRESS_KEY_ID.to_string(),
                    private_key: TEST_ADDRESS_KEY_PRIVATE_KEY.to_string(),
                    token: Some(TEST_ADDRESS_KEY_TOKEN.to_string()),
                    signature: Some(TEST_ADDRESS_KEY_SIGNATURE.to_string()),
                    primary: true.into(),
                    active: true.into(),
                }],
            }],
        })
    });

    api.handler("/core/v4/users", move |_| {
        success(v4::users::GetRes {
            user: v4::users::User {
                id: TEST_USER_ID.to_string(),
                name: "passclitestuser".to_string(),
                email: TEST_ADDRESS_EMAIL.to_string(),
                keys: vec![v4::keys::Key {
                    id: TEST_USER_PRIVATE_KEY_ID.to_string(),
                    private_key: TEST_USER_PRIVATE_KEY.to_string(),
                    token: None,
                    signature: None,
                    primary: true.into(),
                    active: true.into(),
                }],
            },
        })
    });

    api.handler("/core/v4/keys/salts", move |_| {
        success(GetKeySaltsResponse {
            key_salts: vec![KeySaltResponse {
                id: TEST_SALT_ID.to_string(),
                key_salt: Some(TEST_SALT_VALUE.to_string()),
            }],
        })
    });
}

// Helper function to setup a paid user (Plus plan)
pub fn setup_paid_user(api: &ProtonAPI) {
    api.handler("/pass/v1/user/access", move |_| {
        success(GetUserInfoResponse {
            access: UserInfo {
                plan: PassPlan {
                    type_: PlanType::Plus,
                    internal_name: "plus2022".to_string(),
                    display_name: "Plus".to_string(),
                    manage_subscription: false,
                    subscription_end: None,
                    subscription_renewal: false,
                    subscription_coupon: None,
                    subscription_offer: None,
                    hide_upgrade: false,
                    trial_end: None,
                    vault_limit: None,
                    alias_limit: None,
                    totp_limit: None,
                    manage_alias: false,
                    storage_allowed: false,
                    storage_max_file_size: 0,
                    storage_used: 0,
                    storage_quota: 0,
                    cli_allowed: Some(true),
                },
                monitor: MonitorStatus {
                    proton_address: false,
                    aliases: false,
                },
                pending_invites: 0,
                waiting_new_user_invites: 0,
                user_data: UserDataSettings {
                    default_share_id: None,
                    alias_sync_enabled: false,
                    pending_alias_to_sync: 0,
                },
            },
        })
    });
}
