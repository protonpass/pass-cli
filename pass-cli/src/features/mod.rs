pub(crate) mod keyring;

use crate::telemetry::SqliteTelemetryHandler;
use anyhow::{Context, Result};
use pass_db::DatabaseManager;
use pass_domain::{
    AccountCrypto, ClientFeatures, FsStorage, LocalKey, LocalKeyProvider, TelemetryHandler,
};
use pass_fs::RealFsStorage;
use pass_pgp::{NativePgpCrypto, ProtonAccountCrypto};
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

const LOCAL_KEY_FILENAME: &str = "local.key";

fn get_key_provider(base_dir: PathBuf) -> Result<Arc<dyn LocalKeyProvider>> {
    let provider_type = env::var("PROTON_PASS_KEY_PROVIDER").unwrap_or_default();

    match provider_type.as_str() {
        "fs" => {
            info!("Using filesystem-based local key provider");
            Ok(Arc::new(FsLocalKeyProvider::new(base_dir)))
        }
        "keyring" | "" => {
            info!("Using keyring-based local key provider");
            Ok(Arc::new(keyring::KeyringKeyProvider::new(base_dir)?))
        }
        _ => Err(anyhow::anyhow!(
            "Invalid PROTON_PASS_KEY_PROVIDER value: '{}'. Valid values are 'fs' or 'keyring'",
            provider_type
        )),
    }
}

#[derive(Clone)]
pub struct CliClientFeatures {
    pub storage: Arc<RealFsStorage>,
    pub key_provider: Arc<dyn LocalKeyProvider>,
    pub database_manager: DatabaseManager,
    pub telemetry_handler: Arc<SqliteTelemetryHandler>,
}

impl CliClientFeatures {
    pub async fn new(base_dir: PathBuf) -> Result<Self> {
        let key_provider = get_key_provider(base_dir.clone())?;

        // Get the encryption key for the database
        let encryption_key = key_provider
            .get_key()
            .await
            .context("Failed to get encryption key for database")?;

        // Initialize encrypted database with the key
        let db = DatabaseManager::new(base_dir.clone(), encryption_key)
            .await
            .context("Failed to initialize database")?;

        Ok(Self {
            storage: Arc::new(RealFsStorage::new(base_dir.clone())),
            telemetry_handler: Arc::new(SqliteTelemetryHandler::new(db.clone())),
            database_manager: db,
            key_provider,
        })
    }
}

#[derive(Clone)]
pub struct FsLocalKeyProvider {
    base_dir: PathBuf,
}

impl FsLocalKeyProvider {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub async fn get_local_key(&self) -> Result<Vec<u8>> {
        let key_path = self.local_key_path()?;

        if key_path.exists() && key_path.is_file() {
            return tokio::fs::read(&key_path)
                .await
                .context("Error reading local key file");
        }

        info!("Couldn't find local key file, generating one");

        Self::create_key_file(&key_path).context("Error creating local key file")?;

        let key = pass_domain::crypto::generate_encryption_key();
        tokio::fs::write(key_path, &key)
            .await
            .context("Error writing key")?;

        Ok(key)
    }

    fn create_key_file(path: &Path) -> Result<File> {
        let f = File::create(path).context("Error creating local key file")?;

        #[cfg(not(target_os = "windows"))]
        {
            use std::fs::Permissions;
            use std::os::unix::fs::PermissionsExt;
            f.set_permissions(Permissions::from_mode(0o600))
                .context("Error setting permissions")?;
        }

        Ok(f)
    }

    fn local_key_path(&self) -> Result<PathBuf> {
        let session_path_absolute =
            std::fs::canonicalize(&self.base_dir).context("error getting absolute path")?;
        let key_path = session_path_absolute.join(LOCAL_KEY_FILENAME);

        Ok(key_path)
    }
}

#[async_trait::async_trait]
impl LocalKeyProvider for FsLocalKeyProvider {
    async fn get_key(&self) -> Result<LocalKey> {
        Ok(LocalKey::new(self.get_local_key().await?))
    }

    async fn remove_key(&self) -> Result<()> {
        let key_path = self.local_key_path()?;
        if key_path.exists() {
            tokio::fs::remove_file(&key_path)
                .await
                .context("Error removing local key file")?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl ClientFeatures for CliClientFeatures {
    async fn get_local_key_provider(&self) -> Result<Arc<dyn LocalKeyProvider>> {
        Ok(self.key_provider.clone())
    }

    async fn get_account_crypto(&self) -> Arc<dyn AccountCrypto> {
        Arc::new(ProtonAccountCrypto)
    }

    async fn get_fs(&self) -> Arc<dyn FsStorage> {
        self.storage.clone()
    }

    async fn get_pgp_crypto(&self) -> Arc<dyn pass_domain::PgpCrypto> {
        Arc::new(NativePgpCrypto)
    }

    async fn get_telemetry_handler(&self) -> Arc<dyn TelemetryHandler> {
        self.telemetry_handler.clone()
    }
}
