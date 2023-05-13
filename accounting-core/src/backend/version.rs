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

impl From<Version> for bson::Bson {
    fn from(version: Version) -> Self {
        bson::Bson::Int64(version.0 as i64)
    }
}

impl Distribution<Version> for Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Version {
        Version(rng.next_u64())
    }
}
