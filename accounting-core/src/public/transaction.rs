use std::borrow::Cow;

use serde::{Deserialize, Serialize, Serializer};
use time::Date;

use crate::{
    backend::{
        id::Id,
        query::{Comparator, QueryParameter, Queryable, SimpleQuery},
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

    fn path(&self) -> Cow<[&'static str]> {
        match self {
            Self::Date(_) => Cow::Borrowed(&["date"]),
            Self::Description(_) => Cow::Borrowed(&["description"]),
        }
    }

    fn comparator(&self) -> Comparator {
        match self {
            Self::Date(query) => query.comparator(),
            Self::Description(query) => query.comparator(),
        }
    }

    fn serialize_value<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Date(query) => query.serialize_value(serializer),
            Self::Description(query) => query.serialize_value(serializer),
        }
    }
}
