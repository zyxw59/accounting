use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::{
    backend::{
        id::Id,
        query::{Query, Queryable, RawQuery, SimpleQuery, ToValue},
        version::Versioned,
    },
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

impl Queryable for Group {
    type Query = GroupQuery;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum GroupQuery {
    Name(SimpleQuery<String>),
    UserAny(Vec<Id<User>>),
    UserPerm(Id<User>, SimpleQuery<AccessLevel>),
}

impl GroupQuery {
    const USER_PERMISSIONS_PARAMETER: &'static str = "user_permissions";
}

impl Query<Group> for GroupQuery {
    fn matches(&self, group: &Group) -> bool {
        match self {
            Self::Name(query) => query.matches(&group.name),
            Self::UserAny(users) => users
                .iter()
                .any(|user| group.permissions.users.contains_key(user)),
            Self::UserPerm(user, query) => query.matches(&group.permissions.get(*user)),
        }
    }

    fn as_raw_query(&self) -> RawQuery {
        match self {
            Self::Name(query) => RawQuery::simple("name", query.to_value_query()),
            Self::UserAny(users) => RawQuery::complex(
                Self::USER_PERMISSIONS_PARAMETER,
                [(
                    "user",
                    SimpleQuery {
                        in_: Some(users.iter().map(ToValue::to_value).collect()),
                        ..Default::default()
                    }
                    .into(),
                )],
            ),
            Self::UserPerm(user, permissions) => RawQuery::complex(
                Self::USER_PERMISSIONS_PARAMETER,
                [
                    (
                        "user",
                        SimpleQuery {
                            eq: Some(user.to_value()),
                            ..Default::default()
                        }
                        .into(),
                    ),
                    ("access", permissions.to_value_query()),
                ],
            ),
        }
    }
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

#[derive(
    Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Deserialize_repr, Serialize_repr,
)]
#[repr(u8)]
pub enum AccessLevel {
    /// No access
    #[default]
    None,
    /// Read-only access
    Read,
    /// Read-write access
    Write,
}

impl ToValue for AccessLevel {
    type Value<'a> = i32;

    fn to_value(&self) -> Self::Value<'_> {
        *self as i32
    }
}

/// Marker trait indicating that a type can be moved to a different group.
pub trait ChangeGroup {}

impl ChangeGroup for Group {}
