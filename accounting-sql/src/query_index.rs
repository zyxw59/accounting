use std::fmt;

use accounting_core::{
    backend::{
        id::Id,
        query::{Queryable, SimpleQueryRef},
        user::Group,
    },
    date::Date,
};
use sqlx::{Postgres, QueryBuilder, Transaction};

pub mod group;
pub mod transaction;

mod index_values {
    use accounting_core::backend::id::Id;
    use sqlx::{query_builder::Separated, Postgres, QueryBuilder, Transaction};

    use super::TableName;

    pub(super) type PushParameter<'a, T> =
        for<'b, 'c> fn(&'b mut Separated<'c, 'a, Postgres, &'static str>, &T);

    macro_rules! push_parameter {
        ($this:ident . $($rest:tt)+) => {
            |q, $this| {
                q.push_bind($this.$($rest)+);
            }
        };
    }
    pub(super) use push_parameter;

    pub(super) trait IndexValues<'a> {
        /// This should be an array ([`[T; N]`][array]), to ensure that [`COLUMNS`][Self::COLUMNS]
        /// and [`PARAMETERS`][Self::PARAMETERS] are the same length.
        type Array<T>: IntoIterator<Item = T>;

        const COLUMNS: Self::Array<&'static str>;

        const PARAMETERS: Self::Array<PushParameter<'a, Self>>;

        const TABLE: TableName;
    }

    // TODO: use `UNNEST`
    pub(super) fn index_many<'a, 'i, T: 'a, I: 'i + 'a>(
        id: Id<T>,
        indexes: impl IntoIterator<Item = &'i I>,
    ) -> QueryBuilder<'a, Postgres>
    where
        I: IndexValues<'a>,
    {
        let mut qb = QueryBuilder::new(format!("INSERT INTO {}(id, ", I::TABLE));
        let mut push_columns = qb.separated(", ");
        for col in I::COLUMNS {
            push_columns.push(col);
        }
        qb.push(") ");
        qb.push_values(indexes, |mut separated, index| {
            separated.push_bind(id);
            for f in I::PARAMETERS {
                f(&mut separated, index);
            }
        });
        qb
    }

    #[async_trait::async_trait]
    impl<'a, T, I> super::SqlIndexQueries<'a, T> for I
    where
        T: 'a,
        I: IndexValues<'a> + Send + Sync + 'a,
    {
        async fn insert_index(
            &self,
            id: Id<T>,
            transaction: &mut Transaction<'a, Postgres>,
        ) -> sqlx::Result<()> {
            index_many(id, [self])
                .build()
                .execute(&mut **transaction)
                .await?;
            Ok(())
        }

        async fn remove_index(
            id: Id<T>,
            transaction: &mut Transaction<'a, Postgres>,
        ) -> sqlx::Result<()> {
            let mut qb = QueryBuilder::new(format!("DELETE FROM {} WHERE id = ", Self::TABLE));
            qb.push_bind(id);
            qb.build().execute(&mut **transaction).await?;
            Ok(())
        }

        async fn update_index(
            &self,
            id: Id<T>,
            transaction: &mut Transaction<'a, Postgres>,
        ) -> sqlx::Result<()> {
            let mut qb = index_many(id, [self]);
            qb.push(" ON CONFLICT DO UPDATE");
            qb.build().execute(&mut **transaction).await?;
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl<'a, T, I> super::SqlIndexQueries<'a, T> for [I]
    where
        T: 'a,
        I: IndexValues<'a> + Send + Sync + 'a,
    {
        async fn insert_index(
            &self,
            id: Id<T>,
            transaction: &mut Transaction<'a, Postgres>,
        ) -> sqlx::Result<()> {
            index_many(id, self)
                .build()
                .execute(&mut **transaction)
                .await?;
            Ok(())
        }

        async fn remove_index(
            id: Id<T>,
            transaction: &mut Transaction<'a, Postgres>,
        ) -> sqlx::Result<()> {
            let mut qb = QueryBuilder::new(format!("DELETE FROM {} WHERE id = ", I::TABLE));
            qb.push_bind(id);
            qb.build().execute(&mut **transaction).await?;
            Ok(())
        }

        async fn update_index(
            &self,
            id: Id<T>,
            transaction: &mut Transaction<'a, Postgres>,
        ) -> sqlx::Result<()> {
            Self::remove_index(id, transaction).await?;
            self.insert_index(id, transaction).await?;
            Ok(())
        }
    }
}

pub trait QueryOrIndex {
    type Value<'a, T>
    where
        T: 'a;
}
pub struct Query;
impl QueryOrIndex for Query {
    type Value<'a, T> = SimpleQueryRef<'a, T> where T: 'a;
}
pub struct Index;
impl QueryOrIndex for Index {
    type Value<'a, T> = &'a T where T: 'a;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TableName(pub &'static str);

impl fmt::Display for TableName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl TableName {
    pub const SINGULAR_PARAMETERS: Self = Self("singular_parameters");
    pub const ACCOUNT_AMOUNT: Self = Self("account_amount");
    pub const USER_ACCESS: Self = Self("user_access");
}

#[derive(Clone, Copy, Debug)]
pub struct TableIndex(usize);

impl fmt::Display for TableIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

pub trait Indexable: Queryable {
    type IndexQuery<'a>: ToSqlQuery<'a>;
    type Index<'a>: SqlIndexQueries<'a, Self>
    where
        Self: 'a;

    fn index<'a>(&'a self, group: &'a Id<Group>) -> Self::Index<'a>;
    fn transform_query(query: &Self::Query) -> Self::IndexQuery<'_>;
}

pub trait SqlTable {
    fn table(&self) -> TableName;
}

pub trait ToSqlQuery<'a>: SqlTable {
    fn push_query(&self, builder: &mut QueryBuilder<'a, Postgres>, table_index: TableIndex);
}

/// A representation of a set of indexes for a single object, that can be transformed into SQL
/// queries to insert, remove, or update the values in the database.
#[async_trait::async_trait]
pub trait SqlIndexQueries<'a, T: 'a> {
    /// Insert the indexes into the database
    async fn insert_index(
        &self,
        id: Id<T>,
        transaction: &mut Transaction<'a, Postgres>,
    ) -> sqlx::Result<()>;

    /// Remove all indexes corresponding to this object from the database
    async fn remove_index(
        id: Id<T>,
        transaction: &mut Transaction<'a, Postgres>,
    ) -> sqlx::Result<()>;

    /// Update the indexes in the database.
    async fn update_index(
        &self,
        id: Id<T>,
        transaction: &mut Transaction<'a, Postgres>,
    ) -> sqlx::Result<()>;
}

pub fn query<'a, T: Indexable + 'a>(
    select: &'static str,
    queries: &'a [T::Query],
    type_name: &'static str,
) -> QueryBuilder<'a, Postgres> {
    let mut qb = QueryBuilder::new(format!(
        "SELECT {select} FROM resources JOIN {} USING (id)",
        TableName::SINGULAR_PARAMETERS,
    ));
    for (index, param) in queries.iter().enumerate() {
        let table = T::transform_query(param).table();
        if table != TableName::SINGULAR_PARAMETERS {
            qb.push(format_args!(
                " JOIN {table} {} USING (id)",
                TableIndex(index)
            ));
        }
    }
    qb.push(format_args!(" WHERE resources.type = '{type_name}'"));
    for (index, param) in queries.iter().enumerate() {
        T::transform_query(param).push_query(&mut qb, TableIndex(index));
    }
    qb.push(" DISTINCT BY (id)");
    qb
}

fn push_vec_query<'a, T>(
    table: impl fmt::Display,
    column: &'static str,
    operator: &'static str,
    values: &'a [T],
    builder: &mut QueryBuilder<'a, Postgres>,
) where
    T: sqlx::Encode<'a, Postgres>
        + sqlx::Type<Postgres>
        + sqlx::postgres::PgHasArrayType
        + Send
        + Sync
        + 'a,
{
    builder.push(format_args!(" AND {table}.{column} {operator}("));
    builder.push_bind(values);
    builder.push(")");
}

const IN_OPERATOR: &str = "= ANY";
const NOT_IN_OPERATOR: &str = "<> ALL";

fn push_value_query<'a, T>(
    table: impl fmt::Display,
    column: &'static str,
    operator: &'static str,
    value: &'a T,
    builder: &mut QueryBuilder<'a, Postgres>,
) where
    T: sqlx::Encode<'a, Postgres> + sqlx::Type<Postgres> + Send + Sync + 'a,
{
    builder.push(format_args!(" AND {table}.{column} {operator} "));
    builder.push_bind(value);
}

fn push_simple_query<'a, T>(
    table: impl fmt::Display,
    column: &'static str,
    query: SimpleQueryRef<'a, T>,
    builder: &mut QueryBuilder<'a, Postgres>,
) where
    T: sqlx::Encode<'a, Postgres>
        + sqlx::Type<Postgres>
        + sqlx::postgres::PgHasArrayType
        + Send
        + Sync
        + 'a,
{
    let SimpleQueryRef {
        eq,
        ne,
        lt,
        le,
        gt,
        ge,
        in_,
        nin,
    } = query;
    if let Some(val) = eq {
        push_value_query(&table, column, "=", val, builder);
    }
    if let Some(val) = ne {
        push_value_query(&table, column, "<>", val, builder);
    }
    if let Some(val) = lt {
        push_value_query(&table, column, "<", val, builder);
    }
    if let Some(val) = le {
        push_value_query(&table, column, "<=", val, builder);
    }
    if let Some(val) = gt {
        push_value_query(&table, column, ">", val, builder);
    }
    if let Some(val) = ge {
        push_value_query(&table, column, "<=", val, builder);
    }
    if let Some(vals) = in_ {
        push_vec_query(&table, column, IN_OPERATOR, vals, builder)
    }
    if let Some(vals) = nin {
        push_vec_query(&table, column, NOT_IN_OPERATOR, vals, builder)
    }
}

#[derive(derivative::Derivative)]
#[derivative(Default(bound = ""))]
pub struct Singular<'a, T: QueryOrIndex> {
    group: Option<T::Value<'a, Id<Group>>>,
    name: Option<T::Value<'a, String>>,
    description: Option<T::Value<'a, String>>,
    date: Option<T::Value<'a, Date>>,
}

impl<T: QueryOrIndex> Singular<'_, T> {
    const GROUP: &str = "group_";
    const NAME: &str = "name";
    const DESCRIPTION: &str = "description";
    const DATE: &str = "date";
}

impl<T: QueryOrIndex> SqlTable for Singular<'_, T> {
    fn table(&self) -> TableName {
        TableName::SINGULAR_PARAMETERS
    }
}

impl<'a> ToSqlQuery<'a> for Singular<'a, Query> {
    fn push_query(&self, builder: &mut QueryBuilder<'a, Postgres>, _table_index: TableIndex) {
        let Self {
            group,
            name,
            description,
            date,
        } = self;
        if let Some(query) = group {
            push_simple_query(TableName::SINGULAR_PARAMETERS, Self::GROUP, *query, builder);
        }
        if let Some(query) = name {
            push_simple_query(TableName::SINGULAR_PARAMETERS, Self::NAME, *query, builder);
        }
        if let Some(query) = description {
            push_simple_query(
                TableName::SINGULAR_PARAMETERS,
                Self::DESCRIPTION,
                *query,
                builder,
            );
        }
        if let Some(query) = date {
            push_simple_query(TableName::SINGULAR_PARAMETERS, Self::DATE, *query, builder);
        }
    }
}

impl<'a> index_values::IndexValues<'a> for Singular<'a, Index> {
    type Array<T> = [T; 4];

    const COLUMNS: Self::Array<&'static str> =
        [Self::GROUP, Self::NAME, Self::DESCRIPTION, Self::DATE];

    const PARAMETERS: Self::Array<index_values::PushParameter<'a, Self>> = [
        index_values::push_parameter!(this.group),
        index_values::push_parameter!(this.name),
        index_values::push_parameter!(this.description),
        index_values::push_parameter!(this.date),
    ];

    const TABLE: TableName = TableName::SINGULAR_PARAMETERS;
}
