use derivative::Derivative;
use serde::{Deserialize, Serialize};

pub trait Queryable: Sized {
    const TYPE_NAME: &'static str;

    type Query: Query<Self> + Send + Sync;
}

pub trait Query<T> {
    /// Whether the object matches this query.
    fn matches(&self, object: &T) -> bool;
}

#[derive(Debug, Derivative, Serialize)]
#[derivative(Default(bound = ""))]
pub struct SimpleQueryRef<'a, T> {
    pub eq: Option<&'a T>,
    pub ne: Option<&'a T>,
    pub gt: Option<&'a T>,
    pub ge: Option<&'a T>,
    pub lt: Option<&'a T>,
    pub le: Option<&'a T>,
    pub in_: Option<&'a [T]>,
    pub nin: Option<&'a [T]>,
}

impl<T> Copy for SimpleQueryRef<'_, T> {}
impl<T> Clone for SimpleQueryRef<'_, T> {
    fn clone(&self) -> Self {
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

    pub fn in_(values: Vec<T>) -> Self {
        Self {
            in_: Some(values),
            ..Default::default()
        }
    }

    pub fn as_ref(&self) -> SimpleQueryRef<T> {
        SimpleQueryRef {
            eq: self.eq.as_ref(),
            ne: self.ne.as_ref(),
            gt: self.gt.as_ref(),
            ge: self.ge.as_ref(),
            lt: self.lt.as_ref(),
            le: self.le.as_ref(),
            in_: self.in_.as_deref(),
            nin: self.nin.as_deref(),
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
        self.as_ref().matches(object)
    }
}

impl<'a, T> SimpleQueryRef<'a, T> {
    pub fn eq(value: &'a T) -> Self {
        Self {
            eq: Some(value),
            ..Default::default()
        }
    }

    pub fn in_(values: &'a [T]) -> Self {
        Self {
            in_: Some(values),
            ..Default::default()
        }
    }

    pub fn map<F, U>(&self, f: F) -> SimpleQuery<U>
    where
        F: Fn(&'a T) -> U,
        T: 'a,
    {
        SimpleQuery {
            eq: self.eq.map(&f),
            ne: self.ne.map(&f),
            gt: self.gt.map(&f),
            ge: self.ge.map(&f),
            lt: self.lt.map(&f),
            le: self.le.map(&f),
            in_: self.in_.map(|v| v.iter().map(&f).collect()),
            nin: self.nin.map(|v| v.iter().map(&f).collect()),
        }
    }
}

impl<T> SimpleQueryRef<'_, T>
where
    T: Ord,
{
    pub fn matches(&self, object: &T) -> bool {
        self.eq.map(|value| object == value).unwrap_or(true)
            && self.ne.map(|value| object != value).unwrap_or(true)
            && self.gt.map(|value| object > value).unwrap_or(true)
            && self.ge.map(|value| object >= value).unwrap_or(true)
            && self.lt.map(|value| object < value).unwrap_or(true)
            && self.le.map(|value| object <= value).unwrap_or(true)
            && self
                .in_
                .map(|values| values.contains(object))
                .unwrap_or(true)
            && self
                .nin
                .map(|values| !values.contains(object))
                .unwrap_or(true)
    }
}
