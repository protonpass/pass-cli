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

use anyhow::{Context, Result};

pub enum PasswordGenerationArgs {
    Random(RandomPasswordConfig),
    Passphrase(PassphraseConfig),
}

pub struct RandomPasswordConfig {
    pub length: u32,
    pub numbers: bool,
    pub uppercase_letters: bool,
    pub symbols: bool,
}

impl From<RandomPasswordConfig> for proton_pass_common::password::RandomPasswordConfig {
    fn from(config: RandomPasswordConfig) -> Self {
        Self {
            length: config.length,
            numbers: config.numbers,
            uppercase_letters: config.uppercase_letters,
            symbols: config.symbols,
        }
    }
}

pub struct PassphraseConfig {
    pub separator: WordSeparator,
    pub capitalise: bool,
    pub include_numbers: bool,
    pub count: u32,
}

impl From<&PassphraseConfig> for proton_pass_common::password::PassphraseConfig {
    fn from(config: &PassphraseConfig) -> Self {
        Self {
            separator: (&config.separator).into(),
            capitalise: config.capitalise,
            include_numbers: config.include_numbers,
            count: config.count,
        }
    }
}

pub enum WordSeparator {
    Hyphens,
    Spaces,
    Periods,
    Commas,
    Underscores,
    Numbers,
    NumbersAndSymbols,
}

impl From<&WordSeparator> for proton_pass_common::password::WordSeparator {
    fn from(separator: &WordSeparator) -> Self {
        match separator {
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

pub fn generate(args: PasswordGenerationArgs) -> Result<String> {
    let mut generator = proton_pass_common::password::get_generator();
    match args {
        PasswordGenerationArgs::Random(config) => {
            let mapped = config.into();
            generator
                .generate_random(&mapped)
                .context("Error generating random password")
        }
        PasswordGenerationArgs::Passphrase(config) => {
            let mapped = (&config).into();
            generator
                .generate_passphrase(&mapped)
                .context("Error generating passphrase")
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PasswordPenalty {
    NoLowercase,
    NoUppercase,
    NoNumbers,
    NoSymbols,
    Short,
    Consecutive,
    Progressive,
    ContainsCommonPassword,
}

impl std::fmt::Display for PasswordPenalty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<proton_pass_common::password::PasswordPenalty> for PasswordPenalty {
    fn from(penalty: proton_pass_common::password::PasswordPenalty) -> Self {
        match penalty {
            proton_pass_common::password::PasswordPenalty::NoLowercase => Self::NoLowercase,
            proton_pass_common::password::PasswordPenalty::NoUppercase => Self::NoUppercase,
            proton_pass_common::password::PasswordPenalty::NoNumbers => Self::NoNumbers,
            proton_pass_common::password::PasswordPenalty::NoSymbols => Self::NoSymbols,
            proton_pass_common::password::PasswordPenalty::Short => Self::Short,
            proton_pass_common::password::PasswordPenalty::Consecutive => Self::Consecutive,
            proton_pass_common::password::PasswordPenalty::Progressive => Self::Progressive,
            proton_pass_common::password::PasswordPenalty::ContainsCommonPassword => {
                Self::ContainsCommonPassword
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PasswordScore {
    Vulnerable,
    Weak,
    Strong,
}

impl From<proton_pass_common::password::PasswordScore> for PasswordScore {
    fn from(score: proton_pass_common::password::PasswordScore) -> Self {
        match score {
            proton_pass_common::password::PasswordScore::Vulnerable => Self::Vulnerable,
            proton_pass_common::password::PasswordScore::Weak => Self::Weak,
            proton_pass_common::password::PasswordScore::Strong => Self::Strong,
        }
    }
}

impl std::fmt::Display for PasswordScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PasswordScoreResult {
    pub numeric_score: f64,
    pub password_score: PasswordScore,
    pub penalties: Vec<PasswordPenalty>,
}

impl From<proton_pass_common::password::PasswordScoreResult> for PasswordScoreResult {
    fn from(result: proton_pass_common::password::PasswordScoreResult) -> Self {
        Self {
            numeric_score: result.numeric_score,
            password_score: result.password_score.into(),
            penalties: result.penalties.into_iter().map(Into::into).collect(),
        }
    }
}

pub fn score(password: &str) -> PasswordScoreResult {
    proton_pass_common::password::check_score(password).into()
}
