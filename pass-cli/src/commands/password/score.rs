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

use crate::commands::OutputFormat;
use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Serialize)]
pub struct PasswordScoreOutput {
    pub numeric_score: f64,
    pub password_score: String,
    pub penalties: Vec<String>,
}

pub fn score(password: &str, output_format: &OutputFormat) -> Result<()> {
    let score = pass::password::score(password);

    match output_format {
        OutputFormat::Human => {
            println!(
                "- Score: {} ({})",
                score.numeric_score, score.password_score
            );
            for penalty in score.penalties {
                println!("- Penalty: {penalty}");
            }
        }
        OutputFormat::Json => {
            let score_out = PasswordScoreOutput {
                numeric_score: score.numeric_score,
                password_score: score.password_score.to_string(),
                penalties: score.penalties.iter().map(|p| p.to_string()).collect(),
            };
            let as_json =
                serde_json::to_string_pretty(&score_out).context("Error serializing score")?;
            println!("{as_json}");
        }
    }

    Ok(())
}
