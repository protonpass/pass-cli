use anyhow::{Context, Result};
use serde::Serialize;
use crate::commands::OutputFormat;

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
            let score_out = PasswordScoreOutput{
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
