// @generated automatically by Diesel CLI.

diesel::table! {
    group_user_access (id, user_) {
        id -> Int8,
        user_ -> Int8,
        access -> Int8,
    }
}

diesel::table! {
    references_resource (id, reference_id) {
        id -> Int8,
        reference_id -> Int8,
    }
}

diesel::table! {
    resources (id) {
        id -> Int8,
        #[sql_name = "type"]
        type_ -> Text,
        resource -> Jsonb,
    }
}

diesel::table! {
    singular_parameters (id) {
        id -> Int8,
        group_ -> Int8,
        name -> Nullable<Text>,
        description -> Nullable<Text>,
        date_ -> Nullable<Date>,
    }
}

diesel::table! {
    transaction_account_amount (id, account) {
        id -> Int8,
        account -> Int8,
        amount -> Int8,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    group_user_access,
    references_resource,
    resources,
    singular_parameters,
    transaction_account_amount,
);
