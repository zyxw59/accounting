use serde::{Deserialize, Serialize};

use crate::backend::query::{Query, Queryable, SimpleQuery};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Account {
    pub name: String,
    pub description: String,
}

impl Queryable for Account {
    const TYPE_NAME: &'static str = "account";

    type Query = AccountQuery;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AccountQuery {
    Name(SimpleQuery<String>),
    Description(SimpleQuery<String>),
}

impl Query<Account> for AccountQuery {
    fn matches(&self, account: &Account) -> bool {
        match self {
            Self::Name(query) => query.matches(&account.name),
            Self::Description(query) => query.matches(&account.description),
        }
    }
}
