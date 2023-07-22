//! Defines the core backend API

use async_trait::async_trait;

use crate::{
    error::{Error, Result},
    public::{account::Account, transaction::Transaction},
};

pub mod collection;
pub mod id;
pub mod query;
pub mod user;
pub mod version;

use collection::Collection;
use id::Id;
use query::{GroupQuery, Queryable};
use user::{AccessLevel, ChangeGroup, Group, User, WithGroup};
use version::Versioned;

pub struct Backend {
    current_user: Id<user::User>,
    users: Box<dyn Collection<User> + Send + Sync>,
    groups: Box<dyn Collection<Group> + Send + Sync>,
    accounts: Box<dyn Collection<Account> + Send + Sync>,
    transactions: Box<dyn Collection<Transaction> + Send + Sync>,
}

impl Backend {
    async fn get_group_permsissions(&self, group: Id<Group>) -> Result<AccessLevel> {
        Ok(self
            .groups
            .get(group)
            .await
            .transpose()
            .unwrap_or(Err(Error::NotFound))
            .map_err(|err| {
                log::error!("Unable to lookup {group:?}: {err}");
                Error::Unauthorized
            })?
            .object
            .object
            .permissions
            .get(self.current_user))
    }

    async fn get_group_of<T>(&self, id: Id<T>) -> Result<Id<Group>>
    where
        Self: HasCollection<T>,
    {
        self.get_collection()
            .get(id)
            .await?
            .ok_or(Error::NotFound)
            .map(|result| result.group)
    }
}

trait HasCollection<T> {
    fn get_collection(&self) -> &(dyn Collection<T> + Send + Sync);
    fn get_mut_collection(&mut self) -> &mut (dyn Collection<T> + Send + Sync);
}

macro_rules! impl_has_collection {
    ($($field:ident: $type:ty),* $(,)?) => {
        $(
        impl HasCollection<$type> for Backend {
            fn get_collection(&self) -> &(dyn Collection<$type> + Send + Sync) {
                &*self.$field
            }
            fn get_mut_collection(&mut self) -> &mut (dyn Collection<$type> + Send + Sync) {
                &mut *self.$field
            }
        }
        )*
    };
}

impl_has_collection! {
    users: User,
    groups: Group,
    accounts: Account,
    transactions: Transaction,
}

#[async_trait]
impl<T> Collection<T> for Backend
where
    Backend: HasCollection<T>,
    T: Send + 'static,
{
    /// Create a new object
    async fn create(&mut self, object: WithGroup<T>) -> Result<Id<T>> {
        if self.get_group_permsissions(object.group).await? < AccessLevel::Write {
            Err(Error::Unauthorized)
        } else {
            // TODO: validation
            self.get_mut_collection().create(object).await
        }
    }

    /// Get object with id
    async fn get(&self, id: Id<T>) -> Result<Option<WithGroup<Versioned<T>>>> {
        let maybe_object = self.get_collection().get(id).await?;
        if let Some(object) = maybe_object {
            if self.get_group_permsissions(object.group).await? < AccessLevel::Read {
                Err(Error::Unauthorized)
            } else {
                Ok(Some(object))
            }
        } else {
            Ok(None)
        }
    }

    /// Attempt to apply an update to the object.
    ///
    /// If there are conflicting edits, this will fail with `Error::ConflictingEdit`
    async fn update(&mut self, object: Versioned<T>) -> Result<()> {
        let group = self.get_group_of(object.id).await?;
        if self.get_group_permsissions(group).await? < AccessLevel::Write {
            Err(Error::Unauthorized)
        } else {
            // TODO: validation
            self.get_mut_collection().update(object).await
        }
    }

    /// Delete object with id
    async fn delete(&mut self, id: Id<T>) -> Result<()> {
        let group = self.get_group_of(id).await?;
        if self.get_group_permsissions(group).await? < AccessLevel::Write {
            Err(Error::Unauthorized)
        } else {
            // TODO: validation of back-references
            self.get_mut_collection().delete(id).await
        }
    }

    /// Move an object to a different group.
    async fn change_group(&mut self, id: Id<T>, new_group: Id<Group>) -> Result<()>
    where
        T: ChangeGroup,
    {
        let old_group = self.get_group_of(id).await?;
        if self.get_group_permsissions(old_group).await? < AccessLevel::Write
            || self.get_group_permsissions(new_group).await? < AccessLevel::Write
        {
            Err(Error::Unauthorized)
        } else {
            self.get_mut_collection().change_group(id, new_group).await
        }
    }

    async fn query_count(&self, query: &[GroupQuery<T>]) -> Result<usize>
    where
        T: Queryable,
    {
        // TODO: filter by permissions
        self.get_collection().query_count(query).await
    }
}
