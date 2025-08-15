use serde::{Deserialize, Serialize};

use crate::public::amount::Amount;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Account {
    pub metadata: AccountMetadata,
    pub current_balance: Amount,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccountMetadata {
    pub name: String,
    pub description: String,
}
