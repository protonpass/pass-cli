use super::{PatExpiration, expiration_to_timestamp};
use crate::commands::OutputFormat;
use crate::commands::settings_helper::get_format;
use crate::helpers::CliPassClient as PassClient;
use anyhow::Result;
use pass::CreatePersonalAccessTokenArgs;

#[derive(Clone, Debug, serde::Serialize)]
struct CreatePersonalAccessTokenResult {
    env_var: String,
    pat_id: String,
}

pub async fn run(
    client: PassClient,
    name: String,
    expiration: PatExpiration,
    format: Option<OutputFormat>,
) -> Result<()> {
    let format = get_format(format, &client).await?;
    let expiration_timestamp = expiration_to_timestamp(&expiration)?;

    let args = CreatePersonalAccessTokenArgs::new(name, expiration_timestamp)?;
    let response = client.create_personal_access_token(args).await?;

    match format {
        OutputFormat::Json => {
            let res = CreatePersonalAccessTokenResult {
                env_var: response.env_var,
                pat_id: response.personal_access_token_id.value().to_string(),
            };
            let serialized = serde_json::to_string_pretty(&res)?;
            println!("{}", serialized);
        }
        OutputFormat::Human => {
            println!("{}", response.env_var);
        }
    }

    Ok(())
}
