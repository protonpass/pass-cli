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
use pass::domain::LocalKey;
use pass::domain::utils::xor_key;
use std::ops::Deref;

pub fn format_key_for_sqlcipher(key: &[u8]) -> String {
    let key_hex: String = key.iter().map(|b| format!("{:02x}", b)).collect();

    // Format for SQLCipher to use literal bytes: x'BYTES_IN_HEX_HERE'
    format!("x'{}'", key_hex)
}

pub(crate) fn key_pragma_sql(key: &str) -> String {
    format!("PRAGMA key = \"{}\";", key.replace('"', "\"\""))
}

pub struct EncryptedSqliteManager {
    path: String,
    encryption_key: Vec<u8>,
    xor_key: u8,
}

impl EncryptedSqliteManager {
    pub fn new(path: String, encryption_key: LocalKey) -> Self {
        let xor_key_byte = pass::domain::crypto::generate_random_byte();
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
            // Check if file does not exist and create it with right permissions
            #[cfg(unix)]
            {
                use std::fs::{OpenOptions, Permissions};
                use std::os::unix::fs::PermissionsExt;

                if !std::path::Path::new(&path).exists() {
                    let mut options = OpenOptions::new();
                    options.write(true).create(true).truncate(false);
                    let _ = options.open(&path);
                    let _ = std::fs::set_permissions(&path, Permissions::from_mode(0o600));
                }
            }

            let conn = rusqlite::Connection::open(&path)?;
            // Set SQLCipher encryption key immediately after opening. This must happen
            // before any other statement touches the database (see `key_pragma_sql`).
            conn.execute_batch(&key_pragma_sql(&key))?;
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
        // This executes synchronously and returns a ready future
        // Wrap the result in Ok to create the outer Result layer
        std::future::ready(Ok(self.interact_blocking(func)))
    }

    /// Synchronous version of [`Self::interact`]. Futures that must be `Send`
    /// can't hold a `&DbConnection` across an `.await` (the connection isn't
    /// `Sync`), so they can use this instead once they're done awaiting.
    pub fn interact_blocking<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&rusqlite::Connection) -> R + Send,
        R: Send + 'static,
    {
        let conn: &rusqlite::Connection = self.obj.deref();

        // Use block_in_place to run blocking code without moving to a separate thread
        tokio::task::block_in_place(|| func(conn))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DatabaseManager;

    fn temp_db_path(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().join(name);
        (dir, path)
    }

    /// Applies `PRAGMA key` the way pass-db did while it depended on rusqlite <= 0.40,
    /// i.e. through `pragma_update` rendering the key as a quoted string. Any database
    /// written by a released version of pass-cli has to stay readable through this.
    fn apply_legacy_key(conn: &rusqlite::Connection, key: &str) -> rusqlite::Result<()> {
        conn.execute_batch(&format!("PRAGMA key='{}';", key.replace('\'', "''")))
    }

    #[test]
    fn key_pragma_uses_a_quoted_string() {
        assert_eq!(
            key_pragma_sql("x'0011'"),
            "PRAGMA key = \"x'0011'\";",
            "the key must be passed as a quoted string, a bare x'..' literal is a syntax error"
        );
    }

    #[test]
    fn database_created_with_current_key_is_reopenable() {
        let (_dir, path) = temp_db_path("roundtrip.db");
        let key = format_key_for_sqlcipher(&[7u8; 32]);

        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(&key_pragma_sql(&key)).unwrap();
            conn.execute_batch("CREATE TABLE t(a INTEGER); INSERT INTO t VALUES (42);")
                .unwrap();
        }

        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(&key_pragma_sql(&key)).unwrap();
        let value: i64 = conn
            .query_row("SELECT a FROM t", [], |row| row.get(0))
            .expect("data written with our own key must remain readable");
        assert_eq!(value, 42);
    }

    #[test]
    fn database_created_with_legacy_key_is_reopenable() {
        let (_dir, path) = temp_db_path("legacy.db");
        let key = format_key_for_sqlcipher(&[9u8; 32]);

        // Write with the pre-0.41 code path, which is what existing users have on disk.
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            apply_legacy_key(&conn, &key).unwrap();
            conn.execute_batch("CREATE TABLE t(a INTEGER); INSERT INTO t VALUES (1337);")
                .unwrap();
        }

        // Read back with the current code path.
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(&key_pragma_sql(&key)).unwrap();
        let value: i64 = conn
            .query_row("SELECT a FROM t", [], |row| row.get(0))
            .expect("databases created by older versions must stay readable");
        assert_eq!(value, 1337);
    }

    #[test]
    fn existing_database_can_be_opened_without_reading_it_first() {
        // Regression test: rusqlite >= 0.41 makes `pragma_update` validate the pragma
        // name by preparing a statement on the connection before emitting the PRAGMA
        // itself. That prepare hits the still-encrypted database and fails with
        // SQLITE_NOTADB ("file is not a database"). Opening an already-initialised
        // database must therefore not execute anything before the key is applied.
        let (_dir, path) = temp_db_path("validation.db");
        let key = format_key_for_sqlcipher(&[3u8; 32]);

        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(&key_pragma_sql(&key)).unwrap();
            conn.execute_batch("CREATE TABLE t(a INTEGER);").unwrap();
        }

        for _ in 0..3 {
            let conn = rusqlite::Connection::open(&path)
                .expect("opening an existing database must succeed without touching it");
            conn.execute_batch(&key_pragma_sql(&key))
                .expect("applying the key to an existing database must succeed");
            conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
                .expect("decrypted database must be readable");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn database_manager_can_reopen_a_database_it_created() {
        // End-to-end regression test: the whole pool machinery (multiple connections,
        // migrations on an already-initialised database) has to work on reopen too.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pass-cli.db").display().to_string();
        let key = || LocalKey::new(vec![11u8; 32]);

        let db = DatabaseManager::new_with_path(path.clone(), key())
            .await
            .expect("initialising a fresh database must succeed");
        db.get_connection()
            .await
            .expect("a fresh database must hand out connections");
        drop(db);

        let db = DatabaseManager::new_with_path(path, key())
            .await
            .expect("reopening an existing database must succeed");
        let count: i64 = db
            .get_connection()
            .await
            .expect("an existing database must hand out connections")
            .interact(|conn| -> anyhow::Result<i64> {
                Ok(conn.query_row("SELECT count(*) FROM migrations", [], |row| row.get(0))?)
            })
            .await
            .unwrap()
            .unwrap();
        assert!(count > 0, "migrations should have been recorded");
    }
}
