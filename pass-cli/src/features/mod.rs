pub(crate) mod env_key_provider;
pub(crate) mod keyring;

use crate::storage::{CliDataStorage, DatabaseFolderKeyStorage, DatabaseShareKeyStorage};
use crate::telemetry::SqliteTelemetryHandler;
use anyhow::{Context, Result};
use pass_db::DatabaseManager;
use pass_domain::{
    AccountCrypto, ClientFeatures, DataStorage, FsStorage, LocalKey, LocalKeyProvider,
    TelemetryHandler,
};
use pass_fs::RealFsStorage;
use pass_pgp::{NativePgpCrypto, ProtonAccountCrypto};
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
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
        "env" => {
            info!("Using environment variable-based local key provider");
            Ok(Arc::new(env_key_provider::EnvLocalKeyProvider::new()?))
        }
        _ => Err(anyhow::anyhow!(
            "Invalid PROTON_PASS_KEY_PROVIDER value: '{}'. Valid values are 'fs', 'keyring', or 'env'",
            provider_type
        )),
    }
}

#[derive(Clone)]
pub struct CliClientFeatures {
    pub storage: Arc<RealFsStorage>,
    pub key_provider: Arc<dyn LocalKeyProvider>,
    pub telemetry_handler: Arc<SqliteTelemetryHandler>,
    pub data_storage: Arc<dyn DataStorage>,
    pub share_key_storage: Arc<DatabaseShareKeyStorage>,
    pub folder_key_storage: Arc<DatabaseFolderKeyStorage>,
    pub user_id: Arc<RwLock<Option<String>>>,

    #[allow(dead_code)]
    pub database_manager: DatabaseManager,
}

impl CliClientFeatures {
    pub async fn new(base_dir: PathBuf) -> Result<Self> {
        let key_provider = get_key_provider(base_dir.clone())?;
        Self::new_with_key_provider(base_dir, key_provider).await
    }

    pub async fn new_with_key_provider(
        base_dir: PathBuf,
        key_provider: Arc<dyn LocalKeyProvider>,
    ) -> Result<Self> {
        // Get the encryption key for the database
        let encryption_key = key_provider
            .get_key()
            .await
            .context("Failed to get encryption key for database")?;

        // Initialize encrypted database with the key
        let db = DatabaseManager::new(base_dir.clone(), encryption_key)
            .await
            .context("Failed to initialize database")?;

        // Initialize storage
        let share_key_storage = Arc::new(DatabaseShareKeyStorage::new(db.clone()));
        let folder_key_storage = Arc::new(DatabaseFolderKeyStorage::new(db.clone()));
        let data_storage: Arc<dyn DataStorage> = Arc::new(CliDataStorage::new(
            share_key_storage.clone(),
            folder_key_storage.clone(),
        ));

        Ok(Self {
            storage: Arc::new(RealFsStorage::new(base_dir.clone())),
            telemetry_handler: Arc::new(SqliteTelemetryHandler::new(db.clone())),
            database_manager: db,
            user_id: Arc::new(RwLock::new(None)),
            data_storage,
            share_key_storage,
            folder_key_storage,
            key_provider,
        })
    }

    pub async fn set_user_id(&self, user_id: Option<String>) {
        {
            let mut user_id_guard = self.user_id.write().await;
            *user_id_guard = user_id.clone();
        }
        self.telemetry_handler.set_user_id(user_id.clone()).await;
        self.share_key_storage.set_user_id(user_id.clone()).await;
        self.folder_key_storage.set_user_id(user_id).await;
    }

    pub async fn get_user_id(&self) -> Option<String> {
        self.user_id.read().await.clone()
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

    async fn get_data_storage(&self) -> Result<Arc<dyn DataStorage>> {
        Ok(self.data_storage.clone())
    }

    async fn on_session_invalidated(&self) -> Result<()> {
        crate::commands::logout::cleanup().await
    }
}
