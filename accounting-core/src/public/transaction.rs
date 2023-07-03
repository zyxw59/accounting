use serde::{Deserialize, Serialize, Serializer};
use time::Date;

use crate::{
    backend::{
        id::Id,
        query::{QueryParameter, Queryable, SerializedQuery, SimpleQuery},
    },
    map::Map,
    public::{account::Account, amount::Amount},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Transaction {
    #[serde(with = "crate::serde::date")]
    pub date: Date,
    pub description: String,
    pub amounts: Map<Id<Account>, Amount>,
}

impl Queryable for Transaction {
    type Query = TransactionQuery;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TransactionQuery {
    // TODO: serialize correctly
    Date(SimpleQuery<Date>),
    Description(SimpleQuery<String>),
    /// The transaction involves the specified account
    Account(Id<Account>),
    AccountAmount(Id<Account>, SimpleQuery<Amount>),
}

impl QueryParameter<Transaction> for TransactionQuery {
    fn matches(&self, transaction: &Transaction) -> bool {
        match self {
            Self::Date(query) => query.matches(&transaction.date),
            Self::Description(query) => query.matches(&transaction.description),
            Self::Account(account) => transaction.amounts.contains_key(account),
            Self::AccountAmount(account, amount_query) => {
                amount_query.matches(transaction.amounts.get(account).unwrap_or(&Amount::ZERO))
            }
        }
    }

    fn serialize_query<F, S>(&self, factory: F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        match self {
            Self::Date(query) => Ok(SerializedQuery::from_path_and_query(
                "date",
                query.serialize_query(factory)?,
            )),
            Self::Description(query) => Ok(SerializedQuery::from_path_and_query(
                "description",
                query.serialize_query(factory)?,
            )),
            Self::Account(account) => Ok(SerializedQuery::from_path_and_query(
                ["amounts", "0"],
                SerializedQuery::from_value(account, factory)?,
            )),
            Self::AccountAmount(account, amount_query) => Ok(SerializedQuery::from_path_and_query(
                "amounts",
                SerializedQuery::from_path_and_query(
                    "0",
                    SerializedQuery::from_value(account, &factory)?.and(
                        SerializedQuery::from_path_and_query(
                            "1",
                            amount_query.serialize_query(factory)?,
                        ),
                    ),
                ),
            )),
        }
    }
}
