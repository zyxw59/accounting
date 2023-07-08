use serde::{Deserialize, Serialize, Serializer};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::{
    backend::{
        id::Id,
        query::{Query, QueryElement, Queryable, SerializedQuery, SimpleQuery},
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
    // { "name": { op: value }}
    Name(SimpleQuery<String>),
    /// Queries whether the user has any permissions specified, including `None`
    // { "permissions.users.0": value }
    UserAny(Id<User>),
    /// Queries whether the specified user has the specified permission
    // { "permissions.users": { "$elemMatch": { "0": id, "1": { op: value } }
    UserPerm(Id<User>, SimpleQuery<AccessLevel>),
}

impl Query<Group> for GroupQuery {
    fn matches(&self, group: &Group) -> bool {
        match self {
            Self::Name(query) => query.matches(&group.name),
            Self::UserAny(user) => group.permissions.users.contains_key(user),
            Self::UserPerm(user, access) => access.matches(&group.permissions.get(*user)),
        }
    }

    fn serialize_query<F, S>(&self, factory: &F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        match self {
            Self::Name(query) => Ok(SerializedQuery::from_path_and_query(
                "name",
                query.serialize_value(factory)?.into(),
            )),
            Self::UserAny(user) => Ok(SerializedQuery::from_path_and_query(
                "permissions.users",
                QueryElement::ElemMatch(SerializedQuery::from_path_and_query(
                    "0",
                    SimpleQuery::eq(user.serialize(factory())?).into(),
                )),
            )),
            Self::UserPerm(user, access) => Ok(SerializedQuery::from_path_and_query(
                "permissions.users",
                QueryElement::ElemMatch(SerializedQuery::from_path_queries([
                    ("0", SimpleQuery::eq(user.serialize(factory())?).into()),
                    ("1", access.serialize_value(factory)?.into()),
                ])),
            )),
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

/// Marker trait indicating that a type can be moved to a different group.
pub trait ChangeGroup {}

impl ChangeGroup for Group {}
