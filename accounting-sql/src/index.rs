use accounting_core::{
    backend::{
        id::WithId,
        user::{Group, User, WithGroup},
    },
    public::{account::Account, transaction::Transaction},
};
use sqlx::{Postgres, QueryBuilder};

use crate::query::TableName;

pub trait Index: Sized {
    fn index(this: &WithId<WithGroup<Self>>) -> Vec<QueryBuilder<Postgres>>;
}

impl Index for Account {
    fn index(this: &WithId<WithGroup<Self>>) -> Vec<QueryBuilder<Postgres>> {
        let mut qb = QueryBuilder::new(format!(
            "INSERT INTO {}(id, group_, name, description) VALUES (",
            TableName::SINGULAR_PARAMETERS
        ));
        let mut values = qb.separated(",");
        values.push_bind(this.id);
        values.push_bind(this.object.group);
        values.push_bind(&this.object.object.name);
        values.push_bind(&this.object.object.description);
        qb.push(")");
        vec![qb]
    }
}

impl Index for Transaction {
    fn index(this: &WithId<WithGroup<Self>>) -> Vec<QueryBuilder<Postgres>> {
        let mut singular = QueryBuilder::new(format!(
            "INSERT INTO {}(id, group_, description, date) VALUES (",
            TableName::SINGULAR_PARAMETERS,
        ));
        let mut values = singular.separated(",");
        values.push_bind(this.id);
        values.push_bind(this.object.group);
        values.push_bind(&this.object.object.description);
        values.push_bind(this.object.object.date);
        singular.push(")");

        let mut account_amount = QueryBuilder::new(format!(
            "INSERT INTO {}(id, account, amount) ",
            TableName::ACCOUNT_AMOUNT,
        ));
        // TODO: use `UNNEST` for more uniform queries
        account_amount.push_values(this.object.object.amounts.iter(), |mut row, (account, amount)| {
            row.push_bind(this.id);
            row.push_bind(account);
            row.push_bind(amount);
        });
        vec![singular, account_amount]
    }
}

impl Index for Group {
    fn index(this: &WithId<WithGroup<Self>>) -> Vec<QueryBuilder<Postgres>> {
        let mut singular = QueryBuilder::new(format!(
            "INSERT INTO {}(id, group_, name) VALUES (",
            TableName::SINGULAR_PARAMETERS,
        ));
        let mut values = singular.separated(",");
        values.push_bind(this.id);
        values.push_bind(this.object.group);
        values.push_bind(&this.object.object.name);
        singular.push(")");

        let mut user_access = QueryBuilder::new(format!(
            "INSERT INTO {}(id, user, access) ",
            TableName::USER_ACCESS,
        ));
        // TODO: use `UNNEST` for more uniform queries
        user_access.push_values(this.object.object.permissions.users.iter(), |mut row, (user, access)| {
            row.push_bind(this.id);
            row.push_bind(user);
            row.push_bind(access);
        });
        vec![singular, user_access]
    }
}

impl Index for User {
    fn index(this: &WithId<WithGroup<Self>>) -> Vec<QueryBuilder<Postgres>> {
        let mut singular = QueryBuilder::new(format!(
            "INSERT INTO {}(id, group_, name) VALUES (",
            TableName::SINGULAR_PARAMETERS,
        ));
        let mut values = singular.separated(",");
        values.push_bind(this.id);
        values.push_bind(this.object.group);
        values.push_bind(&this.object.object.name);
        singular.push(")");
        vec![singular]
    }
}
