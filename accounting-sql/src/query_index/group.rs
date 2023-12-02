use accounting_core::backend::{
    id::Id,
    query::SimpleQueryRef,
    user::{AccessLevel, Group, GroupQuery, User},
};
use sqlx::{Postgres, QueryBuilder};

use super::{
    index_values::{push_parameter, IndexValues, PushParameter},
    push_simple_query, Indexable, QueryOrIndex, Singular, SqlIndexQueries, SqlTable, TableIndex,
    TableName, ToSqlQuery,
};

pub enum Query<'a> {
    Singular(Singular<'a, super::Query>),
    UserAccess(UserAccess<'a, super::Query>),
}

impl SqlTable for Query<'_> {
    fn table(&self) -> TableName {
        match self {
            Self::Singular(query) => query.table(),
            Self::UserAccess(query) => query.table(),
        }
    }
}

impl<'a> ToSqlQuery<'a> for Query<'a> {
    fn push_query(&self, builder: &mut QueryBuilder<'a, Postgres>, table_index: TableIndex) {
        match self {
            Self::Singular(query) => query.push_query(builder, table_index),
            Self::UserAccess(query) => query.push_query(builder, table_index),
        }
    }
}

pub struct Index<'a> {
    singular: Singular<'a, super::Index>,
    user_access: Vec<UserAccess<'a, super::Index>>,
}

#[async_trait::async_trait]
impl<'a> SqlIndexQueries<'a, Group> for Index<'a> {
    async fn insert_index(
        &self,
        id: Id<Group>,
        transaction: &mut sqlx::Transaction<'a, Postgres>,
    ) -> sqlx::Result<()> {
        self.singular.insert_index(id, transaction).await?;
        self.user_access.insert_index(id, transaction).await?;
        Ok(())
    }

    async fn remove_index(
        id: Id<Group>,
        transaction: &mut sqlx::Transaction<'a, Postgres>,
    ) -> sqlx::Result<()> {
        Singular::remove_index(id, transaction).await?;
        UserAccess::remove_index(id, transaction).await?;
        Ok(())
    }

    async fn update_index(
        &self,
        id: Id<Group>,
        transaction: &mut sqlx::Transaction<'a, Postgres>,
    ) -> sqlx::Result<()> {
        self.singular.update_index(id, transaction).await?;
        self.user_access.update_index(id, transaction).await?;
        Ok(())
    }
}

pub struct UserAccess<'a, T: QueryOrIndex> {
    user: T::Value<'a, Id<User>>,
    access: T::Value<'a, AccessLevel>,
}

impl<T: QueryOrIndex> UserAccess<'_, T> {
    const USER: &str = "user";
    const ACCESS: &str = "access";
}

impl<T: QueryOrIndex> SqlTable for UserAccess<'_, T> {
    fn table(&self) -> TableName {
        TableName::USER_ACCESS
    }
}

impl<'a> ToSqlQuery<'a> for UserAccess<'a, super::Query> {
    fn push_query(&self, builder: &mut QueryBuilder<'a, Postgres>, table_index: TableIndex) {
        let Self { user, access } = self;
        push_simple_query(table_index, Self::USER, *user, builder);
        push_simple_query(table_index, Self::ACCESS, *access, builder);
    }
}

impl<'a> IndexValues<'a> for UserAccess<'a, super::Index> {
    type Array<T> = [T; 2];
    const COLUMNS: Self::Array<&'static str> = [Self::USER, Self::ACCESS];
    const PARAMETERS: Self::Array<PushParameter<'a, Self>> =
        [push_parameter!(this.user), push_parameter!(this.access)];
    const TABLE: TableName = TableName::USER_ACCESS;
}

impl Indexable for Group {
    type IndexQuery<'a> = Query<'a>;
    type Index<'a> = Index<'a>;

    fn index(&self) -> Self::Index<'_> {
        Index {
            singular: Singular {
                name: Some(&self.name),
                ..Default::default()
            },
            user_access: self
                .permissions
                .users
                .iter()
                .map(|(user, access)| UserAccess { user, access })
                .collect(),
        }
    }

    fn transform_query(query: &GroupQuery) -> Self::IndexQuery<'_> {
        match query {
            GroupQuery::Name(name) => Query::Singular(Singular {
                name: Some(name.as_ref()),
                ..Default::default()
            }),
            GroupQuery::UserAny(users) => Query::UserAccess(UserAccess {
                user: SimpleQueryRef::in_(users),
                access: Default::default(),
            }),
            GroupQuery::UserPerm(user, access) => Query::UserAccess(UserAccess {
                user: SimpleQueryRef::eq(user),
                access: access.as_ref(),
            }),
        }
    }
}
