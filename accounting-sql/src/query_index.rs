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
        "SELECT {select} FROM resources JOIN singular_parameters USING (id)"
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

mod index_values {
    use accounting_core::backend::id::Id;
    use sqlx::{query_builder::Separated, Postgres, QueryBuilder, Transaction};

    use super::{Index, Singular, TableName};

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

    impl<'a> IndexValues<'a> for Singular<'a, Index> {
        type Array<T> = [T; 4];

        const COLUMNS: Self::Array<&'static str> = ["group_", "name", "description", "date"];

        const PARAMETERS: Self::Array<PushParameter<'a, Self>> = [
            push_parameter!(this.group),
            push_parameter!(this.name),
            push_parameter!(this.description),
            push_parameter!(this.date),
        ];

        const TABLE: TableName = TableName::SINGULAR_PARAMETERS;
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

    #[cfg(test)]
    #[test]
    fn singular_index_query() -> anyhow::Result<()> {
        use sqlx::Execute;

        let mut query = index_many(
            Id::<()>::new(0),
            &[Singular::<Index>::default(), Singular::<Index>::default()],
        );
        assert_eq!(query.build().sql(), "INSERT INTO singular_parameters(id, group_, name, description, date) VALUES ($1, $2, $3, $4, $5), ($6, $7, $8, $9, $10)");
        Ok(())
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

pub mod transaction {
    use accounting_core::{
        backend::{id::Id, query::SimpleQueryRef, user::Group},
        public::{
            account::Account,
            amount::Amount,
            transaction::{Transaction, TransactionQuery},
        },
    };
    use sqlx::{Postgres, QueryBuilder};

    use super::{
        index_values::{push_parameter, IndexValues, PushParameter},
        push_simple_query, Indexable, QueryOrIndex, Singular, SqlIndexQueries, SqlTable,
        TableIndex, TableName, ToSqlQuery,
    };

    pub enum Query<'a> {
        Singular(Singular<'a, super::Query>),
        AccountAmount(AccountAmount<'a, super::Query>),
    }

    impl SqlTable for Query<'_> {
        fn table(&self) -> TableName {
            match self {
                Self::Singular(query) => query.table(),
                Self::AccountAmount(query) => query.table(),
            }
        }
    }

    impl<'a> ToSqlQuery<'a> for Query<'a> {
        fn push_query(&self, builder: &mut QueryBuilder<'a, Postgres>, table_index: TableIndex) {
            match self {
                Self::Singular(query) => query.push_query(builder, table_index),
                Self::AccountAmount(query) => query.push_query(builder, table_index),
            }
        }
    }

    pub struct Index<'a> {
        singular: Singular<'a, super::Index>,
        account_amount: Vec<AccountAmount<'a, super::Index>>,
    }

    #[async_trait::async_trait]
    impl<'a> SqlIndexQueries<'a, Transaction> for Index<'a> {
        async fn insert_index(
            &self,
            id: Id<Transaction>,
            transaction: &mut sqlx::Transaction<'a, Postgres>,
        ) -> sqlx::Result<()> {
            self.singular.insert_index(id, transaction).await?;
            self.account_amount.insert_index(id, transaction).await?;
            Ok(())
        }

        async fn remove_index(
            id: Id<Transaction>,
            transaction: &mut sqlx::Transaction<'a, Postgres>,
        ) -> sqlx::Result<()> {
            Singular::remove_index(id, transaction).await?;
            AccountAmount::remove_index(id, transaction).await?;
            Ok(())
        }

        async fn update_index(
            &self,
            id: Id<Transaction>,
            transaction: &mut sqlx::Transaction<'a, Postgres>,
        ) -> sqlx::Result<()> {
            self.singular.update_index(id, transaction).await?;
            self.account_amount.update_index(id, transaction).await?;
            Ok(())
        }
    }

    pub struct AccountAmount<'a, T: QueryOrIndex> {
        account: T::Value<'a, Id<Account>>,
        amount: T::Value<'a, Amount>,
    }

    impl<T: QueryOrIndex> AccountAmount<'_, T> {
        const ACCOUNT: &str = "account";
        const AMOUNT: &str = "amount";
    }

    impl<T: QueryOrIndex> SqlTable for AccountAmount<'_, T> {
        fn table(&self) -> TableName {
            TableName::ACCOUNT_AMOUNT
        }
    }

    impl<'a> ToSqlQuery<'a> for AccountAmount<'a, super::Query> {
        fn push_query(&self, builder: &mut QueryBuilder<'a, Postgres>, table_index: TableIndex) {
            let Self { account, amount } = self;
            push_simple_query(table_index, Self::ACCOUNT, *account, builder);
            push_simple_query(table_index, Self::AMOUNT, *amount, builder);
        }
    }

    impl<'a> IndexValues<'a> for AccountAmount<'a, super::Index> {
        type Array<T> = [T; 2];
        const COLUMNS: Self::Array<&'static str> = [Self::ACCOUNT, Self::AMOUNT];
        const PARAMETERS: Self::Array<PushParameter<'a, Self>> =
            [push_parameter!(this.account), push_parameter!(this.amount)];
        const TABLE: TableName = TableName::ACCOUNT_AMOUNT;
    }

    impl Indexable for Transaction {
        type IndexQuery<'a> = Query<'a>;
        type Index<'a> = Index<'a>;

        fn index<'a>(&'a self, group: &'a Id<Group>) -> Self::Index<'a> {
            Index {
                singular: Singular {
                    group: Some(group),
                    date: Some(&self.date),
                    description: Some(&self.description),
                    ..Default::default()
                },
                account_amount: self
                    .amounts
                    .iter()
                    .map(|(account, amount)| AccountAmount { account, amount })
                    .collect(),
            }
        }

        fn transform_query(query: &TransactionQuery) -> Self::IndexQuery<'_> {
            match query {
                TransactionQuery::Date(date) => Query::Singular(Singular {
                    date: Some(date.as_ref()),
                    ..Default::default()
                }),
                TransactionQuery::Description(description) => Query::Singular(Singular {
                    description: Some(description.as_ref()),
                    ..Default::default()
                }),
                TransactionQuery::Account(accounts) => Query::AccountAmount(AccountAmount {
                    account: SimpleQueryRef::in_(accounts),
                    amount: Default::default(),
                }),
                TransactionQuery::AccountAmount(account, amount) => {
                    Query::AccountAmount(AccountAmount {
                        account: SimpleQueryRef::eq(account),
                        amount: amount.as_ref(),
                    })
                }
            }
        }
    }
}

pub mod group {
    use accounting_core::backend::{
        id::Id,
        query::SimpleQueryRef,
        user::{AccessLevel, Group, GroupQuery, User},
    };
    use sqlx::{Postgres, QueryBuilder};

    use super::{
        index_values::{push_parameter, IndexValues, PushParameter},
        push_simple_query, Indexable, QueryOrIndex, Singular, SqlIndexQueries, SqlTable,
        TableIndex, TableName, ToSqlQuery,
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

        fn index<'a>(&'a self, group: &'a Id<Group>) -> Self::Index<'a> {
            Index {
                singular: Singular {
                    group: Some(group),
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
}
