use serde::{Deserialize, Serialize};

use crate::{
    backend::{
        id::Id,
        query::{Query, Queryable, SimpleQuery},
    },
    date::Date,
    map::Map,
    public::{account::Account, amount::Amount},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Transaction {
    pub date: Date,
    pub description: String,
    pub amounts: Map<Id<Account>, Amount>,
}

impl Queryable for Transaction {
    const TYPE_NAME: &'static str = "transaction";

    type Query = TransactionQuery;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TransactionQuery {
    Date(SimpleQuery<Date>),
    Description(SimpleQuery<String>),
    /// The transaction involves at least one of the specified accounts
    Account(Vec<Id<Account>>),
    AccountAmount(Id<Account>, SimpleQuery<Amount>),
}

impl Query<Transaction> for TransactionQuery {
    fn matches(&self, transaction: &Transaction) -> bool {
        match self {
            Self::Date(query) => query.matches(&transaction.date),
            Self::Description(query) => query.matches(&transaction.description),
            Self::Account(accounts) => accounts
                .iter()
                .any(|account| transaction.amounts.contains_key(account)),
            Self::AccountAmount(account, amount_query) => {
                amount_query.matches(transaction.amounts.get(account).unwrap_or(&Amount::ZERO))
            }
        }
    }
}
