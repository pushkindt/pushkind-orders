// @generated automatically by Diesel CLI.

diesel::table! {
    order_products (id) {
        id -> Integer,
        order_id -> Integer,
        product_id -> Nullable<Integer>,
        name -> Text,
        sku -> Nullable<Text>,
        description -> Nullable<Text>,
        price_cents -> Integer,
        currency -> Text,
        quantity -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    orders (id) {
        id -> Integer,
        hub_id -> Integer,
        customer_id -> Nullable<Integer>,
        reference -> Nullable<Text>,
        status -> Text,
        notes -> Nullable<Text>,
        total_cents -> Integer,
        currency -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    products (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
        sku -> Nullable<Text>,
        description -> Nullable<Text>,
        price_cents -> Integer,
        currency -> Text,
        is_archived -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    templates (id) {
        id -> Integer,
        hub_id -> Integer,
        value -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
        email -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(order_products -> orders (order_id));

diesel::allow_tables_to_appear_in_same_query!(order_products, orders, products, templates, users,);
