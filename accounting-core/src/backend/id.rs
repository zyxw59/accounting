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

#[cfg(feature = "sqlx")]
impl<DB, T> sqlx::Type<DB> for Id<T>
where
    DB: sqlx::Database,
    i64: sqlx::Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <i64 as sqlx::Type<DB>>::type_info()
    }
}

#[cfg(feature = "sqlx-postgres")]
impl<T> sqlx::postgres::PgHasArrayType for Id<T> {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        <i64 as sqlx::postgres::PgHasArrayType>::array_type_info()
    }
}

#[cfg(feature = "sqlx")]
impl<'q, DB, T> sqlx::Encode<'q, DB> for Id<T>
where
    DB: sqlx::Database,
    i64: sqlx::Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        <i64 as sqlx::Encode<'q, DB>>::encode_by_ref(&(self.id as i64), buf)
    }
}

#[cfg(feature = "sqlx")]
impl<'r, DB, T> sqlx::Decode<'r, DB> for Id<T>
where
    DB: sqlx::Database,
    i64: sqlx::Decode<'r, DB>,
{
    fn decode(
        value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        <i64 as sqlx::Decode<'r, DB>>::decode(value).map(|id| Self::new(id as _))
    }
}

impl<T> From<Id<T>> for u64 {
    fn from(id: Id<T>) -> u64 {
        id.id
    }
}

impl<T> From<u64> for Id<T> {
    fn from(id: u64) -> Id<T> {
        Id {
            id,
            _marker: PhantomData,
        }
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple(std::any::type_name::<T>())
            .field(&self.id)
            .finish()
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
