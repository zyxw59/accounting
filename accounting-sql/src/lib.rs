use accounting_core::backend::query::{Query, Queryable};
use sqlx::Postgres;

mod query;
mod index;

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
