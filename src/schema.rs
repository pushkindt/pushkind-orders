// @generated automatically by Diesel CLI.

diesel::table! {
    categories (id) {
        id -> Integer,
        hub_id -> Integer,
        parent_id -> Nullable<Integer>,
        name -> Text,
        description -> Nullable<Text>,
        is_archived -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    customers (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
        email -> Text,
        price_level_id -> Nullable<Integer>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

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
    price_levels (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    product_price_levels (id) {
        id -> Integer,
        product_id -> Integer,
        price_level_id -> Integer,
        price_cents -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    product_tags (id) {
        id -> Integer,
        product_id -> Integer,
        tag_id -> Integer,
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
        currency -> Text,
        is_archived -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        units -> Nullable<Text>,
        category_id -> Nullable<Integer>,
    }
}

diesel::table! {
    tags (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
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

diesel::joinable!(customers -> price_levels (price_level_id));
diesel::joinable!(order_products -> orders (order_id));
diesel::joinable!(orders -> customers (customer_id));
diesel::joinable!(product_price_levels -> price_levels (price_level_id));
diesel::joinable!(product_price_levels -> products (product_id));
diesel::joinable!(product_tags -> products (product_id));
diesel::joinable!(product_tags -> tags (tag_id));
diesel::joinable!(products -> categories (category_id));

diesel::allow_tables_to_appear_in_same_query!(
    categories,
    customers,
    order_products,
    orders,
    price_levels,
    product_price_levels,
    product_tags,
    products,
    tags,
    users,
);
