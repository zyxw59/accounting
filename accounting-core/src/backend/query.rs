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
