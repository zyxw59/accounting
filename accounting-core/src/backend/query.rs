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
    type Query: Query<Self> + Send + Sync;

    fn indices(&self) -> Vec<Index>;
}

pub trait Query<T> {
    /// Whether the object matches this query.
    fn matches(&self, object: &T) -> bool;

    fn as_raw_query(&self) -> RawQuery;
}

pub enum Index {
    Simple {
        parameter: &'static str,
        value: IndexValue,
    },
    Complex {
        parameter: &'static str,
        values: BTreeMap<&'static str, IndexValue>,
    },
}

impl Index {
    pub fn simple(parameter: &'static str, value: impl Into<IndexValue>) -> Self {
        Self::Simple {
            parameter,
            value: value.into(),
        }
    }

    pub fn complex(
        parameter: &'static str,
        values: impl IntoIterator<Item = (&'static str, IndexValue)>,
    ) -> Self {
        Self::Complex {
            parameter,
            values: values.into_iter().collect(),
        }
    }
}

pub enum IndexValue {
    String(String),
    Id(Id<()>),
    Integer(i32),
    Amount(Amount),
    Date(Date),
}

impl From<String> for IndexValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl<T> From<Id<T>> for IndexValue {
    fn from(value: Id<T>) -> Self {
        Self::Id(value.transmute())
    }
}

impl From<i32> for IndexValue {
    fn from(value: i32) -> Self {
        Self::Integer(value)
    }
}

impl From<Amount> for IndexValue {
    fn from(value: Amount) -> Self {
        Self::Amount(value)
    }
}

impl From<Date> for IndexValue {
    fn from(value: Date) -> Self {
        Self::Date(value)
    }
}

pub enum RawQuery<'a> {
    /// Query of a simple search parameter
    Simple {
        /// Name of the search parameter
        parameter: &'static str,
        /// Value(s) to search for
        query: Box<SimpleValueQuery<'a>>,
    },
    /// Query of a search parameter with multiple values
    Complex {
        /// Name of the search parameter
        parameter: &'static str,
        /// Values to search for in each value of the seach parameter
        queries: BTreeMap<&'static str, SimpleValueQuery<'a>>,
    },
}

impl<'a> RawQuery<'a> {
    pub fn simple(parameter: &'static str, query: SimpleValueQuery<'a>) -> Self {
        Self::Simple {
            parameter,
            query: Box::new(query),
        }
    }

    pub fn complex<I>(parameter: &'static str, queries: I) -> Self
    where
        I: IntoIterator<Item = (&'static str, SimpleValueQuery<'a>)>,
    {
        Self::Complex {
            parameter,
            queries: queries.into_iter().collect(),
        }
    }
}

pub trait Value<'a>: Sized {
    fn to_value_query(query: SimpleQuery<Self>) -> SimpleValueQuery<'a>;
}

impl<'a> Value<'a> for Cow<'a, str> {
    fn to_value_query(query: SimpleQuery<Self>) -> SimpleValueQuery<'a> {
        SimpleValueQuery::String(query)
    }
}

impl<'a> Value<'a> for Id<()> {
    fn to_value_query(query: SimpleQuery<Self>) -> SimpleValueQuery<'a> {
        SimpleValueQuery::Id(query)
    }
}

impl<'a> Value<'a> for i32 {
    fn to_value_query(query: SimpleQuery<Self>) -> SimpleValueQuery<'a> {
        SimpleValueQuery::Integer(query)
    }
}

impl<'a> Value<'a> for Amount {
    fn to_value_query(query: SimpleQuery<Self>) -> SimpleValueQuery<'a> {
        SimpleValueQuery::Amount(query)
    }
}

impl<'a> Value<'a> for Date {
    fn to_value_query(query: SimpleQuery<Self>) -> SimpleValueQuery<'a> {
        SimpleValueQuery::Date(query)
    }
}

pub trait ToValue {
    type Value<'a>: Value<'a>
    where
        Self: 'a;

    fn to_value(&self) -> Self::Value<'_>;
}

impl ToValue for String {
    type Value<'a> = Cow<'a, str>;

    fn to_value(&self) -> Self::Value<'_> {
        Cow::Borrowed(self)
    }
}

impl<T> ToValue for Id<T> {
    type Value<'a> = Id<()> where T: 'a;

    fn to_value(&self) -> Self::Value<'_> {
        self.transmute()
    }
}

impl ToValue for Date {
    type Value<'a> = Date;

    fn to_value(&self) -> Self::Value<'_> {
        *self
    }
}

impl ToValue for Amount {
    type Value<'a> = Amount;

    fn to_value(&self) -> Self::Value<'_> {
        *self
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

impl<T> SimpleQuery<T> {
    pub fn map_ref<'a, F, U>(&'a self, f: F) -> SimpleQuery<U>
    where
        F: Fn(&'a T) -> U,
        T: 'a,
    {
        SimpleQuery {
            eq: self.eq.as_ref().map(&f),
            ne: self.ne.as_ref().map(&f),
            gt: self.gt.as_ref().map(&f),
            ge: self.ge.as_ref().map(&f),
            lt: self.lt.as_ref().map(&f),
            le: self.le.as_ref().map(&f),
            in_: self.in_.as_ref().map(|v| v.iter().map(&f).collect()),
            nin: self.nin.as_ref().map(|v| v.iter().map(&f).collect()),
        }
    }

    pub fn map<F, U>(self, f: F) -> SimpleQuery<U>
    where
        F: Fn(T) -> U,
    {
        SimpleQuery {
            eq: self.eq.map(&f),
            ne: self.ne.map(&f),
            gt: self.gt.map(&f),
            ge: self.ge.map(&f),
            lt: self.lt.map(&f),
            le: self.le.map(&f),
            in_: self.in_.map(|v| v.into_iter().map(&f).collect()),
            nin: self.nin.map(|v| v.into_iter().map(&f).collect()),
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
    pub fn to_value_query(&self) -> SimpleValueQuery<'_> {
        // the closure here is to avoid a lifetime issue
        // `T::to_value` satisfies `for<'a> F: Fn(&'a T) -> T::Value<'a>`, which requires
        // `for<'a> T: 'a`, which we can't gurantee without `T: 'static`
        // `|val| val.to_value()` just satisfies `F: Fn(&'a T) -> T::Value<'a>` for the specific
        // lifetime it is called with, which doesn't impose any lifetime requirements on `T`
        self.map_ref(|val| val.to_value()).into()
    }
}

impl<'a, T: Value<'a>> From<SimpleQuery<T>> for SimpleValueQuery<'a> {
    fn from(query: SimpleQuery<T>) -> SimpleValueQuery<'a> {
        T::to_value_query(query)
    }
}

pub enum SimpleValueQuery<'a> {
    String(SimpleQuery<Cow<'a, str>>),
    Id(SimpleQuery<Id<()>>),
    Integer(SimpleQuery<i32>),
    Amount(SimpleQuery<Amount>),
    Date(SimpleQuery<Date>),
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
                    in_: Some(groups.iter().copied().map(Id::transmute).collect()),
                    ..Default::default()
                }
                .into(),
            ),
            Self::Other(query) => query.as_raw_query(),
        }
    }
}
