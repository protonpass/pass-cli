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

mod access;
mod create;
mod delete;
mod list;
mod renew;

use crate::commands::OutputFormat;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow};
use clap::Subcommand;
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};
use pass_domain::PersonalAccessTokenId;

pub enum PersonalAccessTokenQuery {
    PersonalAccessTokenId(String),
    PersonalAccessTokenName(String),
}

impl PersonalAccessTokenQuery {
    pub fn new(
        personal_access_token_id: Option<String>,
        personal_access_token_name: Option<String>,
    ) -> Result<Self> {
        match (personal_access_token_id, personal_access_token_name) {
            (Some(id), None) => Ok(Self::PersonalAccessTokenId(id)),
            (None, Some(name)) => {
                if pass::is_id(&name) {
                    Ok(Self::PersonalAccessTokenId(name))
                } else {
                    Ok(Self::PersonalAccessTokenName(name))
                }
            }
            _ => Err(anyhow!("Please provide either --pat-id or --pat-name")),
        }
    }

    pub async fn resolve(&self, client: &PassClient) -> Result<PersonalAccessTokenId> {
        match self {
            PersonalAccessTokenQuery::PersonalAccessTokenId(id) => {
                Ok(PersonalAccessTokenId::new(id.clone()))
            }
            PersonalAccessTokenQuery::PersonalAccessTokenName(name) => {
                let pats = client
                    .list_personal_access_tokens()
                    .await
                    .context("Failed to list personal access tokens")?;

                let pat = pats
                    .iter()
                    .find(|pat| pat.name == *name)
                    .ok_or_else(|| anyhow!("Personal access token not found: {}", name))?;

                Ok(pat.pat_id.clone())
            }
        }
    }
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum PatExpiration {
    #[value(name = "1d")]
    OneDay,
    #[value(name = "1w")]
    OneWeek,
    #[value(name = "1m")]
    OneMonth,
    #[value(name = "3m")]
    ThreeMonths,
    #[value(name = "6m")]
    SixMonths,
    #[value(name = "1y")]
    OneYear,
}

#[derive(Subcommand)]
pub enum PersonalAccessTokenCommands {
    #[command(about = "Create a new personal access token")]
    Create {
        #[arg(long, help = "Name of the personal access token")]
        name: String,
        #[arg(
            long,
            help = "Expiration for the personal access token (1d, 1w, 1m, 3m, 6m, 1y)"
        )]
        expiration: PatExpiration,
        #[arg(long)]
        output: Option<OutputFormat>,
    },
    #[command(about = "List all personal access tokens")]
    List {
        #[arg(long)]
        output: Option<OutputFormat>,
    },
    #[command(about = "Delete a personal access token")]
    Delete {
        #[arg(long, help = "Personal access token ID to delete", alias = "pat-id")]
        personal_access_token_id: String,
    },
    #[command(about = "Renew a personal access token")]
    Renew {
        #[arg(long, help = "Personal access token ID", alias = "pat-id")]
        personal_access_token_id: Option<String>,
        #[arg(long, help = "Personal access token name", alias = "pat-name")]
        personal_access_token_name: Option<String>,
        #[arg(
            long,
            help = "New expiration for the personal access token (1d, 1w, 1m, 3m, 6m, 1y)"
        )]
        expiration: PatExpiration,
        #[arg(long)]
        output: Option<OutputFormat>,
    },
    #[command(about = "Manage personal access token access")]
    Access {
        #[command(subcommand)]
        command: access::AccessCommands,
    },
}

pub fn expiration_to_timestamp(expiration: &PatExpiration) -> Result<i64> {
    let span = match expiration {
        PatExpiration::OneDay => Span::new().days(1),
        PatExpiration::OneWeek => Span::new().weeks(1),
        PatExpiration::OneMonth => Span::new().months(1),
        PatExpiration::ThreeMonths => Span::new().months(3),
        PatExpiration::SixMonths => Span::new().months(6),
        PatExpiration::OneYear => Span::new().years(1),
    };
    let now = Timestamp::now().to_zoned(TimeZone::UTC);
    let future = now
        .checked_add(span)
        .context("Failed to compute expiration timestamp")?;
    Ok(future.timestamp().as_second())
}

pub async fn run(command: PersonalAccessTokenCommands, client: PassClient) -> Result<()> {
    match command {
        PersonalAccessTokenCommands::Create {
            name,
            expiration,
            output,
        } => create::run(client, name, expiration, output).await,
        PersonalAccessTokenCommands::List { output } => list::run(client, output).await,
        PersonalAccessTokenCommands::Delete {
            personal_access_token_id,
        } => delete::run(client, personal_access_token_id).await,
        PersonalAccessTokenCommands::Renew {
            personal_access_token_id,
            personal_access_token_name,
            expiration,
            output,
        } => {
            let query = PersonalAccessTokenQuery::new(
                personal_access_token_id,
                personal_access_token_name,
            )?;
            renew::run(client, query, expiration, output).await
        }
        PersonalAccessTokenCommands::Access { command } => access::run(command, client).await,
    }
}
