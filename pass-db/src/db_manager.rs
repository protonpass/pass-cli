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

use deadpool::managed::{Manager, Metrics, Object, RecycleResult};
use pass_domain::LocalKey;
use pass_domain::utils::xor_key;
use std::ops::Deref;

pub fn format_key_for_sqlcipher(key: &[u8]) -> String {
    let key_hex: String = key.iter().map(|b| format!("{:02x}", b)).collect();

    // Format for SQLCipher to use literal bytes: x'BYTES_IN_HEX_HERE'
    format!("x'{}'", key_hex)
}

pub struct EncryptedSqliteManager {
    path: String,
    encryption_key: Vec<u8>,
    xor_key: u8,
}

impl EncryptedSqliteManager {
    pub fn new(path: String, encryption_key: LocalKey) -> Self {
        let xor_key_byte = pass_domain::crypto::generate_random_byte();
        let xored_key = xor_key(encryption_key.as_ref(), xor_key_byte);
        Self {
            path,
            encryption_key: xored_key,
            xor_key: xor_key_byte,
        }
    }

    fn get_sqlcipher_key(&self) -> String {
        let raw_value = xor_key(&self.encryption_key, self.xor_key);
        format_key_for_sqlcipher(&raw_value)
    }
}

impl Manager for EncryptedSqliteManager {
    type Type = rusqlite::Connection;
    type Error = rusqlite::Error;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let path = self.path.clone();
        let key = self.get_sqlcipher_key();

        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(&path)?;
            // Set SQLCipher encryption key immediately after opening
            // Use pragma_update instead of execute because PRAGMA key returns results
            conn.pragma_update(None, "key", &key)?;
            // Verify the key is correct by querying the database.
            // Errors here (e.g. SQLCipher HMAC failure) should be handled, as a wrong key
            // or corrupted database would otherwise surface as a misleading "out of memory"
            // error later when migrations try to write to the database.
            conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
                .map_err(|e| {
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error {
                            code: rusqlite::ffi::ErrorCode::DatabaseCorrupt,
                            extended_code: 0,
                        },
                        Some(format!(
                            "Failed to open encrypted database: {e}. \
                             The encryption key may not match or the database may be corrupted. \
                             Try running 'pass-cli logout --force' to reset local state."
                        )),
                    )
                })?;
            Ok(conn)
        })
        .await
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
    }

    async fn recycle(
        &self,
        conn: &mut Self::Type,
        _metrics: &Metrics,
    ) -> RecycleResult<Self::Error> {
        conn.execute("SELECT 1", [])
            .map(|_| ())
            .map_err(deadpool::managed::RecycleError::Backend)
    }
}

pub struct DbConnection {
    pub obj: Object<EncryptedSqliteManager>,
}

impl DbConnection {
    /// Execute a closure with access to the database connection in a blocking context
    pub fn interact<F, R>(&self, func: F) -> impl Future<Output = anyhow::Result<R>> + '_
    where
        F: FnOnce(&rusqlite::Connection) -> R + Send,
        R: Send + 'static,
    {
        let conn: &rusqlite::Connection = self.obj.deref();

        // Use block_in_place to run blocking code without moving to a separate thread
        // This executes synchronously and returns a ready future
        // Wrap the result in Ok to create the outer Result layer
        std::future::ready(Ok(tokio::task::block_in_place(|| func(conn))))
    }
}
