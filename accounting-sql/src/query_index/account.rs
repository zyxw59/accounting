use accounting_core::{
    backend::id::Id,
    public::account::{Account, AccountQuery},
};
use sqlx::{Postgres, QueryBuilder};

use super::{Indexable, Singular, SqlIndexQueries, SqlTable, TableIndex, TableName, ToSqlQuery};

pub enum Query<'a> {
    Singular(Singular<'a, super::Query>),
}

impl SqlTable for Query<'_> {
    fn table(&self) -> TableName {
        match self {
            Self::Singular(query) => query.table(),
        }
    }
}

impl<'a> ToSqlQuery<'a> for Query<'a> {
    fn push_query(&self, builder: &mut QueryBuilder<'a, Postgres>, table_index: TableIndex) {
        match self {
            Self::Singular(query) => query.push_query(builder, table_index),
        }
    }
}

pub struct Index<'a> {
    singular: Singular<'a, super::Index>,
}

#[async_trait::async_trait]
impl<'a> SqlIndexQueries<'a, Account> for Index<'a> {
    async fn insert_index(
        &self,
        id: Id<Account>,
        transaction: &mut sqlx::Transaction<'a, Postgres>,
    ) -> sqlx::Result<()> {
        self.singular.insert_index(id, transaction).await?;
        Ok(())
    }

    async fn remove_index(
        id: Id<Account>,
        transaction: &mut sqlx::Transaction<'a, Postgres>,
    ) -> sqlx::Result<()> {
        Singular::remove_index(id, transaction).await?;
        Ok(())
    }

    async fn update_index(
        &self,
        id: Id<Account>,
        transaction: &mut sqlx::Transaction<'a, Postgres>,
    ) -> sqlx::Result<()> {
        self.singular.update_index(id, transaction).await?;
        Ok(())
    }
}

impl Indexable for Account {
    type IndexQuery<'a> = Query<'a>;
    type Index<'a> = Index<'a>;

    fn index(&self) -> Self::Index<'_> {
        Index {
            singular: Singular {
                name: Some(&self.name),
                description: Some(&self.description),
                ..Default::default()
            },
        }
    }

    fn transform_query(query: &AccountQuery) -> Self::IndexQuery<'_> {
        match query {
            AccountQuery::Name(name) => Query::Singular(Singular {
                name: Some(name.as_ref()),
                ..Default::default()
            }),
            AccountQuery::Description(description) => Query::Singular(Singular {
                description: Some(description.as_ref()),
                ..Default::default()
            }),
        }
    }
}
