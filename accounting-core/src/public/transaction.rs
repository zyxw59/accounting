use serde::{Deserialize, Serialize, Serializer};

use crate::{
    backend::{
        id::Id,
        query::{Query, QueryElement, Queryable, SerializedQuery, SimpleQuery},
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
    type Query = TransactionQuery;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TransactionQuery {
    Date(SimpleQuery<Date>),
    Description(SimpleQuery<String>),
    // { "amounts.0": id }
    /// The transaction involves the specified account
    Account(Id<Account>),
    // { "amounts": { "$elemMatch" : {"0": id, "1": query} }
    AccountAmount(Id<Account>, SimpleQuery<Amount>),
}

impl Query<Transaction> for TransactionQuery {
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

    fn serialize_query<F, S>(&self, factory: &F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        match self {
            Self::Date(query) => Ok(SerializedQuery::from_path_and_query(
                "date",
                query.serialize_value(factory)?.into(),
            )),
            Self::Description(query) => Ok(SerializedQuery::from_path_and_query(
                "description",
                query.serialize_value(factory)?.into(),
            )),
            Self::Account(account) => Ok(SerializedQuery::from_path_and_query(
                "amounts",
                QueryElement::ElemMatch(SerializedQuery::from_path_and_query(
                    "0",
                    SimpleQuery::eq(account.serialize(factory())?).into(),
                )),
            )),
            Self::AccountAmount(account, amount_query) => Ok(SerializedQuery::from_path_and_query(
                "amounts",
                QueryElement::ElemMatch(SerializedQuery::from_path_queries([
                    ("0", SimpleQuery::eq(account.serialize(factory())?).into()),
                    ("1", amount_query.serialize_value(factory)?.into()),
                ])),
            )),
        }
    }
}
