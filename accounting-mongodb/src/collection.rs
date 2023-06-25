use accounting_core::{
    backend::{
        collection::Collection,
        id::Id,
        query::{Comparator, Query, QueryParameter, Queryable},
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
    query.try_fold_expr(
        |clauses| Ok(bson::doc! { "$and": clauses }),
        |clauses| Ok(bson::doc! { "$or": clauses }),
        |expr| Ok(bson::doc! { "$not": expr }),
        |param| {
            let path = param.path().join(".");
            let comparator = comparator_to_operator(param.comparator());
            let value = param
                .serialize_value(bson_serializer())
                .map_err(Error::backend)?;
            Ok(bson::doc! { path: { comparator: value } })
        },
    )
}

fn bson_serializer() -> bson::Serializer {
    let options = bson::SerializerOptions::builder()
        .human_readable(false)
        .build();
    bson::Serializer::new_with_options(options)
}

fn comparator_to_operator(comparator: Comparator) -> &'static str {
    match comparator {
        Comparator::Equal => "$eq",
        Comparator::NotEqual => "$neq",
        Comparator::Greater => "$gt",
        Comparator::GreaterOrEqual => "$gte",
        Comparator::Less => "$lt",
        Comparator::LessOrEqual => "$lte",
    }
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
