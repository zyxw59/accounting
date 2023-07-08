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
pub struct TransactionQuery {
    pub date: Option<SimpleQuery<Date>>,
    pub description: Option<SimpleQuery<String>>,
    /// The transaction involves at least one of the specified accounts
    pub account: Option<Vec<Id<Account>>>,
    pub account_amount: Option<(Id<Account>, SimpleQuery<Amount>)>,
}

impl Query<Transaction> for TransactionQuery {
    fn matches(&self, transaction: &Transaction) -> bool {
        self.date
            .as_ref()
            .map(|q| q.matches(&transaction.date))
            .unwrap_or(true)
            && self
                .description
                .as_ref()
                .map(|q| q.matches(&transaction.description))
                .unwrap_or(true)
            && self
                .account
                .as_ref()
                .map(|accounts| {
                    accounts
                        .iter()
                        .any(|account| transaction.amounts.contains_key(account))
                })
                .unwrap_or(true)
            && self
                .account_amount
                .as_ref()
                .map(|(account, amount_query)| {
                    amount_query.matches(transaction.amounts.get(account).unwrap_or(&Amount::ZERO))
                })
                .unwrap_or(true)
    }

    fn serialize_query<F, S>(&self, factory: &F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        let date_query = self
            .date
            .as_ref()
            .map(|query| {
                Ok(SerializedQuery::from_path_and_query(
                    "date",
                    query.serialize_value(factory)?.into(),
                ))
            })
            .transpose()?;
        let description_query = self
            .description
            .as_ref()
            .map(|query| {
                Ok(SerializedQuery::from_path_and_query(
                    "description",
                    query.serialize_value(factory)?.into(),
                ))
            })
            .transpose()?;
        let account_query = self
            .account
            .as_ref()
            .map(|accounts| {
                Ok(SerializedQuery::from_path_and_query(
                    "amounts",
                    QueryElement::ElemMatch(SerializedQuery::from_path_and_query(
                        "0",
                        QueryElement::In(
                            accounts
                                .iter()
                                .map(|account| account.serialize(factory()))
                                .collect::<Result<_, _>>()?,
                        ),
                    )),
                ))
            })
            .transpose()?;
        let amount_query = self
            .account_amount
            .as_ref()
            .map(|(account, amount)| {
                Ok(SerializedQuery::from_path_and_query(
                    "amounts",
                    QueryElement::ElemMatch(SerializedQuery::from_path_queries([
                        ("0", SimpleQuery::eq(account.serialize(factory())?).into()),
                        ("1", amount.serialize_value(factory)?.into()),
                    ])),
                ))
            })
            .transpose()?;

        Ok(SerializedQuery::all_opt([
            date_query,
            description_query,
            account_query,
            amount_query,
        ]))
    }
}
