use serde::{Deserialize, Serialize, Serializer};

use crate::{
    backend::{
        id::Id,
        query::{QueryParameter, Queryable, SerializedQuery, SimpleQuery},
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
                    SerializedQuery::from_value(account, &factory)?,
                )
                .and(SerializedQuery::from_path_and_query(
                    "1",
                    amount_query.serialize_query(factory)?,
                )),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TransactionQuery;
    use crate::{
        backend::{
            id::Id,
            query::{Comparator, QueryParameter, SimpleQuery},
        },
        public::amount::Amount,
    };

    #[test]
    fn serialize_query() {
        let query = TransactionQuery::AccountAmount(
            Id::new(1234),
            SimpleQuery([(Comparator::Greater, Amount::ZERO)].into_iter().collect()),
        );

        let serialized_query = query
            .serialize_query(|| serde_json::value::Serializer)
            .unwrap();
        let serialized = serde_json::to_value(&serialized_query).unwrap();

        let expected = serde_json::json!({
            "amounts": {
                "$and": [
                    { "0": 1234 },
                    { "1": { "$gt": "0" } },
                ],
            }
        });
        pretty_assertions::assert_eq!(format!("{serialized:#}"), format!("{expected:#}"));
    }
}
