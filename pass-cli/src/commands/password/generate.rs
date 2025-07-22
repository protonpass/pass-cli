use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};
use serde::Serialize;
use pass::password::{PassphraseConfig, PasswordGenerationArgs, RandomPasswordConfig};
use crate::commands::OutputFormat;

#[derive(Clone, ValueEnum)]
pub enum WordSeparator {
    Hyphens,
    Spaces,
    Periods,
    Commas,
    Underscores,
    Numbers,
    NumbersAndSymbols,
}

impl From<&WordSeparator> for pass::password::WordSeparator {
    fn from(value: &WordSeparator) -> Self {
        match value {
            WordSeparator::Hyphens => Self::Hyphens,
            WordSeparator::Spaces => Self::Spaces,
            WordSeparator::Periods => Self::Periods,
            WordSeparator::Commas => Self::Commas,
            WordSeparator::Underscores => Self::Underscores,
            WordSeparator::Numbers => Self::Numbers,
            WordSeparator::NumbersAndSymbols => Self::NumbersAndSymbols,
        }
    }
}

#[derive(Serialize)]
struct PasswordOutput<'a> {
    password: &'a str
}

#[derive(Subcommand)]
pub enum GeneratePasswordCommand {
    #[command(about = "Generate a random password")]
    Random {
        #[arg(
            long = "length",
            help = "Length of the random password",
            default_value = "16"
        )]
        length: u32,
        #[arg(
            long = "numbers",
            help = "Whether to include numbers",
            default_value = "true"
        )]
        numbers: Option<bool>,
        #[arg(
            long = "uppercase",
            help = "Whether to include uppercase letters",
            default_value = "true"
        )]
        uppercase_letters: Option<bool>,
        #[arg(
            long = "symbols",
            help = "Whether to include Symbols",
            default_value = "true"
        )]
        symbols: Option<bool>,
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
    #[command(about = "Generate a passphrase")]
    Passphrase {
        #[arg(
            long = "separator",
            help = "Which word separator to use",
            default_value = "hyphens"
        )]
        separator: WordSeparator,
        #[arg(
            long = "capitalise",
            help = "Whether to capitalise words",
            default_value = "true"
        )]
        capitalise: Option<bool>,
        #[arg(
            long = "numbers",
            help = "Whether to include numbers",
            default_value = "true"
        )]
        include_numbers: Option<bool>,
        #[arg(
            long = "count",
            help = "How many words to use in the passphrase",
            default_value = "5"
        )]
        count: u32,
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
}

pub async fn run(command: &GeneratePasswordCommand) -> Result<()> {
    let (args, output) = match command {
        GeneratePasswordCommand::Random {
            length,
            numbers,
            uppercase_letters,
            symbols,
            output,
        } => (PasswordGenerationArgs::Random(RandomPasswordConfig {
            length: *length,
            numbers: numbers.unwrap_or(true),
            uppercase_letters: uppercase_letters.unwrap_or(true),
            symbols: symbols.unwrap_or(true),
        }), output),
        GeneratePasswordCommand::Passphrase {
            separator,
            capitalise,
            include_numbers,
            count,
            output,
        } => (PasswordGenerationArgs::Passphrase(PassphraseConfig {
            separator: separator.into(),
            capitalise: capitalise.unwrap_or(true),
            include_numbers: include_numbers.unwrap_or(true),
            count: *count,
        }), output),
    };

    let password = pass::password::generate(args).context("Failed to generate password")?;

    match output {
        OutputFormat::Human => {
            println!("{password}");
        }
        OutputFormat::Json => {
            let as_json =
                serde_json::to_string_pretty(&PasswordOutput { password: &password }).context("Error serializing password")?;
            println!("{as_json}");
        }
    }
    Ok(())
}
