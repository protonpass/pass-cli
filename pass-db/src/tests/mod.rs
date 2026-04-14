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

use crate::DatabaseManager;
use anyhow::Result;
use pass_domain::LocalKey;

#[macro_export]
macro_rules! test_db {
    () => {{ create_test_db().await.expect("failed to create test db") }};
}

pub async fn create_test_db() -> Result<DatabaseManager> {
    create_test_db_with_key(LocalKey::new(vec![0u8; 32])).await
}

pub async fn create_test_db_with_key(encryption_key: LocalKey) -> Result<DatabaseManager> {
    DatabaseManager::new_test_db(encryption_key).await
}
