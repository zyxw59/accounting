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
pub struct AccountQuery {
    pub name: Option<SimpleQuery<String>>,
    pub description: Option<SimpleQuery<String>>,
}

impl Query<Account> for AccountQuery {
    fn matches(&self, account: &Account) -> bool {
        self.name
            .as_ref()
            .map(|q| q.matches(&account.name))
            .unwrap_or(true)
            && self
                .description
                .as_ref()
                .map(|q| q.matches(&account.description))
                .unwrap_or(true)
    }

    fn serialize_query<F, S>(&self, factory: &F) -> Result<SerializedQuery<S::Ok>, S::Error>
    where
        F: Fn() -> S,
        S: Serializer,
    {
        let name_query = self
            .name
            .as_ref()
            .map(|query| {
                Ok(SerializedQuery::from_path_and_query(
                    "name",
                    query.serialize_value(factory)?.into(),
                ))
            })
            .transpose()?;
        let description_query = self
            .description
            .as_ref()
            .map(|query| {
                Ok(SerializedQuery::from_path_and_query(
                    "description",
                    query.serialize_value(factory)?.into(),
                ))
            })
            .transpose()?;

        Ok(SerializedQuery::all_opt([name_query, description_query]))
    }
}
