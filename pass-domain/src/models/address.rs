#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AddressId(pub(crate) String);
display_for_basic!(AddressId);

impl AddressId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct Address {
    pub id: AddressId,
    pub email: String,
    pub keys: Vec<AddressKey>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddressKeyId(pub(crate) String);
display_for_basic!(AddressKeyId);

impl AddressKeyId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct AddressKey {
    pub id: AddressKeyId,
    pub primary: bool,
    pub active: bool,
    pub private_key: String,
    pub token: Option<String>,
    pub signature: Option<String>,
}
