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
        image_url -> Nullable<Text>,
    }
}

diesel::table! {
    customers (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
        price_level_id -> Nullable<Integer>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        phone -> Text,
        public_id -> Nullable<Text>,
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
        default_price_cents -> Nullable<Integer>,
        approved_quantity -> Nullable<Integer>,
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
        shipping_address -> Nullable<Text>,
        consignee -> Nullable<Text>,
        delivery_notes -> Nullable<Text>,
        payer -> Nullable<Text>,
    }
}

diesel::table! {
    price_levels (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        is_default -> Bool,
    }
}

diesel::table! {
    product_fts (rowid) {
        rowid -> Integer,
        name -> Nullable<Binary>,
        sku -> Nullable<Binary>,
        description -> Nullable<Binary>,
        #[sql_name = "product_fts"]
        product_fts_col -> Nullable<Binary>,
        rank -> Nullable<Binary>,
    }
}

diesel::table! {
    product_fts_config (k) {
        k -> Binary,
        v -> Nullable<Binary>,
    }
}

diesel::table! {
    product_fts_data (id) {
        id -> Nullable<Integer>,
        block -> Nullable<Binary>,
    }
}

diesel::table! {
    product_fts_docsize (id) {
        id -> Nullable<Integer>,
        sz -> Nullable<Binary>,
    }
}

diesel::table! {
    product_fts_idx (segid, term) {
        segid -> Binary,
        term -> Binary,
        pgno -> Nullable<Binary>,
    }
}

diesel::table! {
    product_images (id) {
        id -> Integer,
        product_id -> Integer,
        image_url -> Text,
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
        amount -> Nullable<Float>,
        vendor_id -> Nullable<Integer>,
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

diesel::table! {
    vendor_order (rowid) {
        rowid -> Integer,
        vendor_id -> Integer,
        order_id -> Integer,
    }
}

diesel::table! {
    vendor_user (rowid) {
        rowid -> Integer,
        vendor_id -> Integer,
        user_id -> Integer,
    }
}

diesel::table! {
    vendors (id) {
        id -> Integer,
        name -> Text,
        hub_id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(customers -> price_levels (price_level_id));
diesel::joinable!(order_products -> orders (order_id));
diesel::joinable!(orders -> customers (customer_id));
diesel::joinable!(product_images -> products (product_id));
diesel::joinable!(product_price_levels -> price_levels (price_level_id));
diesel::joinable!(product_price_levels -> products (product_id));
diesel::joinable!(product_tags -> products (product_id));
diesel::joinable!(product_tags -> tags (tag_id));
diesel::joinable!(products -> categories (category_id));
diesel::joinable!(products -> vendors (vendor_id));
diesel::joinable!(vendor_order -> orders (order_id));
diesel::joinable!(vendor_order -> vendors (vendor_id));
diesel::joinable!(vendor_user -> users (user_id));
diesel::joinable!(vendor_user -> vendors (vendor_id));

diesel::allow_tables_to_appear_in_same_query!(
    categories,
    customers,
    order_products,
    orders,
    price_levels,
    product_fts,
    product_fts_config,
    product_fts_data,
    product_fts_docsize,
    product_fts_idx,
    product_images,
    product_price_levels,
    product_tags,
    products,
    tags,
    users,
    vendor_order,
    vendor_user,
    vendors,
);
