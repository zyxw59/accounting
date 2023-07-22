use serde::{Deserialize, Serialize};

use crate::backend::query::{Query, Queryable, RawQuery, SimpleQuery};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Account {
    pub name: String,
    pub description: String,
}

impl Queryable for Account {
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

    fn as_raw_query(&self) -> RawQuery {
        match self {
            Self::Name(query) => RawQuery::simple("name", query.to_value_query()),
            Self::Description(query) => RawQuery::simple("description", query.to_value_query()),
        }
    }
}
