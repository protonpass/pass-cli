use clap::ValueEnum;
use pass_domain::ShareRole;

pub mod info;
pub mod inject;
#[cfg(feature = "internal")]
pub mod internal;
pub mod invite;
pub mod item;
pub mod login;
pub mod logout;
pub mod password;
pub mod run;
pub mod secret_resolver;
pub mod share;
pub mod test;
pub mod user;
pub mod vault;

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Role {
    Viewer,
    Editor,
    Manager,
}

impl From<Role> for ShareRole {
    fn from(role: Role) -> Self {
        match role {
            Role::Viewer => ShareRole::Viewer,
            Role::Editor => ShareRole::Editor,
            Role::Manager => ShareRole::Manager,
        }
    }
}
