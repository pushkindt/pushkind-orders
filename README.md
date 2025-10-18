# pushkind-orders

`pushkind-orders` is the Pushkind hub service for browsing customer orders and the supporting price levels that drive storefront pricing. It ships with
server-rendered management pages, paginated data helpers, and a Diesel-backed persistence layer.
The project is implemented in Rust on top
of Actix Web, Diesel, and Tera and integrates tightly with the shared
`pushkind-common` crate for authentication, configuration, and reusable UI helpers.

## Features

- **Role-gated order dashboard** – Hub members with `SERVICE_ACCESS_ROLE` can browse their orders with pagination, search, statuses, totals, and captured timestamps.
- **Order domain snapshots** – Orders retain product snapshots (name, SKU, quantity, price, currency) so historical totals remain consistent when catalog data changes.
- **Price level directory** – `/price-levels` lists named price tiers with search and pagination to help operators audit configured pricing ladders.
- **Shared Pushkind scaffolding** – Navigation, flash messaging, auth guards, and pagination helpers come from `pushkind-common` for a consistent UX across services.
- **Diesel-backed persistence layer** – Repository traits and a `DieselRepository` implementation span orders, products, price levels, and users for reuse in services and tests.

## Pages

- **Main page** – Browse existing orders with pagination, search, and filters. Selecting an order opens a modal window that shows the order details without leaving the list.
- **Products page** – Review products with search, filters, and pagination. Operators can create individual products, batch upload catalog entries, and open a modal to edit or delete a selected product.
- **Categories page** – Manage product categories with inline actions to browse, create, rename, and delete entries.
- **Prices page** – Inspect and maintain product price levels, including creating, renaming, and deleting tiers.
- **Discounts page** – Assign price levels to clients. Each assignment requires approval from a user with the `orders_manager` role, and clients can only view price levels that have been granted to them.

## Architecture at a Glance

The codebase follows a clean, layered structure so that business logic can be
exercised and tested without going through the web framework:

- **Domain (`src/domain`)** – Type-safe models for orders, products, price levels,
  and users with builders for create/update payloads and query helpers to support
  paginated lookups.
- **Repository (`src/repository`)** – Traits that describe the persistence
  contract and a Diesel-backed implementation (`DieselRepository`) that speaks to
  a SQLite database. Each module translates between Diesel models and domain
  types and exposes strongly typed query builders.
- **Services (`src/services`)** – Application use-cases that orchestrate domain
  logic, repository traits, and Pushkind authentication helpers. Services return
  `ServiceResult<T>` and map infrastructure errors into well-defined service
  errors.
- **Forms (`src/forms`)** – `serde`/`validator` powered structs that handle
  request payload validation, CSV parsing, and transformation into domain types.
- **Routes (`src/routes`)** – Actix Web handlers that wire HTTP requests into the
  service layer and render Tera templates or redirect with flash messages.
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
- Supporting crates: `chrono`, `validator`, `serde`, `ammonia`, `csv`, and
  `thiserror`

## Getting Started

### Prerequisites

- Rust toolchain (install via [rustup](https://www.rust-lang.org/tools/install))
- `diesel-cli` with SQLite support (`cargo install diesel_cli --no-default-features --features sqlite`)
- SQLite 3 installed on your system

### Environment

The service reads configuration from environment variables. The most important
ones are:

| Variable | Description | Default |
| --- | --- | --- |
| `DATABASE_URL` | Path to the SQLite database file | `app.db` |
| `SECRET_KEY` | 32-byte secret for signing cookies; provide one to keep sessions across restarts | generated at runtime |
| `AUTH_SERVICE_URL` | Base URL of the Pushkind authentication service | _required_ |
| `PORT` | HTTP port | `8080` |
| `ADDRESS` | Interface to bind | `127.0.0.1` |
| `DOMAIN` | Cookie domain applied to session cookies (without protocol) | `localhost` |

Create a `.env` file if you want these values loaded automatically via
[`dotenvy`](https://crates.io/crates/dotenvy).

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
- **Explicit errors**: use `thiserror` to define granular error types and convert
  them into `ServiceError`/`RepositoryError` variants instead of relying on
  `anyhow`.
- **No panics in production paths**: avoid `unwrap`/`expect` in request handlers,
  services, and repositories—propagate errors instead.
- **Security aware**: sanitize any user-supplied HTML using `ammonia`, validate
  inputs with `validator`, and always enforce role checks with
  `pushkind_common::routes::check_role`.
- **Testable**: accept traits rather than concrete types in services and prefer
  dependency injection so the mock repositories can be used in tests.

Following these guidelines will help new functionality slot seamlessly into the
existing architecture and keep the service reliable in production.
