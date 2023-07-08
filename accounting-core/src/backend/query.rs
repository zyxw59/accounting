use std::{borrow::Cow, collections::BTreeMap};

use serde::{Deserialize, Serialize, Serializer};

use crate::backend::{
    id::Id,
    user::{Group, WithGroup},
};

pub mod boolean;

pub use boolean::BooleanExpr;

pub trait Queryable: Sized {
    type Query: Query<Self> + Send;
}

pub trait Query<T> {
    /// Whether the object matches this query.
    fn matches(&self, object: &T) -> bool;

    /// Partially serialize the query, using the provided serializer factory to serialize the query
    /// values.
    fn serialize_query<F, S>(&self, factory: &F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer;
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum Comparator {
    #[serde(rename = "$eq")]
    Equal,
    #[serde(rename = "$ne")]
    NotEqual,
    #[serde(rename = "$gt")]
    Greater,
    #[serde(rename = "$gte")]
    GreaterOrEqual,
    #[serde(rename = "$lt")]
    Less,
    #[serde(rename = "$lte")]
    LessOrEqual,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SimpleQuery<T>(pub BTreeMap<Comparator, T>);

impl<T> SimpleQuery<T> {
    pub fn eq(value: T) -> Self {
        Self([(Comparator::Equal, value)].into_iter().collect())
    }
}

impl<T> SimpleQuery<T>
where
    T: Ord + Serialize + Send,
{
    pub fn matches(&self, object: &T) -> bool {
        self.0.iter().all(|(op, value)| match op {
            Comparator::Equal => object == value,
            Comparator::NotEqual => object != value,
            Comparator::Greater => object > value,
            Comparator::GreaterOrEqual => object >= value,
            Comparator::Less => object < value,
            Comparator::LessOrEqual => object <= value,
        })
    }

    pub fn serialize_value<F, S>(&self, factory: F) -> Result<SimpleQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        self.0
            .iter()
            .map(|(op, value)| Ok((*op, value.serialize(factory())?)))
            .collect::<Result<_, _>>()
            .map(SimpleQuery)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GroupQuery<T: Queryable> {
    Group {
        #[serde(rename = "_group")]
        group: Id<Group>,
    },
    Other(T::Query),
}

impl<T> Query<WithGroup<T>> for GroupQuery<T>
where
    T: Queryable,
{
    fn matches(&self, object: &WithGroup<T>) -> bool {
        match self {
            Self::Group { group } => *group == object.group,
            Self::Other(query) => query.matches(&object.object),
        }
    }

    fn serialize_query<F, S>(&self, factory: &F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        match self {
            Self::Group { group } => Ok(SerializedQuery::from_path_and_query(
                "_group",
                SimpleQuery::eq(group.serialize(factory())?).into(),
            )),
            Self::Other(query) => query.serialize_query(factory),
        }
    }
}

/// A query with the structure preserved, but the query values replaced by their serialized forms.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializedQuery<T> {
    expr: BooleanExpr<BTreeMap<Cow<'static, str>, QueryElement<T>>>,
}

impl<T> SerializedQuery<T> {
    pub(crate) fn from_path_and_query(path: &'static str, query: QueryElement<T>) -> Self {
        Self::from_path_queries([(path, query)])
    }

    pub(crate) fn from_path_queries(
        queries: impl IntoIterator<Item = (&'static str, QueryElement<T>)>,
    ) -> Self {
        Self {
            expr: BooleanExpr::Value(
                queries
                    .into_iter()
                    .map(|(path, query)| (Cow::Borrowed(path), query))
                    .collect(),
            ),
        }
    }

    pub(crate) fn and(self, other: Self) -> Self {
        Self {
            expr: self.expr.and(other.expr),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueryElement<T> {
    ElemMatch(SerializedQuery<T>),
    Comparator(SimpleQuery<T>),
    Not(Box<QueryElement<T>>),
}

impl<T> From<SimpleQuery<T>> for QueryElement<T> {
    fn from(query: SimpleQuery<T>) -> Self {
        Self::Comparator(query)
    }
}
