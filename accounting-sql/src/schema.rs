// @generated automatically by Diesel CLI.

diesel::table! {
    amount_parameters (id, param_name, param_value) {
        id -> Int8,
        param_name -> Text,
        param_value -> Int8,
    }
}

diesel::table! {
    date_parameters (id, param_name, param_value) {
        id -> Int8,
        param_name -> Text,
        param_value -> Date,
    }
}

diesel::table! {
    group_user_access (id, user_) {
        id -> Int8,
        user_ -> Int8,
        access -> Int8,
    }
}

diesel::table! {
    integer_parameters (id, param_name, param_value) {
        id -> Int8,
        param_name -> Text,
        param_value -> Int4,
    }
}

diesel::table! {
    reference_parameters (id, param_name, param_value) {
        id -> Int8,
        param_name -> Text,
        param_value -> Int8,
    }
}

diesel::table! {
    resources (id) {
        id -> Int8,
        #[sql_name = "type"]
        type_ -> Text,
        resource -> Nullable<Jsonb>,
    }
}

diesel::table! {
    string_parameters (id, param_name, param_value) {
        id -> Int8,
        param_name -> Text,
        param_value -> Text,
    }
}

diesel::table! {
    transaction_account_amount (id, account) {
        id -> Int8,
        account -> Int8,
        amount -> Int8,
    }
}

diesel::joinable!(amount_parameters -> resources (id));
diesel::joinable!(date_parameters -> resources (id));
diesel::joinable!(integer_parameters -> resources (id));
diesel::joinable!(string_parameters -> resources (id));

diesel::allow_tables_to_appear_in_same_query!(
    amount_parameters,
    date_parameters,
    group_user_access,
    integer_parameters,
    reference_parameters,
    resources,
    string_parameters,
    transaction_account_amount,
);
