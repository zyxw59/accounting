use accounting_core::{
    backend::{
        id::Id,
        user::{Group, User, WithGroup},
    },
    public::{account::Account, transaction::Transaction},
};
use sqlx::{query_builder::Separated, Postgres, QueryBuilder};

use crate::query::TableName;

pub trait Indexable: Sized {
    fn index(id: Id<WithGroup<Self>>, object: &WithGroup<Self>) -> Vec<QueryBuilder<Postgres>>;
}

type PushParameter<'a, T> =
    for<'b, 'c> fn(&'b mut Separated<'c, 'a, Postgres, &'static str>, &'a T);

fn index_table<'a, T: 'a, const C: usize>(
    table: TableName,
    id: Id<T>,
    object: &'a T,
    columns: [&'static str; C],
    parameters: [PushParameter<'a, T>; C],
) -> QueryBuilder<'a, Postgres> {
    let mut qb = QueryBuilder::new(format!("INSERT INTO {table}(id, "));
    let mut push_columns = qb.separated(",");
    for col in columns {
        push_columns.push(col);
    }
    qb.push(") ");
    qb.push_values([parameters], |mut separated, row| {
        separated.push_bind(id);
        for f in row {
            f(&mut separated, object);
        }
    });
    qb
}

impl Indexable for Account {
    fn index(id: Id<WithGroup<Self>>, object: &WithGroup<Self>) -> Vec<QueryBuilder<Postgres>> {
        let singular = index_table(
            TableName::SINGULAR_PARAMETERS,
            id,
            object,
            ["group_", "name", "description"],
            [
                |q, v| {
                    q.push_bind(v.group);
                },
                |q, v| {
                    q.push_bind(&v.object.name);
                },
                |q, v| {
                    q.push_bind(&v.object.description);
                },
            ],
        );
        vec![singular]
    }
}

impl Indexable for Transaction {
    fn index(id: Id<WithGroup<Self>>, object: &WithGroup<Self>) -> Vec<QueryBuilder<Postgres>> {
        let singular = index_table(
            TableName::SINGULAR_PARAMETERS,
            id,
            object,
            ["group_", "description", "date"],
            [
                |q, v| {
                    q.push_bind(v.group);
                },
                |q, v| {
                    q.push_bind(&v.object.description);
                },
                |q, v| {
                    q.push_bind(v.object.date);
                },
            ],
        );

        let mut account_amount = QueryBuilder::new(format!(
            "INSERT INTO {}(id, account, amount) ",
            TableName::ACCOUNT_AMOUNT,
        ));
        // TODO: use `UNNEST` for more uniform queries
        account_amount.push_values(
            object.object.amounts.iter(),
            |mut row, (account, amount)| {
                row.push_bind(id);
                row.push_bind(account);
                row.push_bind(amount);
            },
        );
        vec![singular, account_amount]
    }
}

impl Indexable for Group {
    fn index(id: Id<WithGroup<Self>>, object: &WithGroup<Self>) -> Vec<QueryBuilder<Postgres>> {
        let mut singular = QueryBuilder::new(format!(
            "INSERT INTO {}(id, group_, name) VALUES (",
            TableName::SINGULAR_PARAMETERS,
        ));
        let mut values = singular.separated(",");
        values.push_bind(id);
        values.push_bind(object.group);
        values.push_bind(&object.object.name);
        singular.push(")");

        let mut user_access = QueryBuilder::new(format!(
            "INSERT INTO {}(id, user, access) ",
            TableName::USER_ACCESS,
        ));
        // TODO: use `UNNEST` for more uniform queries
        user_access.push_values(
            object.object.permissions.users.iter(),
            |mut row, (user, access)| {
                row.push_bind(id);
                row.push_bind(user);
                row.push_bind(access);
            },
        );
        vec![singular, user_access]
    }
}

impl Indexable for User {
    fn index(id: Id<WithGroup<Self>>, object: &WithGroup<Self>) -> Vec<QueryBuilder<Postgres>> {
        let mut singular = QueryBuilder::new(format!(
            "INSERT INTO {}(id, group_, name) VALUES (",
            TableName::SINGULAR_PARAMETERS,
        ));
        let mut values = singular.separated(",");
        values.push_bind(id);
        values.push_bind(object.group);
        values.push_bind(&object.object.name);
        singular.push(")");
        vec![singular]
    }
}
