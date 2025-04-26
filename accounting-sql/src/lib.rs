use accounting_core::{
    backend::{
        id::{Id, WithId},
        Backend,
    },
    error::Result,
    public::{
        account::Account,
        amount::Amount,
        transaction::{Transaction, TransactionSplit},
    },
};
use itertools::Itertools;
use time::Date;

pub struct Connection {
    pool: sqlx::Pool<sqlx::Postgres>,
}

impl Backend for Connection {
    async fn create_transaction(&mut self, transaction: Transaction) -> Result<Id<Transaction>> {
        let mut tx = self.pool.begin().await.expect("TODO");
        let id = Id::new_random();
        sqlx::query!(
            "INSERT INTO transactions(id, description, date_) VALUES ($1, $2, $3)",
            id as Id<Transaction>,
            transaction.description,
            transaction.date,
        )
        .execute(&mut *tx)
        .await
        .expect("TODO");
        let (accounts, notes, amounts) = transaction
            .amounts
            .into_iter()
            .map(|split| (split.account, split.note, split.amount))
            .multiunzip::<(Vec<_>, Vec<_>, Vec<_>)>();
        sqlx::query!(
            "INSERT INTO splits(transaction, account, note, amount)
            SELECT $1, * FROM UNNEST($2::bigint[], $3::text[], $4::bigint[])",
            id as Id<_>,
            &accounts as &[Id<_>],
            &notes,
            &amounts as &[Amount],
        )
        .execute(&mut *tx)
        .await
        .expect("TODO");
        sqlx::query!(
            "UPDATE accounts set (balance) = (
                SELECT accounts.balance + SUM(x.change)
                FROM unnest($1::bigint[], $2::bigint[]) as x(account, change)
                WHERE x.account = accounts.id
            )",
            &accounts as &[Id<_>],
            &amounts as &[Amount],
        )
        .execute(&mut *tx)
        .await
        .expect("TODO");
        tx.commit().await.expect("TODO");
        Ok(id)
    }

    async fn get_transactions(&self) -> Result<Vec<WithId<Transaction>>> {
        Ok(sqlx::query!(
            r#"SELECT
                id as "id: Id<Transaction>", description, date_,
                CASE
                    WHEN transaction IS NULL
                    THEN NULL
                    ELSE ARRAY_AGG((account, note, amount))
                END as "splits: Vec<TransactionSplit>"
            FROM transactions
            LEFT JOIN splits ON id = transaction
            GROUP BY id, transaction"#,
        )
        .fetch_all(&self.pool)
        .await
        .expect("TODO")
        .into_iter()
        .map(|record| WithId {
            id: record.id,
            object: Transaction {
                description: record.description,
                date: record.date_,
                amounts: record.splits.unwrap_or_default(),
            },
        })
        .collect())
    }

    async fn get_transactions_by_account(
        &self,
        _account: Id<Account>,
    ) -> Result<Vec<WithId<Transaction>>> {
        todo!();
    }

    async fn get_all_accounts(&self) -> Result<Vec<WithId<Account>>> {
        todo!();
    }

    async fn get_account(
        &self,
        _account: Id<Account>,
        _as_of: Option<Date>,
    ) -> Result<WithId<Account>> {
        todo!();
    }
}
