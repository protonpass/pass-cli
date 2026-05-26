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

mod core_event_storage;
mod data_storage;
mod folder_key_storage;
mod session_storage;
mod share_key_storage;

pub use core_event_storage::DatabaseCoreEventStorage;
pub use data_storage::CliDataStorage;
pub use folder_key_storage::DatabaseFolderKeyStorage;
pub use session_storage::FileSystemSessionStorage;
pub use share_key_storage::DatabaseShareKeyStorage;
