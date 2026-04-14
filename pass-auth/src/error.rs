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
pub enum AuthError {
    CannotDecrypt(anyhow::Error),
    BadExtraPassword,
    Other(anyhow::Error),
}

impl From<anyhow::Error> for AuthError {
    fn from(e: anyhow::Error) -> Self {
        AuthError::Other(e)
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::CannotDecrypt(e) => write!(f, "Cannot decrypt session: {e:#}"),
            AuthError::BadExtraPassword => write!(f, "Incorrect extra password"),
            AuthError::Other(e) => write!(f, "{e:#}"),
        }
    }
}

impl std::error::Error for AuthError {}
