//! A typed 64-bit identifier for a resource.

use std::{fmt, marker::PhantomData};

use derivative::Derivative;
use rand::distributions::{Distribution, Standard};
use serde::{Deserialize, Serialize};

/// A typed 64-bit identifier for a resource.
#[derive(Derivative, Deserialize, Serialize)]
#[derivative(
    Clone(bound = ""),
    Copy(bound = ""),
    Eq(bound = ""),
    Hash(bound = ""),
    Ord(bound = ""),
    PartialEq(bound = ""),
    PartialOrd(bound = "")
)]
#[serde(bound = "", transparent)]
pub struct Id<T> {
    id: u64,
    // `PhantomData<fn() -> T>` is covariant in `T`, but unlike `PhantomData<T>` or
    // `PhantomData<*const T>`, it is always `Send` and `Sync`
    _marker: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// Generate a new random `Id`
    pub fn new_random() -> Self {
        rand::random()
    }

    /// Generate a new `Id` with the specified value
    pub fn new(id: u64) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }

    /// Produce an identical `Id` for a different type
    pub fn transmute<U>(self) -> Id<U> {
        Id {
            _marker: PhantomData,
            id: self.id,
        }
    }

    fn _check_send_sync(self) -> impl Send + Sync {
        self
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple(std::any::type_name::<T>())
            .field(&self.id)
            .finish()
    }
}

impl<T> From<Id<T>> for bson::Bson {
    fn from(id: Id<T>) -> Self {
        bson::Bson::Int64(id.id as i64)
    }
}

impl<T> Distribution<Id<T>> for Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Id<T> {
        Id {
            id: rng.next_u64(),
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct WithId<T> {
    pub id: Id<T>,
    #[serde(flatten)]
    pub object: T,
}
