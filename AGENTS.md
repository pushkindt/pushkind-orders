# AGENTS.md

This document provides guidance to AI code generators when working in this
repository. Follow these practices so that new code matches the established
architecture and conventions.

## Project Context

`pushkind-orders` is a Rust 2024 Actix Web application that uses Diesel with
SQLite, Tera templates, and the shared `pushkind-common` crate. The codebase is
layered into domain models, repository traits and implementations, service
modules, DTOs, Actix routes, forms, and templates. Business logic belongs in the
service layer; handlers and repositories should stay thin and focused on I/O
concerns. The service provides both server-rendered pages for hub operators and
REST APIs for customer-facing storefronts.

## Development Commands

Use these commands to verify your changes before committing:

**Build**
```bash
cargo build --all-features --verbose
```

**Run Tests**
```bash
cargo test --all-features --verbose
```

**Lint (Clippy)**
```bash
cargo clippy --all-features --tests -- -Dwarnings
```

**Format**
```bash
cargo fmt --all -- --check
```

## Coding Standards

- Use idiomatic Rust; avoid `unwrap` and `expect` in production paths.
- Keep modules focused: domain types in `src/domain`, Diesel models in
  `src/models`, DTOs in `src/dto`, and conversions implemented via `From`/`Into`.
- Domain structs should expose strongly typed fields (e.g., `UserEmail`,
  `HubId`, `MenuName`, `RoleName`, `UserName`) that encode validation
  constraints. Construct these types at the boundaries (forms/services) so
  domain data is always trusted and cannot represent invalid input.
- Define error enums with `thiserror` inside the crate that owns the failure and
  return `RepositoryResult<T>` / `ServiceResult<T>` from repository and service
  functions.
- Services should return DTO-level structs when handing data to routes; perform
  domain-to-DTO conversion inside the service layer to keep handlers thin. DTOs
  live in `src/dto` and are optimized for template rendering or JSON serialization.
- Service functions should accept trait bounds (e.g., `OrderReader + OrderWriter`)
  so the `DieselRepository` and `mockall`-powered fakes remain interchangeable.
- Domain structs must not perform validation or normalization (e.g., no
  `to_lowercase`); assume inputs are already sanitized and transformed by forms
  or services before reaching the domain layer.
- Sanitize and validate user input early using `validator` and `ammonia` helpers
  from the form layer.
- Perform trimming, case normalisation, and other input clean-up before
  constructing domain types; domain builders assume callers supply sanitised
  values.
- Forms should have a strongly typed `*Payload` counterpart in the same module
  and a `TryFrom<*Form>` implementation that calls `validate()` and constructs
  strong domain types, mapping type-construction failures to the form error.
  Services should call `try_into()` on incoming forms, then build domain objects
  from the payload plus any contextual ids.
- Prefer dependency injection through function parameters over global state.
- For Diesel update models, avoid nested optionals; prefer single-layer `Option<T>`
  fields and rely on `#[diesel(treat_none_as_null = true)]` when nullable columns
  need to be cleared.
- Document all public APIs and any breaking changes.

## Database Guidelines

- Use Diesel's query builder APIs with the generated `schema.rs` definitions; do
  not write raw SQL.
- Translate between Diesel structs (`src/models`) and domain types inside the
  repository layer using explicit `From` implementations.
- Reuse the filtering builders in `OrderListQuery`/`ProductListQuery` when adding new
  queries and extend those structs rather than duplicating logic.
- Check related records (e.g., users) before inserts or updates and convert
  missing dependencies into `RepositoryError::NotFound` instead of panicking.

## HTTP and Template Guidelines

- Keep Actix handlers in `src/routes` focused on extracting inputs, invoking a
  service, and returning an HTTP response.
- Let Actix handlers manage redirects and flash messaging; keep services
  transport-agnostic.
- Render templates with Tera contexts that only expose sanitized data. Use the
  existing component templates under `templates/` for shared UI.
- For REST APIs, return JSON responses using DTOs from `src/dto`.
- Respect the authorization checks via `pushkind_common::routes::ensure_role` and
  the `SERVICE_ACCESS_ROLE` constant for hub user routes.
- Store API routes use separate session management; check for authenticated
  customers using `get_store_customer_for_hub` from `src/routes/store_session.rs`.

## Store API and Session Management

- Store endpoints (`/api/v1/store/{hub_id}/*`) serve customer-facing storefronts
  and use cookie-based sessions separate from hub user authentication.
- OTP authentication flow: customers request an OTP via `POST /auth/otp`, receive
  it via SMS (published through ZeroMQ), then verify with `POST /auth/otp/verify`
  to establish a session.
- Store sessions persist customer data including assigned price level; use
  `get_store_customer_for_hub` to retrieve the authenticated customer.
- Customer-specific pricing is applied in service layer functions like
  `load_store_products` by checking the customer's assigned price level.
- Store OTP records are managed via the `StoreOtpRepository` trait and expire
  after verification or timeout.

## ZeroMQ Integration

- The service publishes SMS events to a ZeroMQ PUB socket configured via
  `APP_ZMQ_SMS_PUB`.
- Use the `ZmqSender` from `pushkind-common` to publish events; it's injected as
  `Arc<ZmqSender>` in Actix app data.
- SMS messages for OTP delivery are published as JSON events with phone number,
  message body, and sender identifier.
- The ZeroMQ sender runs in a background thread and handles message queuing
  automatically.

## Testing Expectations

- Add unit tests for new service and form logic. When hitting the database, use
  Diesel migrations and helper constructors rather than hard-coded SQL strings.
- Use the mock repository module (`src/repository/mock.rs`) generated with
  `mockall` to isolate service tests from Diesel.
- Test DTO conversions to ensure domain-to-DTO transformations preserve required
  data for templates and API responses.
- Ensure new functionality is covered by tests before opening a pull request.

By following these principles the generated code will align with the project's
architecture, technology stack, and long-term maintainability goals.
