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
#[serde(bound(
    deserialize = "T::Query: Deserialize<'de>",
    serialize = "T::Query: Serialize"
))]
pub struct GroupQuery<T: Queryable> {
    #[serde(rename = "_group", skip_serializing_if = "Option::is_none")]
    pub group: Option<Vec<Id<Group>>>,
    #[serde(flatten)]
    pub other: Option<T::Query>,
}

impl<T> Query<WithGroup<T>> for GroupQuery<T>
where
    T: Queryable,
{
    fn matches(&self, object: &WithGroup<T>) -> bool {
        self.group
            .as_ref()
            .map(|groups| groups.contains(&object.group))
            .unwrap_or(true)
            && self
                .other
                .as_ref()
                .map(|query| query.matches(&object.object))
                .unwrap_or(true)
    }

    fn serialize_query<F, S>(&self, factory: &F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        let group_query = self
            .group
            .as_ref()
            .map(|groups| {
                groups
                    .iter()
                    .map(|gr| gr.serialize(factory()))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .map(|groups| SerializedQuery::from_path_and_query("_group", QueryElement::In(groups)));
        let other_query = self
            .other
            .as_ref()
            .map(|query| query.serialize_query(factory))
            .transpose()?;
        Ok(SerializedQuery::all_opt([group_query, other_query]))
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

    pub(crate) fn all_opt(queries: impl IntoIterator<Item = Option<Self>>) -> Self {
        let mut query = Self {
            expr: BooleanExpr::All(Vec::new()),
        };
        for q in queries.into_iter().flatten() {
            query = query.and(q);
        }
        query
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
    In(Vec<T>),
    Not(Box<QueryElement<T>>),
}

impl<T> From<SimpleQuery<T>> for QueryElement<T> {
    fn from(query: SimpleQuery<T>) -> Self {
        Self::Comparator(query)
    }
}
