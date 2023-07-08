use serde::{Deserialize, Serialize, Serializer};

use crate::backend::query::{Query, Queryable, SerializedQuery, SimpleQuery};

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

    fn serialize_query<F, S>(&self, factory: &F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        match self {
            Self::Name(query) => Ok(SerializedQuery::from_path_and_query(
                "name",
                query.serialize_value(factory)?.into(),
            )),
            Self::Description(query) => Ok(SerializedQuery::from_path_and_query(
                "description",
                query.serialize_value(factory)?.into(),
            )),
        }
    }
}
