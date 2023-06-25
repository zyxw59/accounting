use std::{borrow::Cow, ops};

use serde::{Deserialize, Serialize, Serializer};

pub struct Query<T: Queryable> {
    expr: BooleanExpr<T::Query>,
}

impl<T: Queryable> Query<T> {
    pub fn new(query: T::Query) -> Self {
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

    pub fn expr(&self) -> &BooleanExpr<T::Query> {
        &self.expr
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

pub enum BooleanExpr<T> {
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
    type Query: QueryParameter<Self>;
}

pub trait QueryParameter<T>: Send {
    /// Whether the object matches this query.
    fn matches(&self, object: &T) -> bool;

    /// The path to the queried field in the serialized form of the object.
    fn path(&self) -> Cow<[&'static str]>;

    /// The comparison operator used for this query.
    fn comparator(&self) -> Comparator;

    /// Serializes the value of this query parameter.
    fn serialize_value<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

#[derive(Clone, Copy, Debug)]
pub enum Comparator {
    Equal,
    NotEqual,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
    In,
    NotIn,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SimpleQuery<T> {
    Equal(T),
    NotEqual(T),
    Greater(T),
    GreaterOrEqual(T),
    Less(T),
    LessOrEqual(T),
    In(Vec<T>),
    NotIn(Vec<T>),
}

impl<T> QueryParameter<T> for SimpleQuery<T>
where
    T: Ord + Serialize + Send,
{
    fn matches(&self, object: &T) -> bool {
        match self {
            Self::Equal(query) => object == query,
            Self::NotEqual(query) => object != query,
            Self::Greater(query) => object > query,
            Self::GreaterOrEqual(query) => object >= query,
            Self::Less(query) => object < query,
            Self::LessOrEqual(query) => object <= query,
            Self::In(query) => query.contains(object),
            Self::NotIn(query) => !query.contains(object),
        }
    }

    fn path(&self) -> Cow<[&'static str]> {
        Cow::Borrowed(&[])
    }

    fn comparator(&self) -> Comparator {
        match self {
            Self::Equal(_) => Comparator::Equal,
            Self::NotEqual(_) => Comparator::NotEqual,
            Self::Greater(_) => Comparator::Greater,
            Self::GreaterOrEqual(_) => Comparator::GreaterOrEqual,
            Self::Less(_) => Comparator::Less,
            Self::LessOrEqual(_) => Comparator::LessOrEqual,
            Self::In(_) => Comparator::In,
            Self::NotIn(_) => Comparator::NotIn,
        }
    }

    fn serialize_value<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Equal(val)
            | Self::NotEqual(val)
            | Self::Greater(val)
            | Self::GreaterOrEqual(val)
            | Self::Less(val)
            | Self::LessOrEqual(val) => val.serialize(serializer),
            Self::In(vals) | Self::NotIn(vals) => vals.serialize(serializer),
        }
    }
}
