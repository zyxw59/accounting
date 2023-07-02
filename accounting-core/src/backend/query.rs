use std::{borrow::Cow, collections::BTreeMap, ops};

use serde::{Deserialize, Serialize, Serializer};

use crate::backend::{
    id::Id,
    user::{Group, WithGroup},
};

pub struct Query<T: Queryable> {
    expr: BooleanExpr<GroupQuery<T>>,
}

impl<T: Queryable> Query<T> {
    pub fn new(query: GroupQuery<T>) -> Self {
        Self {
            expr: BooleanExpr::Value(query),
        }
    }

    pub fn and(self, other: Self) -> Self {
        Self {
            expr: self.expr.and(other.expr),
        }
    }

    pub fn or(self, other: Self) -> Self {
        Self {
            expr: self.expr.or(other.expr),
        }
    }

    pub fn serialize_query<F, S>(&self, factory: F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        self.expr.try_fold(
            &|clauses| Ok(SerializedQuery::Boolean(SimpleBooleanExpr::And(clauses))),
            &|clauses| Ok(SerializedQuery::Boolean(SimpleBooleanExpr::Or(clauses))),
            &|expr| {
                Ok(SerializedQuery::Boolean(SimpleBooleanExpr::Not(Box::new(
                    expr,
                ))))
            },
            &|query| query.serialize_query(&factory),
        )
    }
}

impl<T: Queryable> ops::Not for Query<T> {
    type Output = Self;

    fn not(self) -> Self {
        Self {
            expr: self.expr.not(),
        }
    }
}

enum BooleanExpr<T> {
    Value(T),
    Not(Box<Self>),
    Any(Vec<Self>),
    All(Vec<Self>),
}

impl<T> BooleanExpr<T> {
    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::All(mut a), Self::All(mut b)) => {
                a.append(&mut b);
                Self::All(a)
            }
            (Self::All(mut a), other) | (other, Self::All(mut a)) => {
                a.push(other);
                Self::All(a)
            }
            (a, b) => Self::All(vec![a, b]),
        }
    }

    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Any(mut a), Self::Any(mut b)) => {
                a.append(&mut b);
                Self::Any(a)
            }
            (Self::Any(mut a), other) | (other, Self::Any(mut a)) => {
                a.push(other);
                Self::Any(a)
            }
            (a, b) => Self::Any(vec![a, b]),
        }
    }

    fn try_fold<U, E>(
        &self,
        all_f: &impl Fn(Vec<U>) -> Result<U, E>,
        any_f: &impl Fn(Vec<U>) -> Result<U, E>,
        not_f: &impl Fn(U) -> Result<U, E>,
        f: &impl Fn(&T) -> Result<U, E>,
    ) -> Result<U, E> {
        match self {
            BooleanExpr::All(clauses) => {
                let clauses = clauses
                    .iter()
                    .map(|expr| expr.try_fold(all_f, any_f, not_f, f))
                    .collect::<Result<_, E>>()?;
                all_f(clauses)
            }
            BooleanExpr::Any(clauses) => {
                let clauses = clauses
                    .iter()
                    .map(|expr| expr.try_fold(all_f, any_f, not_f, f))
                    .collect::<Result<_, E>>()?;
                any_f(clauses)
            }
            BooleanExpr::Not(expr) => not_f(expr.try_fold(all_f, any_f, not_f, f)?),
            BooleanExpr::Value(value) => f(value),
        }
    }
}

impl<T> ops::Not for BooleanExpr<T> {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            Self::Not(a) => *a,
            a => Self::Not(Box::new(a)),
        }
    }
}

pub trait Queryable: Sized {
    type Query: QueryParameter<Self> + Send;
}

pub trait QueryParameter<T> {
    /// Whether the object matches this query.
    fn matches(&self, object: &T) -> bool;

    /// Partially serialize the query, using the provided serializer factory to serialize the query
    /// values.
    fn serialize_query<F, S>(&self, factory: F) -> Result<SerializedQuery<S::Ok>, S::Error>
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

impl<T> QueryParameter<T> for SimpleQuery<T>
where
    T: Ord + Serialize + Send,
{
    fn matches(&self, object: &T) -> bool {
        self.0.iter().all(|(op, value)| match op {
            Comparator::Equal => object == value,
            Comparator::NotEqual => object != value,
            Comparator::Greater => object > value,
            Comparator::GreaterOrEqual => object >= value,
            Comparator::Less => object < value,
            Comparator::LessOrEqual => object <= value,
        })
    }

    fn serialize_query<F, S>(&self, factory: F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        self.0
            .iter()
            .map(|(op, value)| {
                value
                    .serialize(factory())
                    .map(|ser| (*op, SerializedQuery::Value(ser)))
            })
            .collect::<Result<_, _>>()
            .map(SerializedQuery::Comparator)
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

impl<T> QueryParameter<WithGroup<T>> for GroupQuery<T>
where
    T: Queryable,
{
    fn matches(&self, object: &WithGroup<T>) -> bool {
        match self {
            Self::Group { group } => *group == object.group,
            Self::Other(query) => query.matches(&object.object),
        }
    }

    fn serialize_query<F, S>(&self, factory: F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        match self {
            Self::Group { group } => Ok(SerializedQuery::from_path_and_query(
                &["_group"],
                SerializedQuery::Value(group.serialize(factory())?),
            )),
            Self::Other(query) => query.serialize_query(factory),
        }
    }
}

/// A query with the structure preserved, but the query values replaced by their serialized forms.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SerializedQuery<T> {
    Boolean(SimpleBooleanExpr<SerializedQuery<T>>),
    Comparator(BTreeMap<Comparator, SerializedQuery<T>>),
    Paths(BTreeMap<Cow<'static, str>, SerializedQuery<T>>),
    Value(T),
}

impl<T> SerializedQuery<T> {
    pub fn from_path_and_query(path: &[&'static str], mut query: Self) -> Self {
        for field in path.iter().rev() {
            let map = [(Cow::Borrowed(*field), query)].into_iter().collect();
            query = SerializedQuery::Paths(map);
        }
        query
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimpleBooleanExpr<T> {
    #[serde(rename = "$not")]
    Not(Box<T>),
    #[serde(rename = "$or")]
    Or(Vec<T>),
    #[serde(rename = "$and")]
    And(Vec<T>),
}
