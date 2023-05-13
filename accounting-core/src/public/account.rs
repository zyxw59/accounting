use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Account {
    pub name: String,
    pub description: String,
}
