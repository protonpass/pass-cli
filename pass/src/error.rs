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

#[derive(Debug)]
pub struct SessionInvalidatedError;

impl std::fmt::Display for SessionInvalidatedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "session has been invalidated")
    }
}

impl std::error::Error for SessionInvalidatedError {}

pub trait AnyhowErrorExt {
    fn is_session_invalidated(&self) -> bool;
}

impl AnyhowErrorExt for anyhow::Error {
    fn is_session_invalidated(&self) -> bool {
        self.chain()
            .any(|c| c.downcast_ref::<SessionInvalidatedError>().is_some())
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ProtonApiError {
    #[serde(rename = "Code")]
    pub code: ProtonApiErrorCode,
}

#[allow(dead_code)]
#[derive(Clone, Debug, serde_repr::Deserialize_repr)]
#[repr(i32)]
pub enum ProtonApiErrorCode {
    // Pass-specific
    NotLatestKey = 300001,
    NotLatestRevision = 300002,
    InvalidSignature = 300003,
    DeletedShare = 300004,
    RotationPayloadIncomplete = 300005,
    MissingKeys = 300006,
    ResourceLimitExceeded = 300007,
    SessionLocked = 300008,
    NoQuotaLeft = 300009,

    // Generic
    InvalidRequirements = 2000,
    InvalidValue = 2001,
    InvalidType = 2002,
    ValueOutOfBounds = 2003,
    NotNull = 2004,
    NotEmpty = 2005,
    NotTrue = 2006,
    NotFalse = 2007,
    NotEqual = 2008,
    Equal = 2009,
    NotSameAsField = 2010,
    NotAllowed = 2011,
    RegexError = 2120,
    InvalidNumber = 2021,
    InvalidLength = 2022,
    TooShort = 2023,
    TooLong = 2024,
    HashNotEqual = 2025,
    PermissionDenied = 2026,
    InsufficientScope = 2027,
    Banned = 2028,
    NoResetMethods = 2029,
    UploadFailure = 2030,
    PayloadTooLarge = 2031,
    FeatureDisabled = 2032,
    AdminPermissionMissing = 2033,
    EmailFormat = 2050,
    IpFormat = 2051,
    UrlFormat = 2052,
    CurrencyFormat = 2053,
    LocaleFormat = 2054,
    DateFormat = 2055,
    JsonFormat = 2056,
    MimeFormat = 2057,
    PhoneFormat = 2058,
    DomainFormat = 2059,
    PgpFormat = 2060,
    IdFormat = 2061,
    HexFormat = 2062,
    Base64Format = 2063,
    VersionFormat = 2064,
    ImageFormat = 2065,
    AlreadyExists = 2500,
    NotExists = 2501,
    IsLocked = 2502,
    Timeout = 2503,
    InsertFailed = 2504,
    SelectFailed = 2505,
    IncompatibleState = 2511,
    BadLockId = 2512,
    ProviderUnavailable = 2900,
    ProviderMisconfigured = 2901,
    ProviderFailed = 2902,
    ProviderBlocked = 2903,
    ProviderAuthentication = 2904,
    ProviderResourceNotExists = 2905,
}

impl ProtonApiErrorCode {
    pub fn name(&self) -> String {
        format!("{:?}", self)
    }
}
