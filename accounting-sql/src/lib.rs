use accounting_core::{
    backend::{
        collection::Collection,
        id::Id,
        user::{ChangeGroup, Group, WithGroup, WithGroupQuery},
        version::{Version, Versioned},
    },
    error::{Error, Result},
};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json, Postgres, QueryBuilder};

mod query_index;

use query_index::{Indexable, SqlIndexQueries};

pub struct SqlCollection {
    connection_pool: sqlx::Pool<Postgres>,
}

impl SqlCollection {
    pub async fn query_count<T>(&self, queries: &[WithGroupQuery<T>]) -> sqlx::Result<usize>
    where
        T: Indexable,
    {
        let mut qb = query_index::query::<T>("COUNT(*)", queries);
        let count: i64 = qb
            .build_query_scalar()
            .fetch_one(&self.connection_pool)
            .await?;
        Ok(count as usize)
    }

    async fn insert_object_indexes<T: Indexable>(&mut self, object: &T, id: Id<T>) -> Result<()> {
        let index = object.index();
        let mut transaction = self.connection_pool.begin().await.map_err(Error::backend)?;
        index
            .insert_index(id, &mut transaction)
            .await
            .map_err(Error::backend)?;
        transaction.commit().await.map_err(Error::backend)?;
        Ok(())
    }

    async fn update_object_indexes<T: Indexable>(&mut self, object: &T, id: Id<T>) -> Result<()> {
        let index = object.index();
        let mut transaction = self.connection_pool.begin().await.map_err(Error::backend)?;
        index
            .update_index(id, &mut transaction)
            .await
            .map_err(Error::backend)?;
        transaction.commit().await.map_err(Error::backend)?;
        Ok(())
    }

    async fn remove_object_indexes<T: Indexable>(&mut self, id: Id<T>) -> Result<()> {
        let mut transaction = self.connection_pool.begin().await.map_err(Error::backend)?;
        T::Index::remove_index(id, &mut transaction)
            .await
            .map_err(Error::backend)?;
        transaction.commit().await.map_err(Error::backend)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl<T> Collection<T> for SqlCollection
where
    T: Indexable + Serialize + for<'de> Deserialize<'de> + Send + Sync + Unpin + 'static,
{
    async fn create(&mut self, WithGroup { object, group }: WithGroup<T>) -> Result<Id<T>> {
        let id = Id::new_random();
        let version = Version::new_random();
        let mut qb =
            QueryBuilder::new("INSERT INTO resources(id, type, version, group_, resource) (");
        let mut values = qb.separated(",");
        values.push_bind(id);
        values.push_bind(T::TYPE_NAME);
        values.push_bind(version);
        values.push_bind(group);
        values.push_bind(Json(&object));
        qb.push(")");
        qb.build()
            .execute(&self.connection_pool)
            .await
            .map_err(Error::backend)?;
        self.insert_object_indexes(&object, id).await?;
        Ok(id)
    }

    async fn get(&self, id: Id<T>) -> Result<Option<WithGroup<Versioned<T>>>> {
        let mut qb =
            QueryBuilder::new("SELECT id, version, group_, resource FROM resources WHERE id = ");
        qb.push_bind(id);
        qb.build_query_as::<ResourceTableEntry<T>>()
            .fetch_optional(&self.connection_pool)
            .await
            .map(|opt| opt.map(Into::into))
            .map_err(Error::backend)
    }

    async fn delete(&mut self, id: Id<T>) -> Result<()> {
        let mut qb = QueryBuilder::new("DELETE from resources WHERE id = ");
        qb.push_bind(id);
        qb.build()
            .execute(&self.connection_pool)
            .await
            .map_err(Error::backend)?;
        self.remove_object_indexes(id).await?;
        Ok(())
    }

    async fn update(
        &mut self,
        Versioned {
            object,
            id,
            version,
        }: Versioned<T>,
    ) -> Result<()> {
        let new_version = Version::new_random();
        let mut qb = QueryBuilder::new("UPDATE resources SET (version, resource) = (");
        let mut values = qb.separated(",");
        values.push_bind(new_version);
        values.push_bind(Json(&object));
        qb.push(") WHERE id = ");
        qb.push_bind(id);
        qb.push(" AND version = ");
        qb.push_bind(version);
        let res = qb
            .build()
            .execute(&self.connection_pool)
            .await
            .map_err(Error::backend)?;
        if res.rows_affected() < 1 {
            let mut qb = QueryBuilder::new("SELECT version FROM resources WHERE id = ");
            qb.push_bind(id);
            if qb
                .build()
                .fetch_optional(&self.connection_pool)
                .await
                .map_err(Error::backend)?
                .is_some()
            {
                return Err(Error::ConflictingEdit);
            } else {
                return Err(Error::NotFound);
            }
        }
        self.update_object_indexes(&object, id).await?;
        Ok(())
    }

    async fn change_group(&mut self, id: Id<T>, new_group: Id<Group>) -> Result<()>
    where
        T: ChangeGroup,
    {
        let new_version = Version::new_random();
        let mut qb = QueryBuilder::new("UPDATE resources SET (version, group_) = (");
        let mut values = qb.separated(",");
        values.push_bind(new_version);
        values.push_bind(new_group);
        qb.push(") WHERE id = ");
        qb.push_bind(id);
        qb.build()
            .execute(&self.connection_pool)
            .await
            .map_err(Error::backend)?;
        Ok(())
    }

    async fn query_count(&self, query: &[WithGroupQuery<T>]) -> Result<usize> {
        self.query_count(query).await.map_err(Error::backend)
    }
}

#[derive(sqlx::FromRow)]
struct ResourceTableEntry<T> {
    id: Id<T>,
    #[sqlx(rename = "group_")]
    group: Id<Group>,
    version: Version,
    #[sqlx(json)]
    resource: T,
}

impl<T> From<ResourceTableEntry<T>> for WithGroup<Versioned<T>> {
    fn from(
        ResourceTableEntry {
            id,
            group,
            version,
            resource,
        }: ResourceTableEntry<T>,
    ) -> Self {
        WithGroup {
            group,
            object: Versioned {
                id,
                version,
                object: resource,
            },
        }
    }
}
