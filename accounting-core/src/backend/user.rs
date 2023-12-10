use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::{
    backend::{
        id::Id,
        query::{Query, Queryable, SimpleQuery},
        version::Versioned,
    },
    map::Map,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub is_superuser: bool,
}

impl Queryable for User {
    const TYPE_NAME: &'static str = "user";

    type Query = UserQuery;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum UserQuery {
    Name(SimpleQuery<String>),
}

impl Query<User> for UserQuery {
    fn matches(&self, user: &User) -> bool {
        match self {
            Self::Name(query) => query.matches(&user.name),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Group {
    pub name: String,
    pub permissions: Permissions,
}

impl Queryable for Group {
    const TYPE_NAME: &'static str = "group";

    type Query = GroupQuery;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum GroupQuery {
    Name(SimpleQuery<String>),
    UserAny(Vec<Id<User>>),
    UserPerm(Id<User>, SimpleQuery<AccessLevel>),
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
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[repr(i8)]
pub enum AccessLevel {
    /// No access
    #[default]
    None,
    /// Read-only access
    Read,
    /// Read-write access
    Write,
}

#[cfg(feature = "sqlx-postgres")]
impl sqlx::postgres::PgHasArrayType for AccessLevel {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        <i8 as sqlx::postgres::PgHasArrayType>::array_type_info()
    }
}

/// Marker trait indicating that a type can be moved to a different group.
pub trait ChangeGroup {}

impl ChangeGroup for Group {}

impl<T: Queryable> Queryable for WithGroup<T> {
    const TYPE_NAME: &'static str = T::TYPE_NAME;

    type Query = WithGroupQuery<T>;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(bound(
    deserialize = "T::Query: Deserialize<'de>",
    serialize = "T::Query: Serialize"
))]
pub enum WithGroupQuery<T: Queryable> {
    Group(Vec<Id<Group>>),
    Other(T::Query),
}

impl<T> Query<WithGroup<T>> for WithGroupQuery<T>
where
    T: Queryable,
{
    fn matches(&self, object: &WithGroup<T>) -> bool {
        match self {
            Self::Group(groups) => groups.contains(&object.group),
            Self::Other(query) => query.matches(&object.object),
        }
    }
}
