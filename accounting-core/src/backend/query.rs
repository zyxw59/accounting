use std::{borrow::Cow, collections::BTreeMap};

use derivative::Derivative;
use serde::{Deserialize, Serialize};

use crate::{
    backend::{
        id::Id,
        user::{Group, WithGroup},
    },
    date::Date,
    public::amount::Amount,
};

pub trait Queryable: Sized {
    type Query: Query<Self> + Send;
}

pub trait Query<T> {
    /// Whether the object matches this query.
    fn matches(&self, object: &T) -> bool;

    fn as_raw_query(&self) -> RawQuery;
}

pub enum RawQuery<'a> {
    /// Query of a simple search parameter
    Simple {
        /// Name of the search parameter
        parameter: &'static str,
        /// Value(s) to search for
        query: Box<SimpleQuery<Value<'a>>>,
    },
    /// Query of a search parameter with multiple values
    Complex {
        /// Name of the search parameter
        parameter: &'static str,
        /// Values to search for in each value of the seach parameter
        queries: BTreeMap<&'static str, SimpleQuery<Value<'a>>>,
    },
}

impl<'a> RawQuery<'a> {
    pub fn simple(parameter: &'static str, query: SimpleQuery<Value<'a>>) -> Self {
        Self::Simple {
            parameter,
            query: Box::new(query),
        }
    }

    pub fn complex<I>(parameter: &'static str, queries: I) -> Self
    where
        I: IntoIterator<Item = (&'static str, SimpleQuery<Value<'a>>)>,
    {
        Self::Complex {
            parameter,
            queries: queries.into_iter().collect(),
        }
    }
}

pub enum Value<'a> {
    String(Cow<'a, str>),
    Id(Id<()>),
    Integer(i32),
    Amount(Amount),
    Date(Date),
}

pub trait ToValue {
    fn to_value(&self) -> Value;
}

impl ToValue for str {
    fn to_value(&self) -> Value {
        Value::String(Cow::Borrowed(self))
    }
}

impl ToValue for String {
    fn to_value(&self) -> Value {
        Value::String(Cow::Borrowed(self))
    }
}

impl<T> ToValue for Id<T> {
    fn to_value(&self) -> Value {
        Value::Id(self.transmute())
    }
}

impl ToValue for i32 {
    fn to_value(&self) -> Value {
        Value::Integer(*self)
    }
}

impl ToValue for Amount {
    fn to_value(&self) -> Value {
        Value::Amount(*self)
    }
}

impl ToValue for Date {
    fn to_value(&self) -> Value {
        Value::Date(*self)
    }
}

#[derive(Clone, Debug, Derivative, Deserialize, Serialize)]
#[derivative(Default(bound = ""))]
pub struct SimpleQuery<T> {
    pub eq: Option<T>,
    pub ne: Option<T>,
    pub gt: Option<T>,
    pub ge: Option<T>,
    pub lt: Option<T>,
    pub le: Option<T>,
    pub in_: Option<Vec<T>>,
    pub nin: Option<Vec<T>>,
}

impl<T> SimpleQuery<T> {
    pub fn eq(value: T) -> Self {
        Self {
            eq: Some(value),
            ..Default::default()
        }
    }
}

impl<T> SimpleQuery<T>
where
    T: Ord,
{
    pub fn matches(&self, object: &T) -> bool {
        self.eq
            .as_ref()
            .map(|value| object == value)
            .unwrap_or(true)
            && self
                .ne
                .as_ref()
                .map(|value| object != value)
                .unwrap_or(true)
            && self.gt.as_ref().map(|value| object > value).unwrap_or(true)
            && self
                .ge
                .as_ref()
                .map(|value| object >= value)
                .unwrap_or(true)
            && self.lt.as_ref().map(|value| object < value).unwrap_or(true)
            && self
                .le
                .as_ref()
                .map(|value| object <= value)
                .unwrap_or(true)
            && self
                .in_
                .as_ref()
                .map(|values| values.contains(object))
                .unwrap_or(true)
            && self
                .nin
                .as_ref()
                .map(|values| !values.contains(object))
                .unwrap_or(true)
    }
}

impl<T> SimpleQuery<T>
where
    T: ToValue,
{
    pub fn to_value_query(&self) -> SimpleQuery<Value> {
        SimpleQuery {
            eq: self.eq.as_ref().map(ToValue::to_value),
            ne: self.ne.as_ref().map(ToValue::to_value),
            gt: self.gt.as_ref().map(ToValue::to_value),
            ge: self.ge.as_ref().map(ToValue::to_value),
            lt: self.lt.as_ref().map(ToValue::to_value),
            le: self.le.as_ref().map(ToValue::to_value),
            in_: self
                .in_
                .as_ref()
                .map(|v| v.iter().map(ToValue::to_value).collect()),
            nin: self
                .nin
                .as_ref()
                .map(|v| v.iter().map(ToValue::to_value).collect()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(bound(
    deserialize = "T::Query: Deserialize<'de>",
    serialize = "T::Query: Serialize"
))]
pub enum GroupQuery<T: Queryable> {
    Group(Vec<Id<Group>>),
    Other(T::Query),
}

impl<T> Query<WithGroup<T>> for GroupQuery<T>
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
                "_group",
                SimpleQuery {
                    in_: Some(groups.iter().map(ToValue::to_value).collect()),
                    ..Default::default()
                },
            ),
            Self::Other(query) => query.as_raw_query(),
        }
    }
}
