use accounting_core::{
    backend::{
        collection::Collection,
        id::{Id, WithId},
        query::{Query, Queryable},
        user::{ChangeGroup, Group, WithGroup, WithGroupQuery},
        version::{Version, Versioned},
    },
    error::{Error, Result},
};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json, Postgres, QueryBuilder};

mod index;
mod query;

use index::Indexable;
use query::ToSqlQuery;

pub struct SqlCollection {
    connection_pool: sqlx::Pool<Postgres>,
}

impl SqlCollection {
    pub async fn query_count<T, Q>(&self, queries: &[Q]) -> sqlx::Result<usize>
    where
        T: Queryable,
        Q: Query<T> + ToSqlQuery,
    {
        let mut qb = query::query("COUNT(*)", queries, T::TYPE_NAME);
        let count: i64 = qb
            .build_query_scalar()
            .fetch_one(&self.connection_pool)
            .await?;
        Ok(count as usize)
    }
}

#[async_trait::async_trait]
impl<T> Collection<T> for SqlCollection
where
    T: Queryable + Indexable + Serialize + for<'de> Deserialize<'de> + Send + Sync + Unpin + 'static,
    T::Query: ToSqlQuery,
{
    async fn create(&mut self, object: WithGroup<T>) -> Result<Id<T>> {
        let id = Id::new_random();
        let version = Version::new_random();
        let versioned = Versioned {
            id,
            version,
            object,
        };
        let mut qb = QueryBuilder::new("INSERT INTO resources(id, type, version, resource) (");
        let mut values = qb.separated(",");
        values.push_bind(versioned.id);
        values.push_bind(T::TYPE_NAME);
        values.push_bind(versioned.version);
        values.push_bind(Json(&versioned));
        qb.push(")");
        qb.build()
            .execute(&self.connection_pool)
            .await
            .map_err(Error::backend)?;
        for mut query in T::index(&WithId {
            id,
            object: versioned.object,
        }) {
            query
                .build()
                .execute(&self.connection_pool)
                .await
                .map_err(Error::backend)?;
        }
        Ok(id.transmute())
    }

    async fn get(&self, id: Id<T>) -> Result<Option<WithGroup<Versioned<T>>>> {
        let mut qb = QueryBuilder::new("SELECT resource FROM resources WHERE id = ");
        qb.push_bind(id);
        qb.build_query_scalar()
            .fetch_optional(&self.connection_pool)
            .await
            .map(|opt| opt.map(|Json(resource)| resource))
            .map_err(Error::backend)
    }

    async fn delete(&mut self, id: Id<T>) -> Result<()> {
        let mut qb = QueryBuilder::new("DELETE from resources WHERE id = ");
        qb.push_bind(id);
        qb.build()
            .execute(&self.connection_pool)
            .await
            .map_err(Error::backend)?;
        Ok(())
    }

    async fn update(&mut self, object: Versioned<T>) -> Result<()> {
        let new_version = Version::new_random();
        let mut qb = QueryBuilder::new("UPDATE resources SET (version, resource) = (");
        let mut values = qb.separated(",");
        values.push_bind(new_version);
        values.push_bind(Json(&object));
        qb.push(") WHERE id = ");
        qb.push_bind(object.id);
        qb.push(" AND version = ");
        qb.push_bind(object.version);
        let res = qb
            .build()
            .execute(&self.connection_pool)
            .await
            .map_err(Error::backend)?;
        if res.rows_affected() < 1 {
            let mut qb = QueryBuilder::new("SELECT version FROM resources WHERE id = ");
            qb.push_bind(object.id);
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
        // TODO: reindex
        Ok(())
    }

    async fn change_group(&mut self, id: Id<T>, new_group: Id<Group>) -> Result<()>
    where
        T: ChangeGroup,
    {
        todo!();
    }

    async fn query_count(&self, query: &[WithGroupQuery<T>]) -> Result<usize> {
        todo!();
    }
}
