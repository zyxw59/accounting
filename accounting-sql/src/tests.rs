use accounting_core::{
    backend::{Backend, id::WithId},
    error::Result,
    public::{
        account::AccountMetadata,
        amount::Amount,
        transaction::{Transaction, TransactionSplit},
    },
};

use crate::Connection;

#[sqlx::test]
async fn transactions(pool: sqlx::Pool<sqlx::Postgres>) -> Result<()> {
    let mut connection = Connection { pool };
    let bank = connection
        .create_account(AccountMetadata {
            name: "Bank".into(),
            description: "Bank account".into(),
        })
        .await?;
    let rent = connection
        .create_account(AccountMetadata {
            name: "Rent".into(),
            description: "Rent expenses".into(),
        })
        .await?;
    let groceries = connection
        .create_account(AccountMetadata {
            name: "Groceries".into(),
            description: "Groceries expenses".into(),
        })
        .await?;
    let supplies = connection
        .create_account(AccountMetadata {
            name: "Supplies".into(),
            description: "Supplies expenses".into(),
        })
        .await?;

    let transactions = [
        Transaction {
            date: time::macros::date!(2025 - 01 - 01),
            description: "January rent".into(),
            amounts: vec![
                TransactionSplit {
                    account: bank,
                    amount: Amount::new(-1000),
                    note: String::new(),
                },
                TransactionSplit {
                    account: rent,
                    amount: Amount::new(1000),
                    note: String::new(),
                },
            ],
        },
        Transaction {
            date: time::macros::date!(2025 - 01 - 02),
            description: "Grocery store".into(),
            amounts: vec![
                TransactionSplit {
                    account: bank,
                    amount: Amount::new(-150),
                    note: String::new(),
                },
                TransactionSplit {
                    account: groceries,
                    amount: Amount::new(150),
                    note: String::new(),
                },
            ],
        },
        Transaction {
            date: time::macros::date!(2025 - 01 - 04),
            description: "Grocery store".into(),
            amounts: vec![
                TransactionSplit {
                    account: bank,
                    amount: Amount::new(-30),
                    note: String::new(),
                },
                TransactionSplit {
                    account: groceries,
                    amount: Amount::new(20),
                    note: String::new(),
                },
                TransactionSplit {
                    account: supplies,
                    amount: Amount::new(10),
                    note: String::new(),
                },
            ],
        },
    ];

    let mut transactions_with_id = Vec::with_capacity(transactions.len());

    for object in transactions {
        let id = connection.create_transaction(object.clone()).await?;
        transactions_with_id.push(WithId { id, object });
    }
    let all_txs = connection.get_transactions().await?;
    pretty_assertions::assert_eq!(all_txs, transactions_with_id);

    let grocery_txs = connection.get_transactions_by_account(groceries).await?;
    pretty_assertions::assert_eq!(
        grocery_txs,
        transactions_with_id
            .into_iter()
            .filter(|t| t
                .object
                .amounts
                .iter()
                .any(|split| split.account == groceries))
            .collect::<Vec<_>>()
    );

    Ok(())
}
