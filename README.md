# pushkind-orders

`pushkind-orders` is the Pushkind hub service for browsing customer orders and the supporting price levels that drive storefront pricing. It ships with
server-rendered management pages, paginated data helpers, and a Diesel-backed persistence layer.
The project is implemented in Rust on top
of Actix Web, Diesel, and Tera and integrates tightly with the shared
`pushkind-common` crate for authentication, configuration, and reusable UI helpers.

## Features

- **Role-gated order dashboard** – Hub members with `SERVICE_ACCESS_ROLE` can browse their orders with pagination, search, statuses, totals, and captured timestamps.
- **Order domain snapshots** – Orders retain product snapshots (name, SKU, quantity, price, currency) so historical totals remain consistent when catalog data changes.
- **Customer management** – Track customers with phone-based authentication, assign price levels, and maintain customer-specific pricing for storefront orders.
- **Storefront API** – REST endpoints for customer-facing product browsing, OTP authentication, and order placement with customer-specific pricing.
- **Price level directory** – `/price-levels` lists named price tiers with search and pagination to help operators audit configured pricing ladders.
- **Shared Pushkind scaffolding** – Navigation, flash messaging, auth guards, and pagination helpers come from `pushkind-common` for a consistent UX across services.
- **Diesel-backed persistence layer** – Repository traits and a `DieselRepository` implementation span orders, products, price levels, customers, and users for reuse in services and tests.
- **SMS integration** – ZeroMQ-based event publishing for OTP delivery and other SMS notifications.

## Pages

- **Main page** – Browse existing orders with pagination, search, and filters. Selecting an order opens a modal window that shows the order details without leaving the list.
- **Products page** – Review products with search, filters, and pagination. Operators can create individual products, batch upload catalog entries, and open a modal to edit or delete a selected product.
- **Categories page** – Manage product categories with inline actions to browse, create, rename, and delete entries.
- **Prices page** – Inspect and maintain product price levels, including creating, renaming, and deleting tiers. Assign price levels to clients. Each assignment requires approval from a user with the `orders_manager` role, and clients can only view price levels that have been granted to them.
- **Tags page** – Manage product tags with inline actions to browse, create, rename, and delete entries.
- **Not assigned page** – Displayed to authenticated users who lack proper hub assignment.

## Store API

The service exposes REST endpoints under `/api/v1/store/{hub_id}` for customer-facing storefronts:

- **Product browsing** – `GET /products` and `GET /products/{product_id}` return products with customer-specific pricing based on assigned price levels.
- **Category and tag listing** – `GET /categories` and `GET /tags` support storefront navigation and filtering.
- **OTP authentication** – `POST /auth/otp` requests a one-time password sent via SMS, and `POST /auth/otp/verify` establishes a customer session.
- **Order management** – `POST /orders` creates orders for authenticated customers, and `GET /orders` lists their order history.

Store sessions are managed separately from hub user sessions using dedicated cookie-based storage.

## Architecture at a Glance

The codebase follows a clean, layered structure so that business logic can be
exercised and tested without going through the web framework:

- **Domain (`src/domain`)** – Type-safe models for orders, products, price levels,
  customers, users, categories, tags, and OTP records with builders for create/update
  payloads and query helpers to support paginated lookups.
- **Repository (`src/repository`)** – Traits that describe the persistence
  contract and a Diesel-backed implementation (`DieselRepository`) that speaks to
  a SQLite database. Each module translates between Diesel models and domain
  types and exposes strongly typed query builders.
- **Services (`src/services`)** – Application use-cases that orchestrate domain
  logic, repository traits, and Pushkind authentication helpers. Services return
  `ServiceResult<T>` and map infrastructure errors into well-defined service
  errors.
- **DTOs (`src/dto`)** – Data transfer objects for rendering templates and API
  responses. Services convert domain types to DTOs before handing data to routes,
  keeping handlers thin and domain models focused.
- **Forms (`src/forms`)** – `serde`/`validator` powered structs that handle
  request payload validation, CSV parsing, and transformation into domain types.
- **Routes (`src/routes`)** – Actix Web handlers that wire HTTP requests into the
  service layer and render Tera templates, return JSON responses, or redirect with
  flash messages.
- **Templates (`templates/`)** – Server-rendered UI built with Tera and
  Bootstrap 5, backed by sanitized HTML rendered via `ammonia` when necessary.

Because the repository traits live in `src/repository/mod.rs`, service functions
accept generic parameters that implement those traits. This makes unit tests easy
by swapping in the `mockall`-based fakes from `src/repository/mock.rs`.

## Technology Stack

- Rust 2024 edition
- [Actix Web](https://actix.rs/) with identity, session, and flash message
  middleware
- [Diesel](https://diesel.rs/) ORM with SQLite and connection pooling via r2d2
- [Tera](https://tera.netlify.app/) templates styled with Bootstrap 5.3
- [`pushkind-common`](https://github.com/pushkindt/pushkind-common) shared crate
  for authentication guards, configuration, database helpers, and reusable
  patterns
- Supporting crates: `chrono`, `validator`, `serde`, `ammonia`, `csv`,
  `thiserror`, `zmq` (ZeroMQ), `phonenumber`, and `mockall` (testing)

## Getting Started

### Prerequisites

- Rust toolchain (install via [rustup](https://www.rust-lang.org/tools/install))
- `diesel-cli` with SQLite support (`cargo install diesel_cli --no-default-features --features sqlite`)
- SQLite 3 installed on your system

### Configuration

Settings are layered via the [`config`](https://crates.io/crates/config) crate in the following order (later entries override earlier ones):

1. `config/default.yaml` (checked in)
2. `config/{APP_ENV}.yaml` where `APP_ENV` defaults to `local`
3. Environment variables prefixed with `APP_` (loaded automatically from a `.env` file via `dotenvy`)

Key settings you may want to override:

| Environment variable | Description | Default |
| --- | --- | --- |
| `APP_SECRET` | 64-byte secret used to sign cookies and flash messages | _required_ |
| `APP_DATABASE_URL` | Path to the SQLite database file | `app.db` |
| `APP_ADDRESS` | Interface to bind | `127.0.0.1` |
| `APP_PORT` | HTTP port | `80` (override to `8080` in local.yaml) |
| `APP_DOMAIN` | Cookie domain (without protocol) | _required_ |
| `APP_TEMPLATES_DIR` | Glob pattern for templates consumed by Tera | `templates/**/*` |
| `APP_ZMQ_SMS_PUB` | ZeroMQ PUB endpoint for outgoing SMS events | `tcp://127.0.0.1:5561` |
| `APP_SMS_SENDER` | Sender identifier for outbound SMS messages | `cns.shared` |
| `APP_AUTH_SERVICE_URL` | URL of the Pushkind authentication service | _required_ |
| `APP_CRM_SERVICE_URL` | URL of the Pushkind CRM service | _required_ |

Switch to the production profile with `APP_ENV=prod` or provide your own
`config/{env}.yaml`. Environment variables always win over YAML values, so a
local `.env` file containing `APP_SECRET=<64-byte key>` (generate with
`openssl rand -base64 64`) and any overrides will take effect without changing
the checked-in config files.

### Database

Run the Diesel migrations before starting the server:

```bash
diesel setup
cargo install diesel_cli --no-default-features --features sqlite # only once
diesel migration run
```

A SQLite file will be created at the location given by `DATABASE_URL`.

## Running the Application

Start the HTTP server with:

```bash
cargo run
```

The server listens on `http://127.0.0.1:8080` by default and serves static
assets from `./assets` in addition to the Tera-powered HTML pages. Authentication
and authorization are enforced via the Pushkind auth service and the
`SERVICE_ACCESS_ROLE` constant.

## Quality Gates

The project treats formatting, linting, and tests as required gates before
opening a pull request. Use the following commands locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-features --tests -- -Dwarnings
cargo test --all-features --verbose
cargo build --all-features --verbose
```

Alternatively, the `make check` target will format the codebase, run clippy, and
execute the test suite in one step.

## Testing

Unit tests exercise the service and form layers directly, while integration
tests live under `tests/`. Repository tests rely on Diesel’s query builders and
should avoid raw SQL strings whenever possible. Use the mock repository module to
isolate services from the database when writing new tests.

## Project Principles

- **Domain-driven**: keep business rules in the domain and service layers and
  translate to/from external representations at the boundaries.
- **Boundary sanitation**: perform validation and normalization (like email
  lowercasing) in forms/services so domain structs stay pure data.
- **Explicit errors**: use `thiserror` to define granular error types and convert
  them into `ServiceError`/`RepositoryError` variants instead of relying on
  `anyhow`.
- **No panics in production paths**: avoid `unwrap`/`expect` in request handlers,
  services, and repositories—propagate errors instead.
- **Security aware**: sanitize any user-supplied HTML using `ammonia`, validate
  inputs with `validator`, and always enforce role checks with
  `pushkind_common::routes::ensure_role`.
- **Testable**: accept traits rather than concrete types in services and prefer
  dependency injection so the mock repositories can be used in tests.
- **HTMX edit modals**: implement edit modals as dedicated routes that return a
  template fragment, and inject the result into the modal container via htmx
  (rather than inlining edit forms on the index page).

Following these guidelines will help new functionality slot seamlessly into the
existing architecture and keep the service reliable in production.
