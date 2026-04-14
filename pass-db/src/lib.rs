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

#[macro_use]
extern crate tracing;

#[cfg(test)]
#[macro_use]
pub mod tests;

mod db;
mod db_manager;
mod migration;
mod models;

pub use db::{DATABASE_NAME, DatabaseManager};
pub use db_manager::{DbConnection, EncryptedSqliteManager, format_key_for_sqlcipher};
pub use models::*;

pub use deadpool;
pub use rusqlite;
