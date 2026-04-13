# pushkind-orders

`pushkind-orders` is the Pushkind hub service for browsing customer orders and the supporting price levels that drive storefront pricing. Its hub UI is served from Vite-built React documents while the backend continues to own routing, authentication, authorization, validation, pricing, and persistence. The project is implemented in Rust on top of Actix Web and Diesel and integrates tightly with the shared `pushkind-common` crate for authentication, configuration, and reusable UI helpers.

## Docs

- `SPEC.md` — authoritative behavior spec (routes, invariants, DTOs, idempotency, failure modes).
- `AGENTS.md` — contributor guidance (architecture conventions, coding standards, local workflow).

## Features

- Hub operator pages for orders, products, categories, tags, price levels, and vendors.
- Store API under `/api/v1/store/{hub_id}` for product browsing, order history, and authenticated order placement using the CRM-issued `store-session` cookie.
- Customer-specific pricing via price levels (default + per-customer overrides).
- Vendor-scoped access for `orders_vendor` users and vendor/user assignment tools for `orders_admin`.

## Getting Started

### Prerequisites

- Rust toolchain (install via [rustup](https://www.rust-lang.org/tools/install))
- `diesel-cli` with SQLite support (`cargo install diesel_cli --no-default-features --features sqlite`)
- SQLite 3 installed on your system

### Configuration

Settings are layered via the [`config`](https://crates.io/crates/config) crate in the following order (later entries override earlier ones):

1. `config/default.yaml` (checked in)
2. `config/{APP_ENV}.yaml` where `APP_ENV` defaults to `local`
3. Environment variables prefixed with `APP_` (loaded automatically from a `.env` file via `dotenvy`)

Key settings you may want to override:

| Environment variable | Description | Default |
| --- | --- | --- |
| `APP_SECRET` | 64-byte secret used to sign cookies | _required_ |
| `APP_DATABASE_URL` | Path to the SQLite database file | `app.db` |
| `APP_ADDRESS` | Interface to bind | `127.0.0.1` |
| `APP_PORT` | HTTP port | `80` (override to `8080` in `config/local.yaml`) |
| `APP_DOMAIN` | Cookie domain (without protocol) | _required_ |
| `APP_AUTH_SERVICE_URL` | URL of the Pushkind authentication service | _required_ |
| `APP_CRM_SERVICE_URL` | URL of the Pushkind CRM service | _required_ |

### Database

Run the Diesel migrations before starting the server:

```bash
diesel setup
diesel migration run
```

### Running the Application

Start the HTTP server with:

```bash
cargo run
```

The server listens on `http://127.0.0.1:8080` by default (with `APP_ENV=local`) and serves static assets from `./assets` in addition to its React hub pages.

## Development

### Frontend Assets

The hub UI serves `GET /`, `GET /na`, `GET /order/{order_id}`,
`GET /products`, `GET /categories`, `GET /tags`, `GET /price-levels`, and
`GET /vendors` from built React documents.

Install frontend dependencies with:

```bash
cd frontend
npm install
```

Build frontend assets with:

```bash
cd frontend
npm run build
```

The production build output is written to `assets/dist/`.

`cargo run` can still start the backend without a
frontend build, but `GET /`, `GET /na`, `GET /order/{order_id}`,
`GET /products`, `GET /categories`, `GET /tags`, `GET /price-levels`, and
`GET /vendors`
now depend on the built frontend documents at `assets/dist/app/index.html`,
`assets/dist/app/no-access.html`, `assets/dist/app/order.html`,
`assets/dist/app/products.html`, `assets/dist/app/categories.html`,
`assets/dist/app/tags.html`, `assets/dist/app/price-levels.html`, and
`assets/dist/app/vendors.html`.

If the frontend build has not been run yet, the service keeps working, but
`GET /`, `GET /na`, `GET /order/{order_id}`, `GET /products`,
`GET /categories`, `GET /tags`, `GET /price-levels`, and `GET /vendors`
return a clear
`503 Service Unavailable` response telling you to run
`cd frontend && npm run build`.

The React index page initializes from typed resource APIs:

- `GET /api/v1/iam`
- `GET /api/v1/orders`
- `GET /api/v1/orders/{order_id}`
- `PUT /api/v1/orders/{order_id}`
- `PUT /api/v1/orders/{order_id}/products/approvals`
- `GET /api/v1/products`
- `GET /api/v1/products/{product_id}`
- `POST /api/v1/products`
- `PUT /api/v1/products/{product_id}`
- `POST /api/v1/products/upload`
- `GET /api/v1/categories`
- `GET /api/v1/categories/{category_id}`
- `POST /api/v1/categories`
- `PUT /api/v1/categories/{category_id}`
- `DELETE /api/v1/categories/{category_id}`
- `GET /api/v1/tags`
- `GET /api/v1/tags/{tag_id}`
- `POST /api/v1/tags`
- `PUT /api/v1/tags/{tag_id}`
- `DELETE /api/v1/tags/{tag_id}`
- `GET /api/v1/price-levels`
- `GET /api/v1/price-levels/{price_level_id}`
- `POST /api/v1/price-levels`
- `PUT /api/v1/price-levels/{price_level_id}`
- `DELETE /api/v1/price-levels/{price_level_id}`
- `GET /api/v1/client-price-levels`
- `PUT /api/v1/client-price-levels`
- `GET /api/v1/vendors`
- `GET /api/v1/vendors/{vendor_id}`
- `POST /api/v1/vendors`
- `PUT /api/v1/vendors/{vendor_id}`
- `DELETE /api/v1/vendors/{vendor_id}`
- `GET /api/v1/users`
- `POST /api/v1/users`
- `POST /api/v1/vendors/assignments`
- `DELETE /api/v1/vendors/assignments/{user_id}`

The migration direction is explicitly resource-style. New React GET endpoints
must stay under `/api/v1/...` and must not drift into page-shaped bootstrap
contracts.

Use `AGENTS.md` for the full local workflow and quality gates. The quick “do everything” command is:

```bash
make check
```
