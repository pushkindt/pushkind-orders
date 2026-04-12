# Phase 8 Tasks: Legacy Frontend Removal

## Scope
This phase finishes the React frontend migration for `pushkind-orders` by
removing legacy hub-UI runtime dependencies and dead server-rendered frontend
artifacts that are no longer needed after Phases 1-7.

This phase is cleanup and convergence work only.
Do not introduce new product features.
Do not reshape the Store API under `/api/v1/store/{hub_id}`.

## Goals
- Remove obsolete Tera page templates and fragment endpoints for React-owned
  pages.
- Remove flash-message runtime dependencies from hub frontend flows.
- Remove temporary migration wrappers kept only to bridge the cutover.
- Remove direct `tera` and `actix-web-flash-messages` dependencies from
  `pushkind-orders`.
- Keep the backend-routed, non-SPA React model intact.

## Non-Goals
- Do not migrate the Store API.
- Do not change route structure for React-owned full pages.
- Do not replace backend authorization, validation, pricing, or persistence
  logic with frontend logic.
- Do not add page-shaped bootstrap endpoints.

## Task List

### 1. Audit remaining legacy frontend runtime usage
- Enumerate all remaining Tera-rendered routes, modal fragment routes, and
  flash-message usages in:
  - `src/lib.rs`
  - `src/routes/*.rs`
  - `templates/`
  - `Cargo.toml`
- Separate them into:
  - still required for Store API or non-migrated surfaces
  - removable because a React page and JSON mutation flow already exist
- Confirm that every targeted hub page route now serves built HTML from
  `src/frontend.rs` helpers.

### 2. Remove obsolete legacy hub routes
- Delete legacy modal fragment routes and HTML transport endpoints that are no
  longer used by React pages:
  - order edit modal endpoint
  - category modal endpoint
  - tag modal endpoint
  - price-level modal endpoint
  - vendor modal endpoint
- Delete legacy form POST endpoints that have already been replaced by JSON
  mutations for React-owned pages, provided they are no longer required by any
  remaining non-React surface.
- Remove their registration from `src/lib.rs`.
- Keep route removal resource-style aligned with the migration spec.

### 3. Remove flash-driven hub mutation flow wiring
- Remove `FlashMessage` usage from hub-page routes that remain only as legacy
  wrappers.
- Remove `IncomingFlashMessages`, `FlashMessagesFramework`, and related cookie
  storage setup from the orders service if no remaining route needs them.
- Remove redirect-based UI mutation handling that has already been replaced by
  structured JSON mutation responses.
- Verify that React pages continue to use only JSON mutation responses with
  `{ message, field_errors }`.

### 4. Remove Tera frontend runtime for migrated hub pages
- Delete obsolete Tera templates and inline scripts for migrated pages under
  `templates/`, including page templates and modal fragments that are no
  longer used.
- Keep only templates that are still required by genuinely non-migrated
  surfaces, if any remain.
- Remove any backend code that still assembles full-page Tera contexts for
  already migrated pages.
- Remove page-specific inline JavaScript that is now owned by React.

### 5. Remove direct dependencies no longer needed
- Update `Cargo.toml` to remove direct dependencies on:
  - `tera`
  - `actix-web-flash-messages`
- Remove any now-unused imports, app state wiring, helper functions, and
  support code introduced only for those dependencies.
- Ensure the service still builds cleanly with the remaining dependency graph.

### 6. Tighten docs to final-state wording
- Update `SPEC.md` so migrated hub routes are described only in their final
  React/resource-style form.
- Remove wording that still frames legacy modal endpoints or Tera page runtime
  as supported for migrated pages.
- Update `README.md` to describe the post-migration runtime accurately:
  React-built hub pages, backend auth checks, typed `/api/v1/...` contracts,
  and no direct Tera/flash dependency for migrated hub pages.
- Keep Store API behavior documentation unchanged except where wording needs to
  clarify that it was intentionally not migrated.

### 7. Strengthen verification after removal
- Update or add backend tests for:
  - built-HTML route selection on migrated pages
  - resource-style API behavior after route cleanup
  - unauthorized access behavior after flash removal
- Update or add frontend tests only where cleanup changed assumptions.
- Confirm there is no remaining server-side dependency on removed templates or
  fragment routes.

## Exit Checklist
- No migrated hub page depends on Tera page rendering at runtime.
- No migrated hub mutation depends on flash-message redirects at runtime.
- No obsolete modal fragment endpoint remains for migrated pages.
- `tera` is removed from direct `pushkind-orders` dependencies.
- `actix-web-flash-messages` is removed from direct `pushkind-orders`
  dependencies.
- Store API behavior remains unchanged.
- Docs reflect the final migrated state without legacy drift.

## Required Commands
- `cargo build --all-features --verbose`
- `cargo test --all-features`
- `cargo clippy --all-features --tests -- -Dwarnings`
- `cargo fmt --all -- --check`
- `cd frontend && npm run typecheck`
- `cd frontend && npm run test`
- `cd frontend && npm run build`

## Manual Verification
- Load each migrated hub page and confirm it renders from built frontend
  assets:
  - `/`
  - `/na`
  - `/order/{order_id}`
  - `/products`
  - `/categories`
  - `/tags`
  - `/price-levels`
  - `/vendors`
- Verify at least one JSON mutation flow on each migrated area still shows the
  expected success/error behavior without flash redirects.
- Verify vendor-scoped and admin-scoped access still behave correctly.
- Verify the Store API routes still behave exactly as before.
