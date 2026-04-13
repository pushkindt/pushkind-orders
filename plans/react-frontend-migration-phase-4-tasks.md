# Tasks: React Frontend Migration Phase 4

## Scope
This task file covers only Phase 4 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- bring the React `GET /` page from Phase 3 bootstrap state to orders-dashboard
  parity for the currently supported behavior
- cut over `GET /order/{order_id}` to a Vite-built React document
- replace the current order edit modal HTML fragment flow with React-owned
  modal rendering
- add typed JSON mutation routes for order edit and approval updates under
  `/api/v1/orders/...`
- move order-edit and approval validation ownership into `src/forms` with
  Russian field-level validation copy

Do not start Phase 5, Phase 6, Phase 7, or Phase 8 in this file.
Phase 4 is complete only when `GET /` and `GET /order/{order_id}` both work
end to end through React-owned UI and active runtime behavior no longer depends
on Tera page markup or HTML modal fragments for orders flows.

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)
- Migration plan:
  [../plans/react-frontend-migration.md](../plans/react-frontend-migration.md)
- Phase 3 task file:
  [../plans/react-frontend-migration-phase-3-tasks.md](../plans/react-frontend-migration-phase-3-tasks.md)
- Current React orders dashboard:
  [../frontend/src/pages/OrdersIndexBootstrapPage.tsx](../frontend/src/pages/OrdersIndexBootstrapPage.tsx)
- Current order routes:
  [../src/routes/orders.rs](../src/routes/orders.rs)
- Current orders API routes:
  [../src/routes/api.rs](../src/routes/api.rs)
- Current orders service logic:
  [../src/services/orders.rs](../src/services/orders.rs)
- Current React API DTOs:
  [../src/dto/api.rs](../src/dto/api.rs)
- Current order forms:
  [../src/forms/orders.rs](../src/forms/orders.rs)
- Current Tera order page and modal fragment:
  [../templates/order/index.html](../templates/order/index.html)
  [../templates/order/edit_order_modal.html](../templates/order/edit_order_modal.html)
- Current frontend entries and API helpers:
  [../frontend/src/entries/index.tsx](../frontend/src/entries/index.tsx)
  [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts)
  [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts)
  [../frontend/vite.config.ts](../frontend/vite.config.ts)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-orders`.
- Treat the feature spec and migration plan as the source of truth.
- Assume Phase 3 is already complete:
  `GET /` is React-backed,
  `GET /api/v1/orders` is a typed collection,
  and `GET /api/v1/orders/{order_id}` is a typed details resource.
- `GET /order/{order_id}` is still Tera-rendered at the start of Phase 4.
- `GET /order/{order_id}/modal` is still an HTML fragment route at the start
  of Phase 4.
- `POST /orders/{order_id}/edit` still uses redirect-plus-flash semantics at
  the start of Phase 4.
- `POST /orders/{order_id}/products/approvals` still uses the legacy
  server-side interaction path at the start of Phase 4.
- Keep `/products`, `/categories`, `/tags`, `/price-levels`, and `/vendors` on
  the current Tera runtime path in this phase.
- Keep the Store API under `/api/v1/store/{hub_id}` unchanged.
- Do not introduce page-shaped bootstrap routes such as `/api/v1/order-page`
  or `/api/v1/index`.
- Do not introduce client-side routing. Full-page navigation must stay native.
- Do not remove `tera` or `actix-web-flash-messages` in this phase.

## What You Will Change In Phase 4
You will change only these repository areas:

- edit `src/dto/api.rs`
- edit `src/forms/orders.rs`
- edit `src/error_conversions.rs`
- edit `src/services/orders.rs`
- edit `src/routes/api.rs`
- edit `src/routes/orders.rs`
- edit `src/lib.rs`
- edit `src/frontend.rs`
- edit `frontend/vite.config.ts`
- create `frontend/app/order.html`
- edit `frontend/src/lib/models.ts`
- edit `frontend/src/lib/api.ts`
- edit `frontend/src/lib/api.test.ts`
- create `frontend/src/lib/bootstrap.ts`
- edit `frontend/src/entries/index.tsx`
- create `frontend/src/entries/order.tsx`
- edit `frontend/src/pages/OrdersIndexBootstrapPage.tsx`
- create `frontend/src/pages/OrderDetailsPage.tsx`
- create `frontend/src/pages/OrderDetailsPage.test.tsx`
- create any small orders-only React components needed for the order page and
  modal under `frontend/src/components/`
- edit `tests/api.rs`
- edit `README.md`
- edit `SPEC.md`

If you find yourself editing product, category, tag, price-level, vendor, or
storefront routes, stop. That belongs to later phases.

## Deliverables
- `GET /` remains React-backed and now preserves the actual supported orders
  dashboard behavior:
  search,
  pagination,
  localized status badges,
  role-scoped visibility,
  and navigation into order details.
- `GET /order/{order_id}` is served from a built frontend document and rendered
  by React.
- The order details page preserves:
  order header and metadata,
  customer and shipping display,
  line items,
  approved quantity editing,
  and order edit behavior.
- Order edit and approval updates use typed JSON mutation routes under
  `/api/v1/orders/...`.
- React-owned order edit UI uses a typed React modal instead of
  `/order/{order_id}/modal`.
- Order form validation copy is owned by `src/forms/orders.rs` with Russian
  messages on validator annotations and `#[error("...")]` variants.
- Active runtime behavior no longer depends on
  `templates/order/index.html` or
  `templates/order/edit_order_modal.html`.

## Step 0: Confirm The Starting Point
Run these commands before you make any Phase 4 changes:

```bash
pwd
git status --short
sed -n '1,260p' src/routes/orders.rs
sed -n '1,260p' src/routes/api.rs
sed -n '1,260p' src/forms/orders.rs
sed -n '1,260p' src/dto/api.rs
sed -n '1,260p' frontend/src/pages/OrdersIndexBootstrapPage.tsx
sed -n '1,260p' frontend/src/lib/api.ts
sed -n '1,260p' templates/order/index.html
sed -n '1,220p' templates/order/edit_order_modal.html
```

Expected result before Phase 4 starts:
- the React dashboard is still a minimal Phase 3 page
- `GET /order/{order_id}` still renders `templates/order/index.html`
- the order edit modal still depends on `/order/{order_id}/modal`
- order-edit mutations still redirect with flash messages
- approval updates still rely on the legacy route shape and legacy payload

## Task 1: Stabilize The Dashboard Against The Canonical API
Goal:
bring the React dashboard from bootstrap state to parity for the currently
supported contract without inventing unsupported behavior.

### 1.1 Edit `frontend/src/pages/OrdersIndexBootstrapPage.tsx`
Update [../frontend/src/pages/OrdersIndexBootstrapPage.tsx](../frontend/src/pages/OrdersIndexBootstrapPage.tsx)
so the React `GET /` page preserves the current supported dashboard behavior:

- order rows remain visually close to the current Bootstrap layout
- localized status badges remain the same as today
- search still uses the `search` query parameter
- pagination still uses native links and reloads the page
- row clicks or links still navigate to `/order/{order_id}`
- loading, empty, and fatal states stay explicit

### 1.2 Guardrail: Do Not Port Unsupported Filters
The current Tera dashboard template contains stale filter-modal markup that is
not backed by the current service/API contract.

In Phase 4:
- do not port the unsupported status/date filter modal into React
- do not add new dashboard filters unless you first update the spec and the
  canonical backend contract
- treat the Phase 3 typed collection contract as the source of truth:
  current supported filters are `search` and `page`

### 1.3 Add Focused Frontend Coverage
Extend the frontend tests so the dashboard page or its helpers cover:

- localized status rendering
- empty-state rendering
- pagination-link generation
- malformed collection payload rejection where applicable

## Task 2: Cut Over `GET /order/{order_id}` To Built HTML
Goal:
make the order details page React-owned the same way `GET /` is already
React-owned.

### 2.1 Add A Built Frontend Document
Create a new built document for the order page:

- add `frontend/app/order.html`
- add a Vite input entry for it in `frontend/vite.config.ts`
- add a new frontend entry module such as
  `frontend/src/entries/order.tsx`

### 2.2 Extend Frontend Asset Constants
Update [../src/frontend.rs](../src/frontend.rs) with a constant for the built
order document similar to the existing index and no-access document constants.

### 2.3 Edit `src/routes/orders.rs`
Update [../src/routes/orders.rs](../src/routes/orders.rs) so
`GET /order/{order_id}`:

- still requires the current authenticated access model
- still redirects unauthorized users to `/na`
- still redirects missing orders to `/`
- serves the built React order document instead of rendering
  `templates/order/index.html`
- returns a clear `503 Service Unavailable` response when the order frontend
  document is missing
- does not preload full Tera context just to throw it away

### 2.4 Native Route Contract
Do not introduce client-side routing.
The order page must still be a native full-page route at
`/order/{order_id}`.
The React page should read the order id from `window.location.pathname`.

## Task 3: Add Typed JSON Mutation Contracts For Orders
Goal:
replace redirect-plus-flash semantics for React-owned order interactions with
typed JSON mutation responses.

### 3.1 Expand `src/dto/api.rs`
Add typed mutation DTOs in
[../src/dto/api.rs](../src/dto/api.rs), aligned with the shape already used in
the migrated services:

- a stable field error DTO such as `FormFieldErrorDto`
- a stable mutation error DTO such as `ApiMutationErrorDto`
- success DTOs for order update and approval update

Requirements:
- field errors must be addressable by field name
- error responses must carry user-visible Russian copy
- success responses should return the updated resource or updated details
  payload rather than an HTML redirect target

### 3.2 Move Approval Validation Into `src/forms/orders.rs`
The approvals payload must become a forms-owned boundary type.

Do not keep approval validation in `src/dto/orders.rs`.
Instead:
- add a form type and strongly typed payload counterpart in
  `src/forms/orders.rs`
- validate each product approval item at the form boundary
- author Russian validation strings directly on validator annotations and on
  `#[error("...")]` variants

### 3.3 Keep Common `ServiceError`
Do not introduce a local service error type.
Follow the same pattern as the migrated services:

- keep using `pushkind_common::services::errors::ServiceError`
- keep form-to-payload conversion at the route boundary
- convert form errors into typed mutation DTOs at the HTTP boundary
  rather than inventing route-local or service-local error stacks

### 3.4 Add API Mutation Routes Under `/api/v1/orders/...`
Add canonical mutation routes under the API scope.
Use stable `/api/v1/orders/...` paths rather than page-named routes.

At minimum cover:
- update order metadata
- update approved quantities

Recommended shape for this phase:
- `PUT /api/v1/orders/{order_id}`
- `PUT /api/v1/orders/{order_id}/products/approvals`

Requirements:
- `401` for unauthorized
- `404` for missing order or inaccessible vendor-scoped order
- `422` with typed field errors for form validation failures
- `200` or `204` with a typed success body for successful updates

### 3.5 Keep Legacy HTML Mutation Routes Only As Temporary Backward Compatibility
The legacy order routes may remain temporarily if removing them causes churn,
but:

- the new React orders UI must not depend on them
- `/order/{order_id}/modal` must no longer be part of the active orders flow
- the legacy routes should be obviously removable in a later cleanup phase

## Task 4: Build The React Order Details Page
Goal:
replace the Tera order page and modal fragment with React-owned rendering.

### 4.1 Create `frontend/src/pages/OrderDetailsPage.tsx`
Build a React page that:

- fetches shell state through the existing shell helper
- fetches typed order details from `GET /api/v1/orders/{order_id}`
- renders order metadata close to the current Bootstrap card layout
- renders customer summary and CRM link behavior when `public_id` is present
- renders line items with localized totals and quantities
- renders approval-edit controls
- renders a typed order-edit modal owned by React
- shows explicit loading and fatal error states

### 4.2 Preserve The Current Order Semantics
The React order page must preserve:

- localized order status copy
- customer and shipping display
- order totals formatting
- approval editing semantics
- vendor-aware visibility and access checks
- the current distinction between editable lines with a backing `product_id`
  and non-editable historical lines

### 4.3 Replace The HTML Modal Fragment With A React Modal
Create React-owned modal rendering for order edit.

Requirements:
- use Bootstrap modal behavior through React-safe helpers
- do not fetch HTML from `/order/{order_id}/modal`
- seed the modal from the already loaded typed order details resource
- on successful submit, update the on-page state without a full page reload
- on validation errors, show inline field errors owned by the form

### 4.4 Add Small Local Components If Needed
If the page becomes too large, split it into small orders-only components such
as:

- order header and summary
- order products table
- order edit modal
- approval action row

Keep them local to `frontend/src/components/` and do not build shared
abstractions prematurely.

## Task 5: Extend The Frontend API Layer For Order Mutations
Goal:
make order mutations follow the same robust API-helper pattern already used in
the migrated services.

### 5.1 Edit `frontend/src/lib/api.ts`
Add typed helpers for:

- fetching order details
- updating order metadata
- updating product approvals

Requirements:
- continue handling redirected/non-JSON auth responses before JSON parsing
- reject malformed mutation payloads with explicit parsing errors
- parse field-addressable validation errors into a stable client shape

### 5.2 Extend `frontend/src/lib/models.ts`
Add the frontend types needed for:

- order details page state
- order edit request payloads
- approval update request payloads
- typed mutation success/error payloads

### 5.3 Add Frontend Tests
Extend frontend tests to cover:

- details payload parsing
- mutation success parsing
- field-error parsing
- redirected or non-JSON auth responses for mutation helpers

## Task 6: React-Safe Bootstrap Integration
Goal:
keep Bootstrap modal behavior working without inline scripts or unsafe render
time DOM queries.

### 6.1 Add `frontend/src/lib/bootstrap.ts`
Move Bootstrap-specific helper logic needed by the order page into a small
React-safe helper module.

Use it for:
- modal instance lookup and creation
- show/hide behavior
- cleanup on unmount

Do not query modal DOM nodes during render.
Do not cache Bootstrap modal instances in a way that breaks first-open
behavior.

### 6.2 Keep Existing Shared Shell Behavior Intact
Do not regress:
- user menu dropdown behavior
- auth menu hydration fallback behavior
- flash modal ownership inside `OrdersShell`

## Task 7: Tests And Verification
Goal:
cover the new orders page runtime and mutation boundaries.

### 7.1 Backend Tests
Extend [../tests/api.rs](../tests/api.rs) to cover:

- `GET /api/v1/orders/{order_id}` for authorized users
- unauthorized and not-found behavior on the order resource
- order metadata mutation success
- approval mutation success
- typed `422` responses with field errors
- vendor-scoped or role-scoped access boundaries where applicable

### 7.2 Frontend Tests
Add or extend frontend tests for:

- dashboard rendering helpers
- order details page fatal state
- order details payload parsing
- modal open/close behavior where practical
- mutation error rendering paths

### 7.3 Manual Verification Checklist
Before closing Phase 4, manually verify:

1. `GET /` renders through React and preserves search and pagination.
2. `GET /order/{order_id}` renders through React.
3. The order edit modal opens on first click.
4. Submitting order edits updates the page without reload.
5. Approval updates refresh totals and row state without reload.
6. Unauthorized users are redirected to `/na`.
7. Missing orders still redirect to `/`.

## Task 8: Update The Docs
Reflect the Phase 4 changes in the docs once the implementation is complete.

### 8.1 Edit `SPEC.md`
Update [../SPEC.md](../SPEC.md) so it documents:

- `GET /order/{order_id}` as a React-backed built document
- the orders mutation routes under `/api/v1/orders/...`
- structured JSON mutation responses and field-addressable validation errors
- the fact that active orders UI no longer depends on HTML modal fragments

### 8.2 Edit `README.md`
Update [../README.md](../README.md) so it explains:

- that `GET /order/{order_id}` now also depends on built frontend assets
- which typed orders APIs now back the React orders pages
- that product/category/tag/price-level/vendor pages are still on Tera in this
  phase

## Required Commands
Run all of these before declaring Phase 4 complete:

```bash
cargo fmt --all
cargo build --all-features --verbose
cargo test --all-features --verbose
cargo clippy --all-features --tests -- -Dwarnings
cargo fmt --all -- --check
cd frontend && npm run format
cd frontend && npm run typecheck
cd frontend && npm run test
cd frontend && npm run build
```

Phase 4 is complete only when all commands succeed and the manual verification
checklist passes.
