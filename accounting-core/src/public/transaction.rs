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
    // TODO: amount
}

impl QueryParameter<Transaction> for TransactionQuery {
    fn matches(&self, transaction: &Transaction) -> bool {
        match self {
            Self::Date(query) => query.matches(&transaction.date),
            Self::Description(query) => query.matches(&transaction.description),
        }
    }

    fn serialize_query<F, S>(&self, factory: F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        match self {
            Self::Date(query) => Ok(SerializedQuery::from_path_and_query(
                &["date"],
                query.serialize_query(factory)?,
            )),
            Self::Description(query) => Ok(SerializedQuery::from_path_and_query(
                &["description"],
                query.serialize_query(factory)?,
            )),
        }
    }
}
