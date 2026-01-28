#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ServiceAccountId(pub(crate) String);
display_for_basic!(ServiceAccountId);

impl ServiceAccountId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
