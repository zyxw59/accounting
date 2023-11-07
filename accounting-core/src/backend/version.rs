use rand::distributions::{Distribution, Standard};
use serde::{Deserialize, Serialize};

use crate::backend::id::Id;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Versioned<T> {
    #[serde(rename = "_id")]
    pub id: Id<T>,
    #[serde(rename = "_version")]
    pub version: Version,
    #[serde(flatten)]
    pub object: T,
}

/// An opaque identifier for a version of a document, to detect conflicting edits.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Version(u64);

impl Version {
    /// Generate a new random `Version`
    pub fn new_random() -> Self {
        rand::random()
    }
}

impl Distribution<Version> for Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Version {
        Version(rng.next_u64())
    }
}

#[cfg(feature = "sqlx")]
impl<DB> sqlx::Type<DB> for Version
where
    DB: sqlx::Database,
    i64: sqlx::Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <i64 as sqlx::Type<DB>>::type_info()
    }
}

#[cfg(feature = "sqlx-postgres")]
impl sqlx::postgres::PgHasArrayType for Version {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        <i64 as sqlx::postgres::PgHasArrayType>::array_type_info()
    }
}

#[cfg(feature = "sqlx")]
impl<'q, DB> sqlx::Encode<'q, DB> for Version
where
    DB: sqlx::Database,
    i64: sqlx::Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        <i64 as sqlx::Encode<'q, DB>>::encode_by_ref(&(self.0 as i64), buf)
    }
}

#[cfg(feature = "sqlx")]
impl<'r, DB> sqlx::Decode<'r, DB> for Version
where
    DB: sqlx::Database,
    i64: sqlx::Decode<'r, DB>,
{
    fn decode(
        value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        <i64 as sqlx::Decode<'r, DB>>::decode(value).map(|version| Self(version as _))
    }
}
