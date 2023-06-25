use std::borrow::Cow;

use serde::{Deserialize, Serialize, Serializer};

use crate::backend::query::{Comparator, QueryParameter, Queryable, SimpleQuery};

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

impl QueryParameter<Account> for AccountQuery {
    fn matches(&self, account: &Account) -> bool {
        match self {
            Self::Name(query) => query.matches(&account.name),
            Self::Description(query) => query.matches(&account.description),
        }
    }

    fn path(&self) -> Cow<[&'static str]> {
        match self {
            Self::Name(_) => Cow::Borrowed(&["name"]),
            Self::Description(_) => Cow::Borrowed(&["description"]),
        }
    }

    fn comparator(&self) -> Comparator {
        match self {
            Self::Name(query) => query.comparator(),
            Self::Description(query) => query.comparator(),
        }
    }

    fn serialize_value<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Name(query) => query.serialize_value(serializer),
            Self::Description(query) => query.serialize_value(serializer),
        }
    }
}
