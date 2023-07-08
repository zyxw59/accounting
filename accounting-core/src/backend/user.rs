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
pub struct GroupQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<SimpleQuery<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_any: Option<Vec<Id<User>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_perm: Option<(Id<User>, SimpleQuery<AccessLevel>)>,
}

impl Query<Group> for GroupQuery {
    fn matches(&self, group: &Group) -> bool {
        self.name
            .as_ref()
            .map(|q| q.matches(&group.name))
            .unwrap_or(true)
            && self
                .user_any
                .as_ref()
                .map(|users| {
                    users
                        .iter()
                        .any(|user| group.permissions.users.contains_key(user))
                })
                .unwrap_or(true)
            && self
                .user_perm
                .as_ref()
                .map(|(user, q)| q.matches(&group.permissions.get(*user)))
                .unwrap_or(true)
    }

    fn serialize_query<F, S>(&self, factory: &F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        let name_query = self
            .name
            .as_ref()
            .map(|query| {
                Ok(SerializedQuery::from_path_and_query(
                    "name",
                    query.serialize_value(factory)?.into(),
                ))
            })
            .transpose()?;
        let user_any_query = self
            .user_any
            .as_ref()
            .map(|users| {
                Ok(SerializedQuery::from_path_and_query(
                    "permissions.users",
                    QueryElement::ElemMatch(SerializedQuery::from_path_and_query(
                        "0",
                        QueryElement::In(
                            users
                                .iter()
                                .map(|user| user.serialize(factory()))
                                .collect::<Result<_, _>>()?,
                        ),
                    )),
                ))
            })
            .transpose()?;
        let user_perm_query = self
            .user_perm
            .as_ref()
            .map(|(user, access)| {
                Ok(SerializedQuery::from_path_and_query(
                    "permissions.users",
                    QueryElement::ElemMatch(SerializedQuery::from_path_queries([
                        ("0", SimpleQuery::eq(user.serialize(factory())?).into()),
                        ("1", access.serialize_value(factory)?.into()),
                    ])),
                ))
            })
            .transpose()?;

        Ok(SerializedQuery::all_opt([
            name_query,
            user_any_query,
            user_perm_query,
        ]))
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
