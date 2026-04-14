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

use crate::{DbConnection, EncryptedSqliteManager};
use anyhow::{Context, Result, anyhow};
use deadpool::managed::Pool;
use pass_domain::LocalKey;
use std::path::PathBuf;
use tokio::fs;

pub const DATABASE_NAME: &str = "pass-cli.db";

#[derive(Clone)]
pub struct DatabaseManager {
    pool: Pool<EncryptedSqliteManager>,
}

impl DatabaseManager {
    pub async fn new(base_dir: PathBuf, encryption_key: LocalKey) -> Result<Self> {
        fs::create_dir_all(&base_dir)
            .await
            .context("Failed to create base directory")?;

        let db_path = base_dir.join(DATABASE_NAME);

        if db_path.exists() {
            debug!("Connecting to encrypted database at: {}", db_path.display());
        } else {
            debug!("Initializing encrypted database at: {}", db_path.display());
        }

        let db_path_str = db_path
            .to_str()
            .context("Invalid database path")?
            .to_string();

        Self::new_with_path(db_path_str, encryption_key).await
    }

    pub async fn new_with_path(db_path: String, encryption_key: LocalKey) -> Result<Self> {
        // Create custom manager with encryption
        let manager = EncryptedSqliteManager::new(db_path, encryption_key);

        // Create pool with custom manager
        let pool = Pool::builder(manager)
            .build()
            .context("Failed to create database pool")?;

        let db_manager = Self { pool };

        db_manager.init_migrations().await?;
        db_manager.run_migrations().await?;

        Ok(db_manager)
    }

    #[cfg(test)]
    pub async fn new_test_db(encryption_key: LocalKey) -> Result<Self> {
        // Use a temporary file for SQLCipher since :memory: doesn't work well with connection pools
        // Use keep so it's not cleaned up, as we'll need it to exist to run the tess
        let dir = tempfile::tempdir()
            .context("Failed to create temporary directory")?
            .keep();

        let db_path = dir.join(DATABASE_NAME);

        // Make extra-sure that the DB does not exist before we initialize it
        let _ = std::fs::remove_file(&db_path);
        Self::new_with_path(db_path.display().to_string(), encryption_key).await
    }

    pub async fn get_connection(&self) -> Result<DbConnection> {
        let obj = self
            .pool
            .get()
            .await
            .context("Failed to get database connection")?;
        Ok(DbConnection { obj })
    }

    async fn init_migrations(&self) -> Result<()> {
        let conn = self.get_connection().await?;

        conn.interact(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS migrations (
                    id INTEGER PRIMARY KEY,
                    description TEXT NOT NULL,
                    applied_at INTEGER NOT NULL
                )",
                [],
            )
            .map_err(|e| anyhow!("Failed to create migrations table: {}", e))
        })
        .await
        .map_err(|e| anyhow!("Failed to interact with database: {}", e))??;

        Ok(())
    }

    async fn run_migrations(&self) -> Result<()> {
        let migrations = crate::migration::get_migrations();
        let conn = self.get_connection().await?;

        for migration in migrations {
            let migration_id = migration.id;
            let description = migration.description;
            let sql = migration.sql;

            let applied = conn
                .interact(move |conn| {
                    let mut stmt = conn
                        .prepare("SELECT id FROM migrations WHERE id = ?1")
                        .map_err(|e| anyhow!("Failed to prepare statement: {}", e))?;
                    let exists = stmt.exists([migration_id])?;
                    Ok::<bool, anyhow::Error>(exists)
                })
                .await
                .map_err(|e| anyhow!("Failed to check migration status: {}", e))??;

            if !applied {
                info!("Running migration {}: {}", migration_id, description);

                conn.interact(move |conn| {
                    conn.execute_batch(sql)
                        .map_err(|e| anyhow!("Failed to execute migration: {}", e))?;

                    conn.execute(
                        "INSERT INTO migrations (id, description, applied_at) VALUES (?1, ?2, ?3)",
                        rusqlite::params![
                            migration_id,
                            description,
                            jiff::Timestamp::now().as_second()
                        ],
                    )
                    .map_err(|e| anyhow!("Failed to record migration: {}", e))?;

                    Ok::<(), anyhow::Error>(())
                })
                .await
                .map_err(|e| anyhow!("Failed to run migration: {}", e))??;

                info!("Migration {} completed", migration_id);
            }
        }

        Ok(())
    }
}
