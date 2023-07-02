use accounting_core::{
    backend::{
        collection::Collection,
        id::Id,
        query::{Query, Queryable},
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

    async fn query_count(&self, query: Query<T>) -> Result<usize>
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

fn query_to_document<T>(query: Query<T>) -> Result<bson::Document>
where
    T: Queryable,
{
    query
        .serialize_query(bson_serializer)
        .and_then(|query| to_bson_document(&query))
        .map_err(Error::backend)
}

/// Default BSON serializer options, except the `human_readable` flag is set to false, to ensure
/// correct serialization of dates.
fn bson_serializer_options() -> bson::SerializerOptions {
    bson::SerializerOptions::builder()
        .human_readable(false)
        .build()
}

fn to_bson_document<T: Serialize>(value: &T) -> Result<bson::Document, bson::ser::Error> {
    bson::to_document_with_options(value, bson_serializer_options())
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
