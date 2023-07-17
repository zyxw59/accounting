use accounting_core::{
    backend::{
        collection::Collection,
        id::Id,
        query::{GroupQuery, Query, Queryable, SerializedQuery},
        user::{ChangeGroup, Group, WithGroup},
        version::{Version, Versioned},
    },
    error::{Error, Result},
};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

pub struct MongoDbCollection<T> {
    collection: mongodb::Collection<WithGroup<Versioned<T>>>,
}

#[async_trait]
impl<T> Collection<T> for MongoDbCollection<T>
where
    T: Serialize + DeserializeOwned + Send + Sync + Unpin,
{
    async fn create(&mut self, object: WithGroup<T>) -> Result<Id<T>> {
        let versioned = Versioned {
            id: Id::new_random(),
            version: Version::new_random(),
            object,
        }
        .transpose();

        self.collection
            .insert_one(&versioned, None)
            .await
            .map_err(Error::backend)?;

        Ok(versioned.object.id)
    }

    async fn get(&self, id: Id<T>) -> Result<Option<WithGroup<Versioned<T>>>> {
        self.collection
            .find_one(Some(query_id(id)), None)
            .await
            .map_err(Error::backend)
    }

    async fn update(&mut self, mut object: Versioned<T>) -> Result<()> {
        let query = query_id_version(object.id, object.version);
        object.version = Version::new_random();
        let ser_options = bson::SerializerOptions::builder()
            .human_readable(false)
            .build();
        let update_doc = bson::to_document_with_options(&object, ser_options)
            .map_err(mongodb::error::Error::from)
            .map_err(Error::backend)?;
        let update = bson::doc! { "$set": update_doc };
        let result = self
            .collection
            .update_one(query, update, None)
            .await
            .map_err(Error::backend)?;
        if result.matched_count != 1 {
            // if the id exists, this is a conflicting edit, otherwise it's just object not found
            if self
                .collection
                .find_one(query_id(object.id), None)
                .await
                .map_err(Error::backend)?
                .is_some()
            {
                Err(Error::ConflictingEdit)
            } else {
                Err(Error::NotFound)
            }
        } else {
            Ok(())
        }
    }

    async fn delete(&mut self, id: Id<T>) -> Result<()> {
        self.collection
            .delete_one(query_id(id), None)
            .await
            .map_err(Error::backend)?;
        Ok(())
    }

    async fn change_group(&mut self, id: Id<T>, new_group: Id<Group>) -> Result<()>
    where
        T: ChangeGroup,
    {
        let update_statement = bson::doc! {
            "$set": { VERSION_FIELD: Version::new_random(), GROUP_FIELD: new_group},
        };
        self.collection
            .update_one(query_id(id), update_statement, None)
            .await
            .map_err(Error::backend)?;
        Ok(())
    }

    async fn query_count(&self, query: GroupQuery<T>) -> Result<usize>
    where
        T: Queryable,
    {
        let query = query_to_document(query)?;
        let count = self
            .collection
            .count_documents(Some(query), None)
            .await
            .map_err(Error::backend)?;
        Ok(count as usize)
    }
}

fn query_to_document<T>(query: GroupQuery<T>) -> Result<bson::Document>
where
    T: Queryable,
{
    let query = query
        .serialize_query(&bson_serializer)
        .map_err(Error::backend)?;
    serialized_query_to_document(&query)
}

fn serialized_query_to_document(query: &SerializedQuery<bson::Bson>) -> Result<bson::Document> {
    use accounting_core::backend::query::{boolean::Folder, QueryElement, QueryPathMap};

    fn all_f(clauses: Vec<bson::Document>) -> Result<bson::Document> {
        Ok(bson::doc! { "$and": clauses })
    }

    fn any_f(clauses: Vec<bson::Document>) -> Result<bson::Document> {
        Ok(bson::doc! { "$or": clauses })
    }

    fn not_f(expr: bson::Document) -> Result<bson::Document> {
        Ok(bson::doc! { "$nor": [expr] })
    }

    fn query_element_to_bson(element: &QueryElement<bson::Bson>) -> Result<bson::Bson> {
        match element {
            QueryElement::ElemMatch(query) => {
                Ok(bson::bson!({ "$elemMatch": serialized_query_to_document(query)? }))
            }
            QueryElement::Comparator(query) => bson::to_bson(query).map_err(Error::backend),
            QueryElement::In(set) => Ok(bson::bson!({ "$in": set })),
            QueryElement::Not(query) => Ok(bson::bson!({ "$not": query_element_to_bson(query)? })),
        }
    }

    fn value_f(map: &QueryPathMap<bson::Bson>) -> Result<bson::Document> {
        map.iter()
            .map(|(k, v)| Ok((k.clone().into_owned(), query_element_to_bson(v)?)))
            .collect()
    }

    let folder = Folder {
        all_f,
        any_f,
        not_f,
        value_f,
    };
    query.try_fold_expr(&folder)
}

/// Default BSON serializer options, except the `human_readable` flag is set to false, to ensure
/// correct serialization of dates.
fn bson_serializer_options() -> bson::SerializerOptions {
    bson::SerializerOptions::builder()
        .human_readable(false)
        .build()
}

fn bson_serializer() -> bson::Serializer {
    bson::Serializer::new_with_options(bson_serializer_options())
}

const ID_FIELD: &str = "_id";
const VERSION_FIELD: &str = "_version";
const GROUP_FIELD: &str = "_group";

fn query_id<T>(id: Id<T>) -> bson::Document {
    bson::doc! { ID_FIELD: id }
}

fn query_id_version<T>(id: Id<T>, version: Version) -> bson::Document {
    bson::doc! { ID_FIELD: id, VERSION_FIELD: version }
}

#[cfg(test)]
mod tests {
    use accounting_core::{
        backend::{
            id::Id,
            query::{Comparator, GroupQuery, SimpleQuery},
        },
        public::{
            amount::Amount,
            transaction::{Transaction, TransactionQuery},
        },
    };
    use pretty_assertions::assert_eq;

    use super::{query_to_document, Result};

    #[test]
    fn query_transaction() -> Result<()> {
        let query: GroupQuery<Transaction> = GroupQuery {
            group: None,
            other: Some(TransactionQuery {
                description: Some(SimpleQuery::eq("test transaction".into())),
                date: None,
                account: Some(vec![Id::new(123)]),
                account_amount: Some((
                    Id::new(456),
                    SimpleQuery([(Comparator::Greater, Amount::ZERO)].into_iter().collect()),
                )),
            }),
        };
        // serialize to json for nicer formatting
        let actual = serde_json::to_string_pretty(&query_to_document(query)?).unwrap();
        let expected = serde_json::json!({
            "$and": [
                {"description": { "$eq": "test transaction" }},
                {"amounts": { "$elemMatch": { "0": { "$in": [123] }}}},
                {"amounts": { "$elemMatch": { "0": { "$eq": 456 }, "1": { "$gt": "0" }}}},
            ],
        });
        assert_eq!(actual, format!("{expected:#}"));
        Ok(())
    }
}
