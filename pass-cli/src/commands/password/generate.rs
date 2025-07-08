use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};
use pass::password::{PassphraseConfig, PasswordGenerationArgs, RandomPasswordConfig};

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
        numbers: bool,
        #[arg(
            long = "uppercase",
            help = "Whether to include uppercase letters",
            default_value = "true"
        )]
        uppercase_letters: bool,
        #[arg(
            long = "symbols",
            help = "Whether to include Symbols",
            default_value = "true"
        )]
        symbols: bool,
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
        capitalise: bool,
        #[arg(
            long = "numbers",
            help = "Whether to include numbers",
            default_value = "true"
        )]
        include_numbers: bool,
        #[arg(
            long = "count",
            help = "How many words to use in the passphrase",
            default_value = "5"
        )]
        count: u32,
    },
}

pub async fn run(command: &GeneratePasswordCommand) -> Result<()> {
    let args = match command {
        GeneratePasswordCommand::Random {
            length,
            numbers,
            uppercase_letters,
            symbols,
        } => PasswordGenerationArgs::Random(RandomPasswordConfig {
            length: *length,
            numbers: *numbers,
            uppercase_letters: *uppercase_letters,
            symbols: *symbols,
        }),
        GeneratePasswordCommand::Passphrase {
            separator,
            capitalise,
            include_numbers,
            count,
        } => PasswordGenerationArgs::Passphrase(PassphraseConfig {
            separator: separator.into(),
            capitalise: *capitalise,
            include_numbers: *include_numbers,
            count: *count,
        }),
    };

    let password = pass::password::generate(args).context("Failed to generate password")?;
    println!("{password}");
    Ok(())
}
