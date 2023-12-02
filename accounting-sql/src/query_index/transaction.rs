use accounting_core::{
    backend::{id::Id, query::SimpleQueryRef},
    public::{
        account::Account,
        amount::Amount,
        transaction::{Transaction, TransactionQuery},
    },
};
use sqlx::{Postgres, QueryBuilder};

use super::{
    index_values::{push_parameter, IndexValues, PushParameter},
    push_simple_query, Indexable, QueryOrIndex, Singular, SqlIndexQueries, SqlTable, TableIndex,
    TableName, ToSqlQuery,
};

pub enum Query<'a> {
    Singular(Singular<'a, super::Query>),
    AccountAmount(AccountAmount<'a, super::Query>),
}

impl SqlTable for Query<'_> {
    fn table(&self) -> TableName {
        match self {
            Self::Singular(query) => query.table(),
            Self::AccountAmount(query) => query.table(),
        }
    }
}

impl<'a> ToSqlQuery<'a> for Query<'a> {
    fn push_query(&self, builder: &mut QueryBuilder<'a, Postgres>, table_index: TableIndex) {
        match self {
            Self::Singular(query) => query.push_query(builder, table_index),
            Self::AccountAmount(query) => query.push_query(builder, table_index),
        }
    }
}

pub struct Index<'a> {
    singular: Singular<'a, super::Index>,
    account_amount: Vec<AccountAmount<'a, super::Index>>,
}

#[async_trait::async_trait]
impl<'a> SqlIndexQueries<'a, Transaction> for Index<'a> {
    async fn insert_index(
        &self,
        id: Id<Transaction>,
        transaction: &mut sqlx::Transaction<'a, Postgres>,
    ) -> sqlx::Result<()> {
        self.singular.insert_index(id, transaction).await?;
        self.account_amount.insert_index(id, transaction).await?;
        Ok(())
    }

    async fn remove_index(
        id: Id<Transaction>,
        transaction: &mut sqlx::Transaction<'a, Postgres>,
    ) -> sqlx::Result<()> {
        Singular::remove_index(id, transaction).await?;
        AccountAmount::remove_index(id, transaction).await?;
        Ok(())
    }

    async fn update_index(
        &self,
        id: Id<Transaction>,
        transaction: &mut sqlx::Transaction<'a, Postgres>,
    ) -> sqlx::Result<()> {
        self.singular.update_index(id, transaction).await?;
        self.account_amount.update_index(id, transaction).await?;
        Ok(())
    }
}

pub struct AccountAmount<'a, T: QueryOrIndex> {
    account: T::Value<'a, Id<Account>>,
    amount: T::Value<'a, Amount>,
}

impl<T: QueryOrIndex> AccountAmount<'_, T> {
    const ACCOUNT: &str = "account";
    const AMOUNT: &str = "amount";
}

impl<T: QueryOrIndex> SqlTable for AccountAmount<'_, T> {
    fn table(&self) -> TableName {
        TableName::ACCOUNT_AMOUNT
    }
}

impl<'a> ToSqlQuery<'a> for AccountAmount<'a, super::Query> {
    fn push_query(&self, builder: &mut QueryBuilder<'a, Postgres>, table_index: TableIndex) {
        let Self { account, amount } = self;
        push_simple_query(table_index, Self::ACCOUNT, *account, builder);
        push_simple_query(table_index, Self::AMOUNT, *amount, builder);
    }
}

impl<'a> IndexValues<'a> for AccountAmount<'a, super::Index> {
    type Array<T> = [T; 2];
    const COLUMNS: Self::Array<&'static str> = [Self::ACCOUNT, Self::AMOUNT];
    const PARAMETERS: Self::Array<PushParameter<'a, Self>> =
        [push_parameter!(this.account), push_parameter!(this.amount)];
    const TABLE: TableName = TableName::ACCOUNT_AMOUNT;
}

impl Indexable for Transaction {
    type IndexQuery<'a> = Query<'a>;
    type Index<'a> = Index<'a>;

    fn index(&self) -> Self::Index<'_> {
        Index {
            singular: Singular {
                date: Some(&self.date),
                description: Some(&self.description),
                ..Default::default()
            },
            account_amount: self
                .amounts
                .iter()
                .map(|(account, amount)| AccountAmount { account, amount })
                .collect(),
        }
    }

    fn transform_query(query: &TransactionQuery) -> Self::IndexQuery<'_> {
        match query {
            TransactionQuery::Date(date) => Query::Singular(Singular {
                date: Some(date.as_ref()),
                ..Default::default()
            }),
            TransactionQuery::Description(description) => Query::Singular(Singular {
                description: Some(description.as_ref()),
                ..Default::default()
            }),
            TransactionQuery::Account(accounts) => Query::AccountAmount(AccountAmount {
                account: SimpleQueryRef::in_(accounts),
                amount: Default::default(),
            }),
            TransactionQuery::AccountAmount(account, amount) => {
                Query::AccountAmount(AccountAmount {
                    account: SimpleQueryRef::eq(account),
                    amount: amount.as_ref(),
                })
            }
        }
    }
}
