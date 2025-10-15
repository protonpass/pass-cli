#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::Display, strum::EnumString)]
pub enum FeatureFlag {
    PassCanUseCli,
}

impl FeatureFlag {
    pub fn name(&self) -> String {
        format!("{self}")
    }
}
