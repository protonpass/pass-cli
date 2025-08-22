#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GroupId(pub(crate) String);
display_for_basic!(GroupId);

impl GroupId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
