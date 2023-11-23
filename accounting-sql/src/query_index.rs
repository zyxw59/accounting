use std::fmt;

use accounting_core::{
    backend::{
        id::Id,
        query::{Queryable, SimpleQueryRef},
        user::Group,
    },
    date::Date,
};
use sqlx::{Postgres, QueryBuilder};

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
    type Index<'a>
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
        push_simple_query, Indexable, QueryOrIndex, Singular, SqlTable, TableIndex, TableName,
        ToSqlQuery,
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
