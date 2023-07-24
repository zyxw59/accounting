use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::{
    backend::{
        id::Id,
        query::{Index, Query, Queryable, RawQuery, SimpleQuery, ToValue, REFERENCES_PARAMETER},
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

    fn indices(&self) -> Vec<Index> {
        let mut indices = Vec::with_capacity(self.permissions.users.len() + 1);
        indices.push(Index::simple(GroupQuery::NAME_PARAMETER, self.name.clone()));
        indices.extend(
            self.permissions
                .users
                .keys()
                .map(|user| Index::simple(REFERENCES_PARAMETER, *user)),
        );
        indices.extend(self.permissions.users.iter().map(|(user, access)| {
            Index::complex(
                GroupQuery::USER_ACCESS_PARAMETER,
                [
                    (GroupQuery::USER_PARAMETER, (*user).into()),
                    (GroupQuery::ACCESS_PARAMETER, (*access as i32).into()),
                ],
            )
        }));
        indices
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum GroupQuery {
    Name(SimpleQuery<String>),
    UserAny(Vec<Id<User>>),
    UserPerm(Id<User>, SimpleQuery<AccessLevel>),
}

impl GroupQuery {
    const NAME_PARAMETER: &str = "name";
    const USER_ACCESS_PARAMETER: &str = "user_access";
    const USER_PARAMETER: &str = "user";
    const ACCESS_PARAMETER: &str = "access";
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
            Self::Name(query) => RawQuery::simple(Self::NAME_PARAMETER, query.to_value_query()),
            Self::UserAny(users) => RawQuery::complex(
                Self::USER_ACCESS_PARAMETER,
                [(
                    Self::USER_PARAMETER,
                    SimpleQuery {
                        in_: Some(users.iter().map(ToValue::to_value).collect()),
                        ..Default::default()
                    }
                    .into(),
                )],
            ),
            Self::UserPerm(user, permissions) => RawQuery::complex(
                Self::USER_ACCESS_PARAMETER,
                [
                    (
                        Self::USER_PARAMETER,
                        SimpleQuery {
                            eq: Some(user.to_value()),
                            ..Default::default()
                        }
                        .into(),
                    ),
                    (Self::ACCESS_PARAMETER, permissions.to_value_query()),
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

impl<T: Queryable> Queryable for WithGroup<T> {
    type Query = WithGroupQuery<T>;

    fn indices(&self) -> Vec<Index> {
        let mut indices = self.object.indices();
        indices.push(Index::simple(REFERENCES_PARAMETER, self.group));
        indices.push(Index::simple(
            WithGroupQuery::<T>::GROUP_PARAMETER,
            self.group,
        ));
        indices
    }
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

impl<T: Queryable> WithGroupQuery<T> {
    const GROUP_PARAMETER: &str = "_group";
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

    fn as_raw_query(&self) -> RawQuery {
        match self {
            Self::Group(groups) => RawQuery::simple(
                Self::GROUP_PARAMETER,
                SimpleQuery {
                    in_: Some(groups.iter().copied().map(Id::transmute).collect()),
                    ..Default::default()
                }
                .into(),
            ),
            Self::Other(query) => query.as_raw_query(),
        }
    }
}
