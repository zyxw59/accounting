use serde::{Deserialize, Serialize};
use time::Date;

use crate::{
    backend::id::Id,
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
