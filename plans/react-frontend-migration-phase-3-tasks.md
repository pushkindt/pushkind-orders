# Tasks: React Frontend Migration Phase 3

## Scope
This task file covers only Phase 3 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- cut over `GET /` to a Vite-built HTML document
- add canonical resource-style `GET /api/v1/orders`
  and `GET /api/v1/orders/{order_id}` contracts
- extend the shared frontend models and API helpers for order collection and
  order details data
- replace the Phase 1 placeholder on `GET /` with a minimal React-owned orders
  bootstrap page that initializes from typed APIs
- keep all other hub pages on the current Tera runtime path

Do not start Phase 4, Phase 5, Phase 6, Phase 7, or Phase 8 in this file.
Phase 3 is complete only when `GET /` is served from a built frontend document
and initializes from typed resource APIs, while `GET /order/{order_id}`,
`GET /products`, `GET /categories`, `GET /tags`, `GET /price-levels`, and
`GET /vendors` still render through the current Tera templates.

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)
- Migration plan:
  [../plans/react-frontend-migration.md](../plans/react-frontend-migration.md)
- Phase 1 task file:
  [../plans/react-frontend-migration-phase-1-tasks.md](../plans/react-frontend-migration-phase-1-tasks.md)
- Phase 2 task file:
  [../plans/react-frontend-migration-phase-2-tasks.md](../plans/react-frontend-migration-phase-2-tasks.md)
- Current shell and no-access DTOs:
  [../src/dto/api.rs](../src/dto/api.rs)
- Current orders API routes:
  [../src/routes/api.rs](../src/routes/api.rs)
- Current index route:
  [../src/routes/main.rs](../src/routes/main.rs)
- Current order details route:
  [../src/routes/orders.rs](../src/routes/orders.rs)
- Current legacy index DTOs:
  [../src/dto/main.rs](../src/dto/main.rs)
- Current legacy order details DTOs:
  [../src/dto/orders.rs](../src/dto/orders.rs)
- Current frontend entry and API helpers:
  [../frontend/src/entries/index.tsx](../frontend/src/entries/index.tsx)
  [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts)
  [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts)
- Current dashboard template behavior to preserve later:
  [../templates/main/index.html](../templates/main/index.html)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-orders`.
- Treat the feature spec and migration plan as the source of truth.
- Assume Phase 2 is already complete:
  `/na` is React-backed,
  `GET /api/v1/iam` exists,
  `GET /api/v1/no-access` exists,
  and the shared shell lives under `frontend/src/components/`
  and `frontend/src/lib/`.
- Keep `GET /order/{order_id}` on the current Tera rendering path in this
  phase.
- Keep `/order/{order_id}/modal` as the existing HTML fragment route in this
  phase.
- Keep `GET /products`, `GET /categories`, `GET /tags`, `GET /price-levels`,
  and `GET /vendors` on the current Tera rendering path in this phase.
- Do not add page-shaped bootstrap endpoints such as `/api/v1/index` or
  `/api/v1/order-page`.
- Do not remove Tera, flash-message middleware, or HTMX in this phase.

## What You Will Change In Phase 3
You will change only these repository areas:

- edit `src/dto/api.rs`
- edit `src/services/api.rs`
- edit `src/routes/api.rs`
- edit `src/routes/main.rs`
- edit `frontend/src/lib/models.ts`
- edit `frontend/src/lib/api.ts`
- create `frontend/src/lib/api.test.ts`
- edit `frontend/src/entries/index.tsx`
- create `frontend/src/pages/OrdersIndexBootstrapPage.tsx`
- create `frontend/src/pages/OrdersIndexBootstrapPage.test.tsx`
- edit `README.md`
- create `tests/api.rs`

If you find yourself editing `src/routes/orders.rs`, `src/routes/products.rs`,
`src/routes/categories.rs`, `src/routes/tags.rs`,
`src/routes/price_levels.rs`, `src/routes/vendors.rs`, or the Tera templates
for those pages, stop. That belongs to later phases.

## Deliverables
- `GET /` is served from the built `assets/dist/app/index.html` document after
  authentication and authorization checks.
- `GET /` no longer loads full Tera page data on the server just to decide
  whether the user is allowed to open the page.
- `GET /api/v1/orders` returns a canonical typed collection DTO rather than the
  raw `Paginated<Order>` payload.
- `GET /api/v1/orders/{order_id}` returns a typed order details DTO.
- The new GET API surface is resource-style and reusable, not page-shaped.
- The React `GET /` entry uses the shared shell and loads its data entirely
  from typed client APIs.
- `GET /order/{order_id}` and `/order/{order_id}/modal` still run through the
  current Tera and HTMX flow.

## Step 0: Confirm The Starting Point
Run these commands before you make any Phase 3 changes:

```bash
pwd
git status --short
sed -n '1,260p' src/dto/api.rs
sed -n '1,260p' src/services/api.rs
sed -n '1,260p' src/routes/api.rs
sed -n '1,220p' src/routes/main.rs
sed -n '1,220p' frontend/src/entries/index.tsx
sed -n '1,260p' frontend/src/lib/models.ts
sed -n '1,260p' frontend/src/lib/api.ts
```

Expected result before Phase 3 starts:
- `GET /` is still rendered by `src/routes/main.rs`
- `show_index` still loads the full orders page data on the server
- `GET /api/v1/orders` still returns `response.orders`
- there is no local `GET /api/v1/orders/{order_id}`
- `frontend/src/entries/index.tsx` still mounts the Phase 1 placeholder page
- there is no typed order collection or order details model in
  `frontend/src/lib/models.ts`

## Task 1: Cut Over `GET /` To Built HTML With A Lightweight Access Check
Goal:
stop rendering the dashboard HTML through Tera and stop preloading the full
orders list on the server when the React page will fetch that data again.

### 1.1 Edit `src/routes/main.rs`
Update [../src/routes/main.rs](../src/routes/main.rs) so `GET /`:

- still requires authentication through the existing Actix scope
- still redirects unauthorized users to `/na`
- serves the built `FRONTEND_INDEX_DOCUMENT` instead of rendering
  `templates/main/index.html`
- returns a clear `503 Service Unavailable` message if the built HTML file does
  not exist yet
- logs and returns `500` for unexpected frontend asset errors

### 1.2 Access-Check Rule
Do not keep this pattern:
- calling `main_service::load_index_page(...)`
- discarding the data
- then serving the built HTML anyway

Instead:
- perform only the lightweight authorization check needed for the route
- reuse `resolve_hub_access` or an equally small service-layer helper
- do not issue the full orders list query from `show_index`

This is mandatory because later React bootstrap will fetch the typed order
collection itself.

### 1.3 Do Not Change Other Full-Page Routes
In this phase:
- do not cut over `GET /order/{order_id}` to built HTML
- do not create `frontend/app/order.html`
- do not touch the modal-fragment route `/order/{order_id}/modal`

## Task 2: Expand The Orders API DTO Module
Keep React-facing API DTOs in `src/dto/api.rs`. Do not grow
`src/dto/main.rs` or `src/dto/orders.rs` with the new React API contracts;
those remain legacy DTOs for the current Tera routes.

### 2.1 Edit `src/dto/api.rs`
Expand [../src/dto/api.rs](../src/dto/api.rs) so it contains:

- the existing shell DTOs:
  `CurrentUserDto`,
  `NavigationItemDto`,
  `IamDto`,
  `NoAccessPageDto`
- a canonical orders collection contract:
  `OrderListItemDto`,
  `OrderPaginationDto`,
  `OrderCollectionFiltersDto`,
  `OrderCollectionDto`
- a canonical order details contract:
  `OrderCustomerSummaryDto`,
  `OrderProductItemDto`,
  `OrderDetailsDto`

### 2.2 Collection DTO Requirements
`GET /api/v1/orders` must become a reusable resource collection contract, not
a page DTO.

The collection contract should expose at least:
- `items`
- `pagination`
- `active_filters`

Each order list item should expose UI-ready fields needed by the dashboard:
- `id`
- `reference`
- `status`
- `created_at`
- `updated_at`
- `total_cents`
- `currency`
- `products_count`

Use stable strings for enum-like values and explicit strings for date or
datetime values.

### 2.3 Order Details DTO Requirements
`GET /api/v1/orders/{order_id}` should expose the data needed by the future
React order details page without leaking raw template context.

The details payload should include at least:
- top-level order metadata:
  `id`,
  `reference`,
  `status`,
  `created_at`,
  `updated_at`,
  `total_cents`,
  `currency`,
  `notes`,
  `shipping_address`,
  `consignee`,
  `delivery_notes`,
  `payer`
- optional customer summary if available
- order products with their snapshot fields needed by the current page:
  `product_id`,
  `name`,
  `sku`,
  `quantity`,
  `approved_quantity`,
  `price_cents`,
  `currency`,
  `default_price_cents`

Do not include page-only shell data inside this DTO.
Do not introduce a page-shaped `order_page` or `bootstrap` wrapper.

### 2.4 DTO Tests
Add focused tests in `src/dto/api.rs` that cover:
- `CurrentUserDto` conversion
- one collection-item conversion
- one collection DTO construction
- one order-details DTO construction

## Task 3: Add Resource-Style Service Helpers For Orders Collection And Details
Keep the React API composition logic in `src/services/api.rs`.

### 3.1 Edit `src/services/api.rs`
Expand [../src/services/api.rs](../src/services/api.rs) with:

- `get_order_collection_data(...)`
- `get_order_details_data(...)`

Requirements:
- `get_order_collection_data` should build the canonical collection DTO from
  the existing orders list service rather than exposing `response.orders`
  directly
- `get_order_details_data` should build the canonical details DTO from the
  existing order details service
- keep the services resource-oriented:
  collection endpoint returns a collection DTO,
  detail endpoint returns a detail DTO
- do not add page bootstrap service functions such as
  `get_index_page_bootstrap_data`
- keep shell data in `get_shell_data`; do not merge it into the orders
  collection response

### 3.2 Error Rules
These helpers must preserve the existing backend authority for:
- unauthorized access
- not found order ids
- vendor-scoped visibility rules

Do not move authorization into the frontend.

## Task 4: Add Canonical Resource-Style GET Endpoints

### 4.1 Edit `src/routes/api.rs`
Update [../src/routes/api.rs](../src/routes/api.rs) so it exposes:

- `GET /api/v1/iam`
- `GET /api/v1/no-access`
- `GET /api/v1/orders`
- `GET /api/v1/orders/{order_id}`
- `GET /api/v1/client-price-levels`
- `PUT /api/v1/client-price-levels`

Behavior requirements:
- `GET /api/v1/orders` must return the canonical `OrderCollectionDto`
- `GET /api/v1/orders/{order_id}` must return the canonical `OrderDetailsDto`
- `GET /api/v1/orders/{order_id}` must return `404` when the order does not
  exist or is not visible to the authenticated user under the existing service
  rules
- `GET /api/v1/orders` and `GET /api/v1/orders/{order_id}` must keep the
  current authz behavior enforced by backend services

### 4.2 Explicit API Contract Rules
Follow these rules strictly:
- no page bootstrap endpoints
- no `index`-named JSON endpoint
- no endpoint that bundles shell data and order data together
- no raw domain-struct passthroughs
- no Tera-shaped template context serialization

`/api/v1/orders` is the collection.
`/api/v1/orders/{order_id}` is the detail resource.
Nothing page-shaped should be added instead.

### 4.3 Add Backend API Tests
Create [../tests/api.rs](../tests/api.rs) and cover at least:
- `GET /api/v1/orders` returns a typed collection shape
- `GET /api/v1/orders/{order_id}` returns `404` for a missing order
- unauthorized access still returns the expected status

If an existing integration-test module is clearly a better home, document that
choice in the test file comments rather than scattering route tests implicitly.

## Task 5: Extend The Frontend Models And API Client Layer
The React page must consume typed APIs, not anonymous `unknown` payloads wired
directly inside the page component.

### 5.1 Edit `frontend/src/lib/models.ts`
Expand [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts) with:

- `OrderListItem`
- `OrderPagination`
- `OrderCollectionFilters`
- `OrderCollectionData`
- `OrderCustomerSummary`
- `OrderProductItem`
- `OrderDetailsData`

Keep the existing shell models unchanged.

### 5.2 Edit `frontend/src/lib/api.ts`
Expand [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts) with:

- parsers for the new order collection and order details DTOs
- `fetchOrdersCollection(...)`
- `fetchOrderDetails(orderId: number)`

Requirements:
- keep the strict parsing style already used in the repo
- preserve explicit error messages such as
  `Invalid API response: expected string at ...`
- keep redirect/non-JSON auth handling in front of JSON parsing
- build query strings explicitly for collection filters like `search` and
  `page`

### 5.3 Add Frontend API Tests
Create [../frontend/src/lib/api.test.ts](../frontend/src/lib/api.test.ts) and
cover at least:
- collection payload parsing
- order details payload parsing
- one failure case for a malformed numeric field

## Task 6: Replace The Phase 1 Placeholder On `GET /`
This is a route cutover, not the full dashboard migration. The goal is to get
the real page onto the shared React shell and typed APIs without yet reaching
full feature parity.

### 6.1 Replace The Entry
Edit [../frontend/src/entries/index.tsx](../frontend/src/entries/index.tsx).

Stop mounting `PhaseOnePlaceholderPage`.
Mount a new page component instead.

### 6.2 Create `frontend/src/pages/OrdersIndexBootstrapPage.tsx`
Create [../frontend/src/pages/OrdersIndexBootstrapPage.tsx](../frontend/src/pages/OrdersIndexBootstrapPage.tsx).

This page must:
- use `useOrdersShell`
- fetch the canonical orders collection from `/api/v1/orders`
- render explicit loading and fatal error states
- render inside `OrdersShell`
- initialize entirely from typed client APIs after the static HTML document
  loads

Phase 3 UI requirements:
- it may stay intentionally transitional
- it does not need full parity with the Tera dashboard yet
- it should render enough real collection data to prove the route is no longer
  placeholder-only
- it should not depend on embedded server JSON

Recommended minimum visible output:
- page title or eyebrow indicating the orders list
- current item count
- a simple list or table of orders using the typed collection data
- one or two representative fields per order:
  id,
  status,
  updated timestamp,
  total,
  or products count

### 6.3 Add A Small Frontend Test
Create [../frontend/src/pages/OrdersIndexBootstrapPage.test.tsx](../frontend/src/pages/OrdersIndexBootstrapPage.test.tsx).

Cover at least one render-path expectation for the transitional page:
- loading state
- fatal state
- or ready-state rendering from a typed fixture

### 6.4 Do Not Start Full Dashboard Migration Yet
In this phase:
- do not rebuild the full filter modal
- do not migrate the full current row layout one-to-one yet
- do not add order-edit modal React behavior
- do not migrate approval editing
- do not migrate `/order/{order_id}`

Those belong to Phase 4.

## Task 7: Document The New Runtime Expectation For `GET /`
Update [../README.md](../README.md).

Document that:
- `GET /` now depends on built frontend assets
- `/na` and `/` are both served from built frontend documents
- `GET /order/{order_id}` and the admin/catalog pages still use Tera in this
  phase
- the React `GET /` page initializes from `/api/v1/iam` and `/api/v1/orders`
  rather than server-rendered template context

Do not claim that order details or admin pages are already React-owned in
Phase 3.

## Task 8: Verify Phase 3
Run these commands from `pushkind-orders` unless noted otherwise:

1. `cd frontend && npm run typecheck`
2. `cd frontend && npm run test`
3. `cd frontend && npm run build`
4. `cargo build --all-features --verbose`
5. `cargo test --all-features --verbose`
6. `cargo clippy --all-features --tests -- -Dwarnings`
7. `cargo fmt --all -- --check`

Manual checks:
- open `GET /` as an authenticated authorized user and confirm it now serves
  the React-built document
- confirm missing frontend assets produce a clear server-side error on `GET /`
- confirm `GET /` still redirects unauthorized users to `/na`
- confirm `/api/v1/orders` returns the canonical collection shape
- confirm `/order/{order_id}` still uses the existing Tera runtime path

## Phase 3 Exit Checklist
Mark Phase 3 done only if all of the following are true:

- `GET /` is served from a Vite-built HTML document.
- `GET /` no longer preloads the full orders list on the server just to render
  or authorize the page.
- `GET /api/v1/orders` returns a typed collection DTO.
- `GET /api/v1/orders/{order_id}` returns a typed detail DTO.
- The GET API surface is resource-style, not page-shaped.
- The React `GET /` page uses the shared shell and typed API helpers.
- `GET /order/{order_id}` still uses Tera.
- `README.md` documents the new runtime split correctly.

## Explicit Non-Goals For This Task File
Do not do these here:

- migrate the full dashboard behavior one-to-one
- migrate `GET /order/{order_id}` to React
- replace `/order/{order_id}/modal`
- convert order edit or approval updates to JSON mutation flows
- add `GET /api/v1/products`, `GET /api/v1/categories`, `GET /api/v1/tags`,
  `GET /api/v1/price-levels`, `GET /api/v1/vendors`, or `GET /api/v1/users`
- add page bootstrap endpoints instead of resource endpoints
- remove `tera`
- remove `actix-web-flash-messages`
- change the Store API contract
