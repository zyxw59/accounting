use serde::{Deserialize, Serialize};

use crate::{
    backend::{
        id::Id,
        query::{Index, Query, Queryable, RawQuery, SimpleQuery, ToValue},
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

    fn indices(&self) -> Vec<Index> {
        let mut indices = Vec::with_capacity(self.amounts.len() + 2);
        indices.push(Index::simple("date", self.date));
        indices.push(Index::simple("description", self.description.clone()));
        indices.extend(self.amounts.iter().map(|(account, amount)| {
            Index::complex(
                TransactionQuery::ACCOUNT_AMOUNT_PARAMETER,
                [("account", (*account).into()), ("amount", (*amount).into())],
            )
        }));
        indices
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TransactionQuery {
    Date(SimpleQuery<Date>),
    Description(SimpleQuery<String>),
    /// The transaction involves at least one of the specified accounts
    Account(Vec<Id<Account>>),
    AccountAmount(Id<Account>, SimpleQuery<Amount>),
}

impl TransactionQuery {
    const ACCOUNT_AMOUNT_PARAMETER: &'static str = "account_amount";
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

    fn as_raw_query(&self) -> RawQuery {
        match self {
            Self::Date(query) => RawQuery::simple("date", query.to_value_query()),
            Self::Description(query) => RawQuery::simple("description", query.to_value_query()),
            Self::Account(accounts) => RawQuery::complex(
                Self::ACCOUNT_AMOUNT_PARAMETER,
                [(
                    "account",
                    SimpleQuery {
                        in_: Some(accounts.iter().map(ToValue::to_value).collect()),
                        ..Default::default()
                    }
                    .into(),
                )],
            ),
            Self::AccountAmount(account, amount_query) => RawQuery::complex(
                Self::ACCOUNT_AMOUNT_PARAMETER,
                [
                    (
                        "account",
                        SimpleQuery {
                            eq: Some(account.to_value()),
                            ..Default::default()
                        }
                        .into(),
                    ),
                    ("amount", amount_query.to_value_query()),
                ],
            ),
        }
    }
}
