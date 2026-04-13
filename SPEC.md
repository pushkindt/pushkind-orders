# pushkind-orders — Specification

## Purpose

`pushkind-orders` is the Pushkind hub service for:

- Hub-operator management pages for browsing and maintaining orders and catalog data.
  `GET /`, `GET /na`, `GET /order/{order_id}`, `GET /products`,
  `GET /categories`, `GET /tags`, `GET /price-levels`, and `GET /vendors`
  are served from Vite-built React documents.
- Customer-facing Store API (`/api/v1/store/{hub_id}`) for product browsing, JWT-authenticated customer access, and order placement.

The system is implemented as an Actix Web application with Diesel (SQLite) and follows a layered architecture (domain → repository → services → routes).

## Storefront Auth Contract

The feature spec [crm-direct-store-auth.md](/home/matrizaev/pushkind/specs/features/crm-direct-store-auth.md)
defines the current storefront authentication split.

- `pushkind-crm` is the owner of storefront auth endpoints and issuer of the browser `store-session` cookie.
- Orders remains the owner of storefront catalog, pricing, and order endpoints.
- Orders validates a dedicated storefront JWT from `store-session` and resolves a local customer from the claims.
- The storefront JWT claim shape is:
  - `sub`: CRM client `public_id` UUID string
  - `hub_id`: hub identifier
  - `name`: client display name
  - `phone`: normalized E.164 phone number
  - `email`: optional client email
  - `exp`: expiration timestamp

## Actors and Roles

### Hub user (operator)

- Authenticated via `pushkind-common` identity/session middleware and the Pushkind auth service (`AppConfig.auth_service_url`).
- Must have `SERVICE_ACCESS_ROLE` (`src/lib.rs`: `orders`) to access hub pages and `/api/v1/*` JSON endpoints.
- Administrative actions require `ADMIN_ACCESS_ROLE` (`src/lib.rs`: `orders_admin`).
- Only hub operators with `ADMIN_ACCESS_ROLE` can create vendors and assign users to vendors.
- Users lacking access are redirected to the local `/na` page owned by `pushkind-orders`.

### Vendor hub user

- Authenticated via the same hub identity/session middleware as hub operators.
- Must have `SERVICE_ACCESS_ROLE` (`src/lib.rs`: `orders`) and `VENDOR_ACCESS_ROLE` (`src/lib.rs`: `orders_vendor`).
- Must be assigned to exactly one `Vendor` (see `vendor_user` under the data model).
- Cannot create vendors or assign/unassign users to vendors.
- Authorization is vendor-scoped:
  - Vendor users can only see products that are associated with their vendor (`products.vendor_id`).
  - Vendor users can only see orders associated with their vendor (orders are linked to vendors via `vendor_order` when they contain vendor-owned products).
- Vendor users have read-only access to hub-wide catalog configuration:
  - Can view tags, categories, and price levels.
  - Cannot create/update/delete these; write actions are restricted to hub operators with `ADMIN_ACCESS_ROLE`.

### Store customer (end user)

- Authenticates via `pushkind-crm`, which issues the `store-session` browser cookie.
- Orders treats `store-session` as a JWT and authorizes storefront requests only when the token `hub_id` matches the requested hub.
- Orders resolves the local `Customer` record by JWT `sub` (`public_id`) and may fall back to phone-based lookup during migration of legacy customer records.

## High-Level Architecture

- `src/domain`: strong types and domain entities (`Order`, `Product`, `PriceLevel`, `Customer`, `Vendor`, etc.).
- `src/repository`: repository traits (Reader/Writer) and the Diesel-backed `DieselRepository` implementation.
- `src/services`: business logic (authorization checks, pricing rules, vendor scoping, etc.); returns `ServiceResult<T>`.
- `src/routes`: Actix handlers; extract inputs, call services, serve built frontend HTML, or return JSON/redirect.
- `migrations/` + `src/schema.rs`: SQLite schema managed via Diesel migrations.

## Configuration

Configuration is loaded by `config` in this order (`src/main.rs`):

1. `config/default.yaml`
2. `config/{APP_ENV}.yaml` (optional; `APP_ENV` defaults to `local`)
3. Environment variables with prefix `APP_` (loaded from `.env` via `dotenvy` in local dev)

Key settings (`src/models/config.rs`):

- `APP_SECRET` (required): cookie signing key material.
- `APP_DOMAIN` (required): cookie domain; sessions set `.{domain}`.
- `APP_ADDRESS` (default `127.0.0.1`), `APP_PORT` (default `80`, local `8080`).
- `APP_DATABASE_URL` (default `app.db`): SQLite file path.
- `APP_AUTH_SERVICE_URL`, `APP_CRM_SERVICE_URL`: upstream service URLs used for navigation and links.

## Data Model (SQLite)

Primary tables (SQLite schema; see `src/schema.rs` and `migrations/`):

- `orders` + `order_products`: orders with captured product snapshots (name/SKU/description/price/currency/quantity).
- `products`: catalog items; `is_archived` hides items from store; optional `vendor_id` associates a product with the vendor that created/owns it.
- `product_price_levels`: price overrides per (product, price_level).
- `price_levels`: named pricing tiers; one can be marked `is_default` per hub.
- `customers`: phone-based customers; may have `price_level_id` and optional `public_id`.
- `categories` (tree via `parent_id`) and `tags` (+ join tables).
- `vendors`: vendor entities per hub.
- `vendor_user`: assignment of hub users to vendors (one vendor per user).
- `vendor_order`: association between a vendor and an order (derived from order line items that reference vendor-owned products; an order may be linked to at most one vendor).
- `product_fts*`: SQLite FTS structures for catalog search (used by repository query builders).

## Domain Invariants

This section states invariants as *hard rules* when they are enforced by code/DB, and calls out where behavior is merely *conventional*.

### Tenanting and identity

- **Hub scoping (hard rule)**: core records are scoped by `hub_id` (orders/products/categories/tags/price levels/customers/users).
- **Customer phone uniqueness (hard rule)**: phone numbers are unique **per hub** (`UNIQUE(hub_id, phone)`), and Store API normalizes inputs to E.164 before lookup/creation.
- **Vendor scoping (required behavior)**: vendors are scoped by `hub_id`, and vendor assignments/associations (`vendor_user`, `vendor_order`, `products.vendor_id`) must not cross hub boundaries.
- **Vendor user cardinality (required behavior)**: a hub user may be assigned to **at most one** vendor; users with `VENDOR_ACCESS_ROLE` must have exactly one vendor assignment.
- **Order vendor constraint (required behavior)**: an order must not contain products from multiple vendors. An order either:
  - contains only vendorless products (`products.vendor_id IS NULL`) and has no vendor association, or
  - contains only products from a single vendor and is associated with exactly that vendor.

### Uniqueness constraints (hard rules)

- **Users**: `(hub_id, email)` is unique.
- **Vendors**: `(hub_id, name)` is unique.
- **Products**: `(hub_id, sku)` is unique when `sku` is non-null.
- **Categories**: `(hub_id, parent_id, name)` is unique (same-name siblings are prevented).
- **Tags**: `(hub_id, name)` is unique.
- **Price levels**: `(hub_id, name)` is unique.
- **Orders**: `(hub_id, reference)` is unique when `reference` is non-null.
- **Product price levels**: `(product_id, price_level_id)` is unique.
- **Vendor users**: `(vendor_id, user_id)` is unique.
- **Vendor orders**: `(vendor_id, order_id)` is unique.

### Referential actions (hard rules; foreign keys enabled)

The DB connection pool enables SQLite `PRAGMA foreign_keys = ON` (and WAL) on every acquire via `pushkind_common::db::establish_connection_pool`.

- **Price level deletion**:
  - `customers.price_level_id` is `ON DELETE SET NULL`
  - `product_price_levels.price_level_id` is `ON DELETE CASCADE`
  - Consequence: deleting a price level is allowed even if referenced; it clears customers and removes product rates.
- **Customer deletion**: `orders.customer_id` is `ON DELETE SET NULL`.
- **Category deletion**:
  - `categories.parent_id` is `ON DELETE SET NULL`
  - `products.category_id` is `ON DELETE SET NULL`
  - Consequence: deleting a category is allowed even if it has children or products; children become top-level and products become uncategorized.
- **Order deletion**: `order_products.order_id` is `ON DELETE CASCADE`.
- **Product deletion**: product-related join tables (`product_tags`, `product_images`, `product_price_levels`) cascade; `order_products.product_id` is *not* a foreign key (orders keep snapshots even if products are removed).

### Price level defaults (mixed: partly enforced)

- **At most one default per hub (hard rule)**: when a price level is updated with `is_default=true`, the repository clears `is_default` for all other levels in the hub.
- **At least one default per hub (convention, not guaranteed)**: the first created price level becomes default automatically, but deleting the default can leave a hub with no default price level.
  - Store pricing then may yield `price unavailable` (e.g., order creation) unless a customer-specific price level is present and applicable.

### Orders, approvals, and mutability (current behavior)

- **Order line “approval” is mutable**: `approved_quantity` (and derived line totals) can be updated repeatedly via `PUT /api/v1/orders/{order_id}/products/approvals`; there is no status-based lock.
- **Order metadata is mutable**:
  - Hub users can update status/notes/reference/shipping fields via `PUT /api/v1/orders/{order_id}` regardless of current status.
  - Store customers can update only shipping-related fields via `PATCH /api/v1/store/{hub_id}/orders/{order_id}`; there is no status-based lock.
- **Order product snapshots are stable vs catalog changes**: order lines store name/SKU/description/currency as snapshots; catalog edits do not rewrite historical lines. (However, approvals *do* change `approved_quantity` and `price_cents` snapshots.)

## Order Lifecycle (Optional / Current State)

### Status values (hard rule)

Order statuses are constrained by both domain parsing and a DB `CHECK` constraint:

- `Draft`
- `Pending`
- `Processing`
- `Completed`
- `Cancelled`

### Transitions (not enforced)

There is currently no explicit state machine enforcing allowed transitions:

- Hub users can set `status` via `PUT /api/v1/orders/{order_id}` without transition validation.
- Store customers cannot change `status` via the Store API.

### Terminal states (convention)

The system does not prevent edits in any state, but operationally:

- `Completed` and `Cancelled` are typically treated as terminal in downstream workflows.
- If a formal state machine is introduced, specify allowed transitions and “locked fields” here (for example, disallow approvals/metadata edits after `Completed`).

## Hub UI

Hub pages require an authenticated hub user.

- Full hub-operator access requires `ADMIN_ACCESS_ROLE`.
- Vendor access requires `VENDOR_ACCESS_ROLE` and is limited to vendor-associated products and orders.
- Vendor users can view tags, categories, and price levels read-only; create/update/delete actions require `ADMIN_ACCESS_ROLE`.

### Routes

- `GET /` — React-owned orders dashboard document. The page fetches its data from resource-style JSON endpoints after load.
- `GET /na` — React-owned local no-access page for authenticated users without the `orders` role.
- `GET /order/{order_id}` — React-owned order details document. The backend performs lightweight auth and resource existence checks, then serves the built `app/order.html`.

- `GET /products` — React-owned product management document. The backend performs lightweight auth and serves the built `app/products.html`; the page fetches typed resource data from `/api/v1/products`.

- `GET /categories` — React-owned category management document. The backend performs lightweight auth and serves the built `app/categories.html`.

- `GET /tags` — React-owned tag management document. The backend performs lightweight auth and serves the built `app/tags.html`.

- `GET /price-levels` — React-owned price-level management document. The backend performs lightweight auth and serves the built `app/price-levels.html`.

- `GET /vendors` — React-owned vendor management document. The backend performs lightweight auth and serves the built `app/vendors.html`.

Static assets are served from `GET /assets/*` (folder `./assets`).

The React migration direction for full-page routes is:

- Vite-built static HTML documents served after backend auth and authorization checks
- typed JSON initialization via `/api/v1/...`
- reusable resource-style GET contracts rather than page-shaped bootstrap endpoints

## Hub JSON API (`/api/v1/*`)

All endpoints require an authenticated hub user (wrapped by `RedirectUnauthorized`).

- Full access requires `ADMIN_ACCESS_ROLE`.
- When accessed by a vendor user (`VENDOR_ACCESS_ROLE`), product/order endpoints must return only vendor-associated data.
- Vendor users may read hub-wide configuration, but must not be able to create/update/delete tags, categories, or price levels.
- React-facing GET endpoints must follow resource-style contracts. Page-shaped bootstrap endpoints such as `/api/v1/index` are not part of the target contract.

- `GET /api/v1/iam` — shell data for React-owned pages.
  - intentionally available to authenticated users even without `orders`, because the local `/na` page also uses the shell
- `GET /api/v1/no-access` — local content for the React-owned `/na` page.
- `GET /api/v1/vendors` — canonical typed vendor collection.
  - query model supports `search` and `page`
  - response shape is resource-style:
    - `items`
    - `pagination`
    - `active_filters`
- `GET /api/v1/vendors/{vendor_id}` — canonical typed vendor details resource.
- `POST /api/v1/vendors` — canonical typed vendor create mutation.
  - request body matches the strongly typed add-vendor form contract
  - `200 OK` returns `{ message, vendor }`
- `PUT /api/v1/vendors/{vendor_id}` — canonical typed vendor update mutation.
  - request body matches the strongly typed edit-vendor form contract
  - `200 OK` returns `{ message, vendor }`
- `DELETE /api/v1/vendors/{vendor_id}` — canonical typed vendor delete mutation.
  - `200 OK` returns `{ message }`
- `GET /api/v1/users` — canonical typed local-user collection for vendor assignment management.
- `POST /api/v1/users` — canonical typed local-user creation mutation for vendor users managed inside Orders.
  - request body matches the strongly typed add-user form contract
  - `200 OK` returns `{ message }`
- `POST /api/v1/vendors/assignments` — canonical typed user-to-vendor assignment mutation.
  - request body matches the strongly typed assign-vendor-user form contract
  - `200 OK` returns `{ message }`
- `DELETE /api/v1/vendors/assignments/{user_id}` — canonical typed vendor-assignment clear mutation.
  - `200 OK` returns `{ message }`
- `GET /api/v1/orders` — canonical typed order collection.
  - query model currently supports `search` and `page`
  - response shape is resource-style:
    - `items`
    - `pagination`
    - `active_filters`
- `GET /api/v1/orders/{order_id}` — canonical typed order details resource used by the React order page.
- `PUT /api/v1/orders/{order_id}` — canonical typed order metadata mutation.
  - request body matches the strongly typed edit-order form contract
  - `200 OK` returns `{ message, order }`
  - `422 Unprocessable Entity` returns `{ message, field_errors }`
- `PUT /api/v1/orders/{order_id}/products/approvals` — canonical typed approvals mutation.
  - request body matches the strongly typed approvals form contract
  - `200 OK` returns `{ message, order }`
  - `422 Unprocessable Entity` returns `{ message, field_errors }`
- `GET /api/v1/products` — canonical typed product collection.
  - query model currently supports `search`, `page`, and `show_archived`
  - response shape is resource-style:
    - `items`
    - `pagination`
    - `active_filters`
    - `editor_options`
- `GET /api/v1/products/{product_id}` — canonical typed product details resource used by the React products page.
- `POST /api/v1/products` — canonical typed create-product mutation.
  - request body matches the strongly typed add-product form contract
  - `200 OK` returns `{ message, product }`
  - `422 Unprocessable Entity` returns `{ message, field_errors }`
- `PUT /api/v1/products/{product_id}` — canonical typed edit-product mutation.
  - request body matches the strongly typed edit-product form contract
  - `200 OK` returns `{ message, product }`
  - `422 Unprocessable Entity` returns `{ message, field_errors }`
- `POST /api/v1/products/upload` — canonical typed CSV upload mutation.
  - request body is multipart and matches the strongly typed upload form contract
  - `200 OK` returns `{ message, created_count }`
  - `422 Unprocessable Entity` returns `{ message, field_errors }`
- `GET /api/v1/categories` — canonical typed category tree resource for the React categories page.
- `GET /api/v1/categories/{category_id}` — canonical typed category details resource.
- `POST /api/v1/categories` — canonical typed category creation mutation.
  - `200 OK` returns `{ message, category }`
  - `422 Unprocessable Entity` returns `{ message, field_errors }`
- `PUT /api/v1/categories/{category_id}` — canonical typed category update mutation.
  - `200 OK` returns `{ message, category }`
  - `422 Unprocessable Entity` returns `{ message, field_errors }`
- `DELETE /api/v1/categories/{category_id}` — canonical typed category deletion mutation.
  - `200 OK` returns `{ message }`
- `GET /api/v1/tags` — canonical typed tag collection resource for the React tags page.
- `GET /api/v1/tags/{tag_id}` — canonical typed tag details resource.
- `POST /api/v1/tags` — canonical typed tag creation mutation.
  - `200 OK` returns `{ message, tag }`
  - `422 Unprocessable Entity` returns `{ message, field_errors }`
- `PUT /api/v1/tags/{tag_id}` — canonical typed tag update mutation.
  - `200 OK` returns `{ message, tag }`
  - `422 Unprocessable Entity` returns `{ message, field_errors }`
- `DELETE /api/v1/tags/{tag_id}` — canonical typed tag deletion mutation.
  - `200 OK` returns `{ message }`
- `GET /api/v1/price-levels` — canonical typed price-level collection resource for the React price-levels page.
- `GET /api/v1/price-levels/{price_level_id}` — canonical typed price-level details resource.
- `POST /api/v1/price-levels` — canonical typed price-level creation mutation.
  - `200 OK` returns `{ message, price_level }`
  - `422 Unprocessable Entity` returns `{ message, field_errors }`
- `PUT /api/v1/price-levels/{price_level_id}` — canonical typed price-level update mutation.
  - `200 OK` returns `{ message, price_level }`
  - `422 Unprocessable Entity` returns `{ message, field_errors }`
- `DELETE /api/v1/price-levels/{price_level_id}` — canonical typed price-level deletion mutation.
  - `200 OK` returns `{ message }`
- `GET /api/v1/client-price-levels` — list hub customers with assigned `price_level_id` (plus hub default level).
- `PUT /api/v1/client-price-levels` — assign or clear a customer’s price level by phone.
  - `200 OK` returns `{ message }`
  - `422 Unprocessable Entity` returns `{ message, field_errors }`

## Store API (`/api/v1/store/{hub_id}/*`)

Store endpoints use a dedicated cookie session (`store-session`) and are CORS-permissive.

### Session

- `GET /api/v1/store/{hub_id}/auth/session`
  - `200 OK` with the persisted `Customer` if the session is valid and the customer still exists in DB
  - `401 Unauthorized` if no session exists or it is invalid for the hub

### Catalog

These endpoints do not require authentication, but apply customer-specific pricing if a valid store session exists.

- `GET /api/v1/store/{hub_id}/vendors`
  - Returns `200 OK` with `Vec<StoreVendor { id, name }>`
- `GET /api/v1/store/{hub_id}/products`
  - Query params (camelCase): `categoryId`, `tagId`, `vendorId`, `search`, `page`
  - `vendorId` filters products by their associated vendor (`products.vendor_id`); vendor filtering is by id (not by vendor name)
  - Returns `200 OK` with `Vec<StoreProduct>` (see DTO rules below)
- `GET /api/v1/store/{hub_id}/products/{product_id}`
  - Returns `200 OK` with `StoreProduct`, `404 Not Found` if the product is not available
- `GET /api/v1/store/{hub_id}/categories`
  - Optional query param: `parentId`
  - Returns top-level categories when `parentId` is omitted
- `GET /api/v1/store/{hub_id}/tags`

### Storefront authentication

- `pushkind-orders` does not own storefront login endpoints.
- Storefront authentication is performed by `pushkind-crm`, which issues the `store-session` cookie described in the Storefront Auth Contract.
- Store API endpoints in orders either:
  - work anonymously for product browsing, or
  - require a valid `store-session` cookie for customer-specific order history and mutations.

### Orders

These endpoints require an authenticated store session.

- `POST /api/v1/store/{hub_id}/orders`
  - Body: `Vec<StoreOrderLinePayload>` (`{ productId, quantity }`-style payload)
  - Returns:
    - `201 Created` with `StoreOrder`
    - `401 Unauthorized` when not authenticated
    - `422` with `{ "error": "..." }` for invalid payloads (unknown product, missing price, non-positive quantity, mixed currencies, etc.)
  - Business rules:
    - Only non-archived products may be ordered.
    - A single order cannot contain products with mixed currencies.
    - A single order must not contain products from multiple vendors:
      - all line items must be vendorless, or
      - all line items must belong to the same vendor.
    - Line totals are computed and stored as snapshots; later catalog edits do not affect order totals.

- `GET /api/v1/store/{hub_id}/orders`
  - Query param: `page`
  - Returns `200 OK` with `Vec<StoreOrder>`

- `PATCH /api/v1/store/{hub_id}/orders/{order_id}`
  - Body: `StoreOrderUpdatePayload` (editable metadata such as shipping fields)
  - Updates only orders belonging to the authenticated customer.
  - Returns `200 OK` with updated `StoreOrder`, `401`/`404` as appropriate, and `422` on validation error.

### Store pricing rules

Pricing is resolved from `product_price_levels` and `price_levels`:

- The hub default price level is the `price_levels.is_default == true` record (resolved by listing hub price levels).
- For store customers with `customer.price_level_id`:
  - Product listing is filtered to products that have a matching `product_price_levels.price_level_id`.
  - `StoreProduct.price_cents` uses the customer’s price level.
  - `StoreProduct.base_price_cents` uses the hub default price level; it is omitted (`null`) when it equals `price_cents`.
- Without a customer price level:
  - `StoreProduct.price_cents` falls back to the hub default price level (when available).

## API Versioning and Deprecation Policy

Current state:

- Store API is versioned in the path: `/api/v1/store/{hub_id}`.
- Hub JSON API also uses `/api/v1/*` routes.
- There is no in-band deprecation mechanism (no response headers or version negotiation) beyond the URL path.

Compatibility policy (recommended for future work):

- Treat `v1` as **additive-only**: adding fields/endpoints is allowed; removing/renaming/changing semantics is not.
- Breaking changes require a new versioned prefix (e.g. `/api/v2/store/{hub_id}`) with a migration window.
- Deprecate by:
  - keeping old fields functional until a stated sunset date, and
  - documenting replacements in the spec and changelog.

## DTO Conventions

- Store API DTOs live in `src/dto/store.rs` and use `camelCase` JSON fields.
- Numeric ids are serialized as `i32` values; timestamps use `chrono::NaiveDateTime`.

## Representative DTO Examples

This section sketches the key storefront DTOs and payload semantics for frontend/integration consumers.

### `StoreProduct` (response)

- Required: `id`, `name`, `currency`, `tags`, `imageUrls`, `updatedAt`
- Optional: `categoryId`, `vendorId`, `vendorName`, `sku`, `description`, `units`, `priceCents`, `basePriceCents`, `amount`

Example:

```json
{
  "id": 123,
  "categoryId": 10,
  "vendorId": 42,
  "vendorName": "Coffee Co",
  "name": "Coffee",
  "sku": "SKU-COFFEE",
  "description": "Ground coffee",
  "units": "pcs",
  "currency": "USD",
  "priceCents": 1250,
  "basePriceCents": null,
  "tags": [{ "id": 7, "name": "Organic" }],
  "imageUrls": ["https://cdn.example.com/coffee.jpg"],
  "updatedAt": "2026-01-14T15:58:00",
  "amount": 1.0
}
```

Notes:

- `priceCents` is the effective price for the requesting customer context; it can be `null` if no applicable price exists.
- `basePriceCents` is the hub default price when it differs from `priceCents`; otherwise it is `null`.
- `vendorId` / `vendorName` are `null` when a product is not associated with a vendor.
- `vendorName` is display-only; filtering is done via the `vendorId` query parameter.

### `StoreVendor` (response)

- Required: `id`, `name`

Example:

```json
{ "id": 42, "name": "Coffee Co" }
```

### `StoreOrder` (response)

- Required: `id`, `hubId`, `status`, `totalCents`, `currency`, `products`, `createdAt`, `updatedAt`
- Optional: `customerId`, `reference`, `notes`, `shippingAddress`, `consignee`, `deliveryNotes`, `payer`

Example:

```json
{
  "id": 99,
  "hubId": 1,
  "customerId": 10,
  "reference": null,
  "status": "Pending",
  "notes": null,
  "totalCents": 1300,
  "currency": "USD",
  "products": [
    {
      "productId": 1,
      "name": "Product 1",
      "sku": "SKU1",
      "description": "Description 1",
      "priceCents": 1000,
      "currency": "USD",
      "quantity": 2,
      "approvedQuantity": 2
    }
  ],
  "createdAt": "2026-01-14T15:58:00",
  "updatedAt": "2026-01-14T15:58:00",
  "shippingAddress": null,
  "consignee": null,
  "deliveryNotes": null,
  "payer": null
}
```

### Request payloads

Order creation:

- `POST /orders`: `[{ "productId": 1, "quantity": 2 }, { "productId": 2, "quantity": 1 }]`

Order updates (tri-state semantics):

- `PATCH /orders/{order_id}` supports `shippingAddress`, `consignee`, `deliveryNotes`, `payer`.
- Each field is `Option<Option<String>>` in JSON:
  - omitted: no change
  - `null`: clear existing value
  - string: set/replace value (after sanitization)

Example “clear consignee, set shippingAddress, keep others unchanged”:

```json
{ "shippingAddress": "New address", "consignee": null }
```

## Concurrency and Idempotency

Unless explicitly stated below, endpoints provide **no idempotency guarantees** (no idempotency keys).

- `POST /api/v1/store/{hub_id}/orders`:
  - **Not idempotent**: repeated requests create repeated orders.
  - Order creation is a single DB transaction (order + order lines).
- `PATCH /api/v1/store/{hub_id}/orders/{order_id}`:
  - Not explicitly versioned for concurrency (no ETag/If-Match); last write wins at the DB level.

## Error and Status Code Conventions (HTTP)

- Hub document routes:
  - Unauthorized users are redirected to `/na`.
  - Infrastructure errors generally return `500 Internal Server Error`.
- JSON APIs (`/api/*` and store API):
  - `400 Bad Request` for invalid path parameters (non-integer ids).
  - `401 Unauthorized` for missing/invalid auth context.
  - `404 Not Found` for missing entities.
  - `422 Unprocessable Entity` with `{ "error": "..." }` for validation errors.

## External Integrations

- **Auth service**: URL configured by `APP_AUTH_SERVICE_URL`; used by `pushkind-common` middleware for identity and by React shell data for navigation.
- **CRM service**: `APP_CRM_SERVICE_URL`; referenced from order/price-level pages for outbound links.

## Failure Modes and Operational Notes

- **SQLite locking**: the connection pool enables WAL and sets a busy timeout, but callers may still see `500` on persistent DB contention.

## Observability (Optional / Current State)

- **Logging**: Actix `Logger` middleware is enabled; application code uses the `log` crate (`error!`, `warn!`, `info!`) in routes/services for failures and key events.
- **Correlation IDs**: no request/trace correlation ID is currently injected or propagated (no `X-Request-Id`/`traceparent` handling specified).
- **Metrics**: no metrics endpoint or instrumentation is currently defined in this service.
- **Tracing (future)**: if needed, introduce structured logging and distributed tracing via `tracing` + a request-id middleware, and define a minimal metrics surface (request counts/latency, DB errors, order creation counts).

## Development and Quality Gates

Suggested commands (also in `AGENTS.md` / `README.md`):

```bash
cargo fmt --all -- --check
cargo clippy --all-features --tests -- -Dwarnings
cargo test --all-features --verbose
cargo build --all-features --verbose
```

## Testing Strategy (Existing)

- Unit tests exist for service-layer rules (including pricing and vendor-scoping logic) using mock repositories (`src/repository/mock.rs`).
- Integration tests in `tests/` run against a temporary SQLite DB with migrations applied (see `tests/common/mod.rs`).

## Rollout And Rollback

### Rollout Checklist

- Apply migrations (including `2026-01-22-103021_add-vendor`).
- Provision the `orders_vendor` role in the auth service.
- Create vendors and assign hub users.
- (Optional) Run vendor backfill for existing products/orders.
- Verify vendor users can only access their products/orders.

### Rollback Plan

- Stop traffic to the service or place it in maintenance mode.
- Back up the database before reversing migrations.
- Run the down migration for `2026-01-22-103021_add-vendor`.
- Rolling back removes `vendor_order`, `vendor_user`, and `products.vendor_id` data.
