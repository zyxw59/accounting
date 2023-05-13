use serde::{Deserialize, Serialize};

use crate::{
    backend::{id::Id, version::Versioned},
    map::Map,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub is_superuser: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Group {
    pub name: String,
    pub permissions: Permissions,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WithGroup<T> {
    #[serde(rename = "_group")]
    pub group: Id<Group>,
    #[serde(flatten)]
    pub object: T,
}

impl<T> WithGroup<Versioned<T>> {
    pub fn transpose(self) -> Versioned<WithGroup<T>> {
        Versioned {
            id: self.object.id.transmute(),
            version: self.object.version,
            object: WithGroup {
                group: self.group,
                object: self.object.object,
            },
        }
    }
}

impl<T> Versioned<WithGroup<T>> {
    pub fn transpose(self) -> WithGroup<Versioned<T>> {
        WithGroup {
            group: self.object.group,
            object: Versioned {
                id: self.id.transmute(),
                version: self.version,
                object: self.object.object,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Permissions {
    pub users: Map<Id<User>, AccessLevel>,
    pub default: AccessLevel,
}

impl Permissions {
    pub fn get(&self, id: Id<User>) -> AccessLevel {
        self.users.get(&id).copied().unwrap_or(self.default)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum AccessLevel {
    /// No access
    #[default]
    None,
    /// Read-only access
    Read,
    /// Read-write access
    Write,
}

/// Marker trait indicating that a type can be moved to a different group.
pub trait ChangeGroup {}

impl ChangeGroup for Group {}
