use anyhow::{Context, Result, bail};
use clap::Args;
use pass::PassClient;
use pass::wifi::WifiItemCreatePayload;
use pass_domain::WifiSecurity;
use std::io::{self, Read};

use crate::commands::item::common::ShareQuery;

#[derive(serde::Deserialize, serde::Serialize)]
struct WifiTemplate {
    title: String,
    ssid: String,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    security: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

impl Default for WifiTemplate {
    fn default() -> Self {
        Self {
            title: "".to_string(),
            ssid: "".to_string(),
            password: Some("".to_string()),
            security: Some("".to_string()),
            note: Some("".to_string()),
        }
    }
}

impl WifiTemplate {
    fn into_payload(self) -> Result<WifiItemCreatePayload> {
        let security = if let Some(security_str) = self.security {
            Some(parse_security_type(&security_str)?)
        } else {
            None
        };

        Ok(WifiItemCreatePayload {
            title: self.title,
            ssid: Some(self.ssid),
            password: self.password,
            security,
            note: self.note,
        })
    }
}

fn parse_security_type(type_str: &str) -> Result<WifiSecurity> {
    match type_str.to_lowercase().as_str() {
        "wpa" => Ok(WifiSecurity::WPA),
        "wpa2" => Ok(WifiSecurity::WPA2),
        "wpa3" => Ok(WifiSecurity::WPA3),
        "wep" => Ok(WifiSecurity::WEP),
        "open" | "none" | "" => Ok(WifiSecurity::UnspecifiedWifiSecurity),
        "unspecified" => Ok(WifiSecurity::UnspecifiedWifiSecurity),
        _ => bail!(
            "Invalid security type '{}'. Valid values: wpa, wpa2, wpa3, wep, open, none, unspecified",
            type_str
        ),
    }
}

#[derive(Args)]
pub struct WifiArgs {
    /// Display a template
    #[arg(long, conflicts_with_all = ["from_template", "share_id", "title", "ssid", "password", "security", "note"])]
    get_template: bool,

    /// Create from template file (use '-' for stdin)
    #[arg(long)]
    from_template: Option<String>,

    /// Share ID
    #[arg(long, required_unless_present_any = ["get_template", "from_template"])]
    share_id: Option<String>,

    /// Vault name
    #[arg(long, help = "Name of the vault to create the WiFi item in")]
    vault_name: Option<String>,

    /// Item title
    #[arg(long)]
    title: Option<String>,

    /// Network SSID (name)
    #[arg(long, help = "Network SSID (name)")]
    ssid: Option<String>,

    /// Network password
    #[arg(long, help = "Network password (leave empty for open networks)")]
    password: Option<String>,

    /// Security type (wpa, wpa2, wpa3, wep, open, none)
    #[arg(long, help = "Security type (wpa, wpa2, wpa3, wep, open, none)")]
    security: Option<String>,

    /// Note
    #[arg(long)]
    note: Option<String>,

    /// Folder ID to create the item in
    #[cfg(feature = "internal")]
    #[arg(long, help = "Folder ID to create the item in")]
    folder_id: Option<String>,
}

pub async fn run(args: WifiArgs, client: PassClient) -> Result<()> {
    if args.get_template {
        let template = WifiTemplate::default();
        let json = serde_json::to_string_pretty(&template)
            .context("Error serializing template to JSON")?;
        println!("{}", json);
        return Ok(());
    }

    if let Some(template_path) = args.from_template {
        let share_query = ShareQuery::new(args.share_id, args.vault_name)?;
        #[cfg(feature = "internal")]
        let folder_id = args
            .folder_id
            .as_ref()
            .map(|id| pass_domain::FolderId::new(id.clone()));
        #[cfg(not(feature = "internal"))]
        let folder_id = None;

        return create_wifi_from_template(&template_path, share_query, folder_id, client).await;
    }

    let share_query = ShareQuery::new(args.share_id, args.vault_name)?;

    let title = args
        .title
        .ok_or_else(|| anyhow::anyhow!("--title is required when not using --from-template"))?;

    let ssid = args
        .ssid
        .ok_or_else(|| anyhow::anyhow!("--ssid is required when not using --from-template"))?;

    let security = if let Some(security_str) = args.security {
        Some(parse_security_type(&security_str)?)
    } else {
        None
    };

    let payload = WifiItemCreatePayload {
        title,
        ssid: Some(ssid),
        password: args.password,
        security,
        note: args.note,
    };

    #[cfg(feature = "internal")]
    let folder_id = args
        .folder_id
        .as_ref()
        .map(|id| pass_domain::FolderId::new(id.clone()));
    #[cfg(not(feature = "internal"))]
    let folder_id = None;

    create_wifi_from_payload(payload, share_query, folder_id, client).await
}

async fn create_wifi_from_template(
    template_path: &str,
    share_query: ShareQuery,
    folder_id: Option<pass_domain::FolderId>,
    client: PassClient,
) -> Result<()> {
    let template_json = if template_path == "-" {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Error reading template from stdin")?;
        buffer
    } else {
        std::fs::read_to_string(template_path)
            .with_context(|| format!("Error reading template file: {}", template_path))?
    };

    let template: WifiTemplate = serde_json::from_str(&template_json)
        .context("Error parsing template JSON. Use --get-template to see the expected format")?;

    let payload = template
        .into_payload()
        .context("Error converting template to payload")?;

    create_wifi_from_payload(payload, share_query, folder_id, client).await
}

async fn create_wifi_from_payload(
    payload: WifiItemCreatePayload,
    share_query: ShareQuery,
    folder_id: Option<pass_domain::FolderId>,
    client: PassClient,
) -> Result<()> {
    let share_id = share_query.share_id(&client).await?;
    let item_id = client
        .create_wifi(&share_id, payload, folder_id.as_ref())
        .await
        .context("Error creating WiFi item")?;

    println!("{}", item_id.value());
    Ok(())
}
