use async_trait::async_trait;

use crate::{
    backend::{
        id::Id,
        query::{GroupQuery, Queryable},
        user::{ChangeGroup, Group, WithGroup},
        version::Versioned,
    },
    error::Result,
};

/// A collection of resources
#[async_trait]
pub trait Collection<T> {
    /// Create a new object
    async fn create(&mut self, object: WithGroup<T>) -> Result<Id<T>>;

    /// Get object with id
    async fn get(&self, id: Id<T>) -> Result<Option<WithGroup<Versioned<T>>>>;

    /// Attempt to apply an update to the object.
    ///
    /// If there are conflicting edits, this will fail with `Error::ConflictingEdit`
    async fn update(&mut self, object: Versioned<T>) -> Result<()>;

    /// Delete object with id
    async fn delete(&mut self, id: Id<T>) -> Result<()>;

    /// Move an object to a different group.
    async fn change_group(&mut self, id: Id<T>, new_group: Id<Group>) -> Result<()>
    where
        T: ChangeGroup;

    /// Count the number of objects matching all the queries.
    async fn query_count(&self, query: &[GroupQuery<T>]) -> Result<usize>
    where
        T: Queryable;
}
