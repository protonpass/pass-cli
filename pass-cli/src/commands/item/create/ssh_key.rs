use anyhow::{Context, Result, anyhow};
use clap::{Args, Subcommand};
use pass::PassClient;
use pass::ssh_key::SshKeyItemCreatePayload;
use std::io::Read;
use std::path::PathBuf;

use crate::commands::item::common::ShareQuery;

const SSH_KEY_PASSWORD_ENV_VAR: &str = "PROTON_PASS_SSH_KEY_PASSWORD";
const SSH_KEY_PASSWORD_FILE_ENV_VAR: &str = "PROTON_PASS_SSH_KEY_PASSWORD_FILE";

#[derive(Args)]
pub struct SshKeyArgs {
    #[command(subcommand)]
    command: SshKeyCommand,
}

#[derive(Subcommand)]
enum SshKeyCommand {
    /// Import an SSH key from a private key file
    Import {
        /// Path to the private key file
        #[arg(long = "from-private-key")]
        private_key_file: PathBuf,

        /// Enable passphrase for the SSH key
        #[arg(long)]
        password: bool,

        /// Share ID of the vault to create the SSH key item in
        #[arg(long)]
        share_id: Option<String>,

        /// Name of the vault to create the SSH key item in
        #[arg(long, help = "Name of the vault to create the SSH key item in")]
        vault_name: Option<String>,

        /// Title of the SSH key item
        #[arg(long)]
        title: String,

        /// Folder ID to create the item in
        #[cfg(feature = "internal")]
        #[arg(long, help = "Folder ID to create the item in")]
        folder_id: Option<String>,
    },
    /// Generate a new SSH key
    Generate {
        /// Comment for the SSH key
        #[arg(long)]
        comment: Option<String>,

        /// Type of SSH key to generate
        #[arg(long, default_value = "ed25519")]
        key_type: SshKeyTypeArg,

        /// Enable passphrase for the SSH key
        #[arg(long)]
        password: bool,

        /// Share ID of the vault to create the SSH key item in
        #[arg(long)]
        share_id: Option<String>,

        /// Name of the vault to create the SSH key item in
        #[arg(long, help = "Name of the vault to create the SSH key item in")]
        vault_name: Option<String>,

        /// Title of the SSH key item
        #[arg(long)]
        title: String,

        /// Folder ID to create the item in
        #[cfg(feature = "internal")]
        #[arg(long, help = "Folder ID to create the item in")]
        folder_id: Option<String>,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum SshKeyTypeArg {
    Ed25519,
    Rsa2048,
    Rsa4096,
}

impl From<SshKeyTypeArg> for proton_pass_common::sshkey::SshKeyType {
    fn from(arg: SshKeyTypeArg) -> Self {
        match arg {
            SshKeyTypeArg::Ed25519 => proton_pass_common::sshkey::SshKeyType::Ed25519,
            SshKeyTypeArg::Rsa2048 => proton_pass_common::sshkey::SshKeyType::RSA2048,
            SshKeyTypeArg::Rsa4096 => proton_pass_common::sshkey::SshKeyType::RSA4096,
        }
    }
}

pub async fn run(args: SshKeyArgs, client: PassClient) -> Result<()> {
    match args.command {
        SshKeyCommand::Import {
            private_key_file,
            password,
            share_id,
            vault_name,
            title,
            #[cfg(feature = "internal")]
            folder_id,
        } => {
            #[cfg(feature = "internal")]
            let folder_id = folder_id
                .as_ref()
                .map(|id| pass_domain::FolderId::new(id.clone()));
            #[cfg(not(feature = "internal"))]
            let folder_id = None;

            run_import(
                private_key_file,
                password,
                share_id,
                vault_name,
                title,
                folder_id,
                client,
            )
            .await
        }
        SshKeyCommand::Generate {
            comment,
            key_type,
            password,
            share_id,
            vault_name,
            title,
            #[cfg(feature = "internal")]
            folder_id,
        } => {
            #[cfg(feature = "internal")]
            let folder_id = folder_id
                .as_ref()
                .map(|id| pass_domain::FolderId::new(id.clone()));
            #[cfg(not(feature = "internal"))]
            let folder_id = None;

            run_generate(
                comment, key_type, password, share_id, vault_name, title, folder_id, client,
            )
            .await
        }
    }
}

async fn run_import(
    private_key_file: PathBuf,
    password_flag: bool,
    share_id: Option<String>,
    vault_name: Option<String>,
    title: String,
    folder_id: Option<pass_domain::FolderId>,
    client: PassClient,
) -> Result<()> {
    let share_query = ShareQuery::new(share_id, vault_name)?;

    let private_key_content = std::fs::read_to_string(&private_key_file)
        .with_context(|| format!("Error reading private key file: {:?}", private_key_file))?;

    let private_key = ssh_key::private::PrivateKey::from_openssh(&private_key_content)
        .context("Failed to parse SSH private key")?;

    let passphrase = get_ssh_key_password(password_flag, true)?;

    if private_key.is_encrypted() && passphrase.is_none() {
        eprintln!(
            "Warning: Private key is encrypted but password was not provided. The key will be stored encrypted."
        );
    }

    let public_key = private_key.public_key();
    let public_key_str = public_key
        .to_openssh()
        .context("Failed to convert public key to OpenSSH format")?;

    let share_id = share_query.share_id(&client).await?;
    let payload = SshKeyItemCreatePayload {
        title,
        private_key: private_key_content,
        public_key: public_key_str,
        passphrase,
    };

    let item_id = client
        .create_ssh_key(&share_id, payload, folder_id.as_ref())
        .await
        .context("Error creating SSH key item")?;

    println!("{item_id}");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_generate(
    comment: Option<String>,
    key_type: SshKeyTypeArg,
    password_flag: bool,
    share_id: Option<String>,
    vault_name: Option<String>,
    title: String,
    folder_id: Option<pass_domain::FolderId>,
    client: PassClient,
) -> Result<()> {
    let share_query = ShareQuery::new(share_id, vault_name)?;

    let passphrase = get_ssh_key_password(password_flag, true)?;
    let comment = comment.unwrap_or_default();

    // Generate SSH key using proton_pass_common
    let key_type_common: proton_pass_common::sshkey::SshKeyType = key_type.into();
    let key_pair =
        proton_pass_common::sshkey::generate_ssh_key(comment, key_type_common, passphrase.clone())
            .map_err(|e| anyhow!("Failed to generate SSH key: {:?}", e))?;

    let share_id = share_query.share_id(&client).await?;
    let payload = SshKeyItemCreatePayload {
        title,
        private_key: key_pair.private_key,
        public_key: key_pair.public_key,
        passphrase,
    };

    let item_id = client
        .create_ssh_key(&share_id, payload, folder_id.as_ref())
        .await
        .context("Error creating SSH key item")?;

    println!("{item_id}");
    Ok(())
}

fn get_ssh_key_password(password_flag: bool, is_generate: bool) -> Result<Option<String>> {
    // Check if password is provided via env var
    if let Ok(password) = std::env::var(SSH_KEY_PASSWORD_ENV_VAR) {
        eprintln!("Reading password from environment variable {SSH_KEY_PASSWORD_ENV_VAR}");
        return Ok(Some(password));
    }

    // Check if password is provided via file
    if let Ok(file_path) = std::env::var(SSH_KEY_PASSWORD_FILE_ENV_VAR) {
        eprintln!("Reading password from file {file_path}");
        let mut f = std::fs::File::open(file_path).context("Error opening password file")?;
        let mut buff = String::new();
        f.read_to_string(&mut buff)
            .context("Error reading password file")?;
        return Ok(Some(buff.trim().to_string()));
    }

    // Password not provided neither via env var or via env var pointing to file
    // Check if they want to add a password
    if !password_flag {
        return Ok(None);
    }

    // They want to enter a password interactively.
    // Depending on whether is a generation or not, ask for it once or twice
    if is_generate {
        loop {
            let password = crate::utils::ask_for_input("Enter SSH key passphrase: ", true)?;
            let confirmation = crate::utils::ask_for_input("Confirm SSH key passphrase: ", true)?;

            if password == confirmation {
                return Ok(Some(password));
            }

            eprintln!("Passphrases do not match. Please try again.");
        }
    } else {
        let password = crate::utils::ask_for_input("Enter SSH key passphrase: ", true)?;
        Ok(Some(password))
    }
}
