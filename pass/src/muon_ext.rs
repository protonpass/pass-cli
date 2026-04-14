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

use std::error::Error;

pub trait MuonErrorExt {
    fn is_logged_out_error(&self) -> bool;
}

impl MuonErrorExt for muon::Error {
    fn is_logged_out_error(&self) -> bool {
        if self.kind() == muon::ErrorKind::Send
            && let Some(source) = self.source()
        {
            return source.to_string() == "non-existent session";
        }

        false
    }
}
