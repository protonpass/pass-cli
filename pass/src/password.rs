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

use crate::organization::OrganizationPasswordPolicy;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};

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

fn validate_random_password_config(
    config: &RandomPasswordConfig,
    policy: &OrganizationPasswordPolicy,
) -> Result<()> {
    if !policy.random_password_allowed {
        return Err(anyhow!(
            "Your organization does not allow generating random passwords"
        ));
    }
    if let Some(min) = policy.random_password_min_length
        && config.length < min
    {
        return Err(anyhow!(
            "Your organization requires random passwords to be at least {min} characters long"
        ));
    }
    if let Some(max) = policy.random_password_max_length
        && config.length > max
    {
        return Err(anyhow!(
            "Your organization requires random passwords to be at most {max} characters long"
        ));
    }
    if let Some(true) = policy.random_password_must_include_numbers
        && !config.numbers
    {
        return Err(anyhow!(
            "Your organization requires random passwords to include numbers"
        ));
    }
    if let Some(true) = policy.random_password_must_include_symbols
        && !config.symbols
    {
        return Err(anyhow!(
            "Your organization requires random passwords to include symbols"
        ));
    }
    if let Some(true) = policy.random_password_must_include_uppercase
        && !config.uppercase_letters
    {
        return Err(anyhow!(
            "Your organization requires random passwords to include uppercase characters"
        ));
    }
    Ok(())
}

fn validate_passphrase_config(
    config: &PassphraseConfig,
    policy: &OrganizationPasswordPolicy,
) -> Result<()> {
    if !policy.memorable_password_allowed {
        return Err(anyhow!(
            "Your organization does not allow generating memorable passwords"
        ));
    }
    if let Some(min) = policy.memorable_password_min_words
        && config.count < min
    {
        return Err(anyhow!(
            "Your organization requires memorable passwords to have at least {min} words"
        ));
    }
    if let Some(max) = policy.memorable_password_max_words
        && config.count > max
    {
        return Err(anyhow!(
            "Your organization requires memorable passwords to have at most {max} words"
        ));
    }
    if let Some(true) = policy.memorable_password_must_capitalize
        && !config.capitalise
    {
        return Err(anyhow!(
            "Your organization requires memorable passwords to capitalise words"
        ));
    }
    if let Some(true) = policy.memorable_password_must_include_numbers
        && !config.include_numbers
    {
        return Err(anyhow!(
            "Your organization requires memorable passwords to include numbers"
        ));
    }
    Ok(())
}

fn validate_against_policy(
    args: &PasswordGenerationArgs,
    policy: &OrganizationPasswordPolicy,
) -> Result<()> {
    match args {
        PasswordGenerationArgs::Random(config) => validate_random_password_config(config, policy),
        PasswordGenerationArgs::Passphrase(config) => validate_passphrase_config(config, policy),
    }
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn generate_password(&self, args: PasswordGenerationArgs) -> Result<String> {
        if let Some(org_policy) = self
            .get_organization_policy()
            .await
            .context("Error getting organization policy")?
        {
            validate_against_policy(&args, &org_policy.settings.password_policy)?;
        }
        generate(args)
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
