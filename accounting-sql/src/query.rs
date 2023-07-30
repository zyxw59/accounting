use std::fmt;

use accounting_core::{
    backend::{
        query::{Queryable, SimpleQuery},
        user::{GroupQuery, WithGroupQuery},
    },
    public::{account::AccountQuery, transaction::TransactionQuery},
};
use sqlx::{Postgres, QueryBuilder};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TableName(pub &'static str);

impl fmt::Display for TableName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl TableName {
    const SINGULAR_PARAMETERS: Self = Self("singular_parameters");
    const ACCOUNT_AMOUNT: Self = Self("account_amount");
    const USER_ACCESS: Self = Self("user_access");
}

pub fn query<'a, Q: ToSqlQuery>(
    select: &'static str,
    queries: &'a [Q],
    type_name: &'static str,
) -> QueryBuilder<'a, Postgres> {
    let mut qb = QueryBuilder::new(format!(
        "SELECT {select} FROM resources JOIN singular_parameters USING (id)"
    ));
    for (index, param) in queries.iter().enumerate() {
        let table = param.table();
        if table != TableName::SINGULAR_PARAMETERS {
            qb.push(format_args!(
                " JOIN {table} {} USING (id)",
                TableIndex(index)
            ));
        }
    }
    qb.push(format_args!(" WHERE resources.type = '{type_name}'"));
    for (index, param) in queries.iter().enumerate() {
        param.push_query(&mut qb, TableIndex(index));
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
    query: &'a SimpleQuery<T>,
    builder: &mut QueryBuilder<'a, Postgres>,
) where
    T: sqlx::Encode<'a, Postgres>
        + sqlx::Type<Postgres>
        + sqlx::postgres::PgHasArrayType
        + Send
        + Sync
        + 'a,
{
    let SimpleQuery {
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

#[derive(Clone, Copy, Debug)]
pub struct TableIndex(usize);

impl fmt::Display for TableIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

pub trait ToSqlQuery {
    fn table(&self) -> TableName;
    fn push_query<'a>(&'a self, builder: &mut QueryBuilder<'a, Postgres>, table_index: TableIndex);
}

impl ToSqlQuery for AccountQuery {
    fn table(&self) -> TableName {
        match self {
            AccountQuery::Name(_) | AccountQuery::Description(_) => TableName::SINGULAR_PARAMETERS,
        }
    }

    fn push_query<'a>(
        &'a self,
        builder: &mut QueryBuilder<'a, Postgres>,
        _table_index: TableIndex,
    ) {
        match self {
            AccountQuery::Name(query) => {
                push_simple_query(TableName::SINGULAR_PARAMETERS, "name", query, builder)
            }
            AccountQuery::Description(query) => push_simple_query(
                TableName::SINGULAR_PARAMETERS,
                "description",
                query,
                builder,
            ),
        }
    }
}

impl ToSqlQuery for TransactionQuery {
    fn table(&self) -> TableName {
        match self {
            TransactionQuery::Description(_) | TransactionQuery::Date(_) => {
                TableName::SINGULAR_PARAMETERS
            }
            TransactionQuery::Account(_) | TransactionQuery::AccountAmount(..) => {
                TableName::ACCOUNT_AMOUNT
            }
        }
    }

    fn push_query<'a>(&'a self, builder: &mut QueryBuilder<'a, Postgres>, table_index: TableIndex) {
        match self {
            TransactionQuery::Description(query) => push_simple_query(
                TableName::SINGULAR_PARAMETERS,
                "description",
                query,
                builder,
            ),
            TransactionQuery::Date(query) => {
                push_simple_query(TableName::SINGULAR_PARAMETERS, "date", query, builder)
            }
            TransactionQuery::Account(accounts) => {
                push_vec_query(table_index, "account", IN_OPERATOR, accounts, builder)
            }
            TransactionQuery::AccountAmount(account, amount) => {
                push_value_query(table_index, "account", "=", account, builder);
                push_simple_query(table_index, "amount", amount, builder);
            }
        }
    }
}

impl ToSqlQuery for GroupQuery {
    fn table(&self) -> TableName {
        match self {
            GroupQuery::Name(_) => TableName::SINGULAR_PARAMETERS,
            GroupQuery::UserAny(_) | GroupQuery::UserPerm(..) => TableName::USER_ACCESS,
        }
    }

    fn push_query<'a>(&'a self, builder: &mut QueryBuilder<'a, Postgres>, table_index: TableIndex) {
        match self {
            GroupQuery::Name(query) => {
                push_simple_query(TableName::SINGULAR_PARAMETERS, "name", query, builder);
            }
            GroupQuery::UserAny(users) => {
                push_vec_query(table_index, "user", IN_OPERATOR, users, builder);
            }
            GroupQuery::UserPerm(user, access) => {
                push_value_query(table_index, "user", "=", user, builder);
                push_simple_query(table_index, "access", access, builder);
            }
        }
    }
}

impl<T> ToSqlQuery for WithGroupQuery<T>
where
    T: Queryable,
    T::Query: ToSqlQuery,
{
    fn table(&self) -> TableName {
        match self {
            WithGroupQuery::Group(_) => TableName::SINGULAR_PARAMETERS,
            WithGroupQuery::Other(query) => query.table(),
        }
    }

    fn push_query<'a>(&'a self, builder: &mut QueryBuilder<'a, Postgres>, table_index: TableIndex) {
        match self {
            WithGroupQuery::Group(groups) => push_vec_query(
                TableName::SINGULAR_PARAMETERS,
                "group_",
                IN_OPERATOR,
                groups,
                builder,
            ),
            WithGroupQuery::Other(query) => query.push_query(builder, table_index),
        }
    }
}
