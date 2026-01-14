# pushkind-orders

`pushkind-orders` is the Pushkind hub service for browsing customer orders and the supporting price levels that drive storefront pricing. It ships with server-rendered management pages, paginated data helpers, and a Diesel-backed persistence layer. The project is implemented in Rust on top of Actix Web, Diesel, and Tera and integrates tightly with the shared `pushkind-common` crate for authentication, configuration, and reusable UI helpers.

## Docs

- `SPEC.md` — authoritative behavior spec (routes, invariants, DTOs, idempotency, failure modes).
- `AGENTS.md` — contributor guidance (architecture conventions, coding standards, local workflow).

## Features

- Hub operator pages for orders, products, categories, tags, and price levels.
- Store API under `/api/v1/store/{hub_id}` for product browsing, OTP auth, and order placement.
- Customer-specific pricing via price levels (default + per-customer overrides).
- ZeroMQ publishing for outbound SMS OTP delivery and client sync events.

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
| `APP_SECRET` | 64-byte secret used to sign cookies and flash messages | _required_ |
| `APP_DATABASE_URL` | Path to the SQLite database file | `app.db` |
| `APP_ADDRESS` | Interface to bind | `127.0.0.1` |
| `APP_PORT` | HTTP port | `80` (override to `8080` in `config/local.yaml`) |
| `APP_DOMAIN` | Cookie domain (without protocol) | _required_ |
| `APP_TEMPLATES_DIR` | Glob pattern for templates consumed by Tera | `templates/**/*` |
| `APP_ZMQ_SMS_PUB` | ZeroMQ PUB endpoint for outgoing SMS events | `tcp://127.0.0.1:5561` |
| `APP_ZMQ_CLIENTS_PUB` | ZeroMQ PUB endpoint for outgoing client sync events | `tcp://127.0.0.1:5565` |
| `APP_SMS_SENDER` | Sender identifier for outbound SMS messages | `cns.shared` |
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

The server listens on `http://127.0.0.1:8080` by default (with `APP_ENV=local`) and serves static assets from `./assets` in addition to the Tera-powered HTML pages.

## Development

Use `AGENTS.md` for the full local workflow and quality gates. The quick “do everything” command is:

```bash
make check
```
