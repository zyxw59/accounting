use serde::{Deserialize, Serialize};
use time::Date;

use crate::{
    backend::id::Id,
    public::{account::Account, amount::Amount},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Transaction {
    #[serde(with = "crate::serde::date")]
    pub date: Date,
    pub description: String,
    pub amounts: Vec<TransactionSplit>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransactionSplit {
    pub account: Id<Account>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    pub amount: Amount,
}
