use itertools::Itertools;
use serde::{Deserialize, Serialize};
use time::Date;

use crate::{
    backend::id::Id,
    public::{account::Account, amount::Amount},
};

#[derive(Clone, Debug, Eq, Deserialize, Serialize)]
pub struct Transaction {
    #[serde(with = "crate::serde::date")]
    pub date: Date,
    pub description: String,
    pub amounts: Vec<TransactionSplit>,
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.date == other.date
            && self.description == other.description
            && self.amounts.len() == other.amounts.len()
            && self.amounts.iter().counts() == other.amounts.iter().counts()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow, sqlx::Type))]
pub struct TransactionSplit {
    pub account: Id<Account>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    pub amount: Amount,
}
