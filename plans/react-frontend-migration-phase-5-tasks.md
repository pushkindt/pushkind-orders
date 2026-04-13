# Tasks: React Frontend Migration Phase 5

## Scope
This task file covers only Phase 5 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- cut over `GET /products` to a Vite-built React document
- add canonical typed product APIs under `/api/v1/products`
- add structured JSON mutation responses for product create, edit, and CSV
  upload flows
- replace the current Tera-owned add/edit/filter modal behavior with
  React-owned UI
- move product form and upload validation ownership into `src/forms/products.rs`
  with Russian copy

Do not start Phase 6, Phase 7, or Phase 8 in this file.
Phase 5 is complete only when the products page works end to end through
React-owned UI and active runtime behavior no longer depends on flash-driven
redirects, inline modal scripts, or `templates/products/*.html`.

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)
- Migration plan:
  [../plans/react-frontend-migration.md](../plans/react-frontend-migration.md)
- Phase 4 task file:
  [../plans/react-frontend-migration-phase-4-tasks.md](../plans/react-frontend-migration-phase-4-tasks.md)
- Current products routes:
  [../src/routes/products.rs](../src/routes/products.rs)
- Current products service logic:
  [../src/services/products.rs](../src/services/products.rs)
- Current products forms:
  [../src/forms/products.rs](../src/forms/products.rs)
- Current legacy products DTOs:
  [../src/dto/products.rs](../src/dto/products.rs)
- Current React API DTOs:
  [../src/dto/api.rs](../src/dto/api.rs)
- Current API routes:
  [../src/routes/api.rs](../src/routes/api.rs)
- Current API service composition:
  [../src/services/api.rs](../src/services/api.rs)
- Current frontend shell and helpers:
  [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts)
  [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts)
  [../frontend/src/lib/bootstrap.ts](../frontend/src/lib/bootstrap.ts)
  [../frontend/src/components/OrdersShell.tsx](../frontend/src/components/OrdersShell.tsx)
- Current Tera products page and partials:
  [../templates/products/index.html](../templates/products/index.html)
  [../templates/products/product_list.html](../templates/products/product_list.html)
  [../templates/products/add_product_modal.html](../templates/products/add_product_modal.html)
  [../templates/products/edit_product_modal.html](../templates/products/edit_product_modal.html)
  [../templates/products/filter_products_modal.html](../templates/products/filter_products_modal.html)
- Current shared markdown macro:
  [../templates/components/macros.html](../templates/components/macros.html)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-orders`.
- Treat the feature spec and migration plan as the source of truth.
- Assume Phase 4 is already complete:
  `GET /`,
  `GET /na`,
  and `GET /order/{order_id}` are React-backed,
  and the shared shell plus mutation helper patterns already exist in
  `frontend/src/`.
- Keep `GET /categories`, `GET /tags`, `GET /price-levels`, and `GET /vendors`
  on the current Tera runtime path in this phase.
- Keep the Store API under `/api/v1/store/{hub_id}` unchanged.
- Do not introduce page-shaped bootstrap endpoints such as
  `/api/v1/products-page`.
- Do not introduce client-side routing. `GET /products` must stay a native
  full-page route.
- Do not remove `tera` or `actix-web-flash-messages` in this phase.
- Treat service-layer role and vendor-scoping logic as authoritative even when
  route comments or old templates drift.

## What You Will Change In Phase 5
You will change only these repository areas:

- edit `src/dto/api.rs`
- edit `src/forms/products.rs`
- edit `src/error_conversions.rs`
- edit `src/services/api.rs`
- edit `src/services/products.rs`
- edit `src/routes/api.rs`
- edit `src/routes/products.rs`
- edit `src/lib.rs`
- edit `src/frontend.rs`
- edit `frontend/vite.config.ts`
- create `frontend/app/products.html`
- edit `frontend/src/lib/models.ts`
- edit `frontend/src/lib/api.ts`
- edit `frontend/src/lib/api.test.ts`
- create `frontend/src/entries/products.tsx`
- create `frontend/src/pages/ProductsPage.tsx`
- create `frontend/src/pages/ProductsPage.test.tsx`
- create any product-only React components needed under `frontend/src/components/`
- edit `tests/api.rs`
- edit `README.md`
- edit `SPEC.md`

If you find yourself editing category, tag, price-level, vendor, order-detail,
or Store routes beyond shared helper reuse, stop. That belongs to later phases.

## Deliverables
- `GET /products` is served from a built frontend document and rendered by
  React.
- `GET /api/v1/products` returns a canonical typed collection DTO for the
  products page.
- `GET /api/v1/products/{product_id}` returns a canonical typed detail DTO for
  edit-modal hydration.
- Product create, edit, and CSV upload flows use structured JSON mutation
  responses rather than flash-message redirects.
- Product form validation copy is owned by `src/forms/products.rs`, with
  Russian strings on validator annotations and `#[error("...")]` variants.
- The React products page preserves current supported behavior:
  search,
  `show_archived`,
  pagination,
  row-driven editing,
  role-scoped vendor behavior,
  markdown description authoring,
  tag multi-select,
  and CSV upload.
- The products page no longer depends at runtime on:
  `templates/products/index.html`,
  `templates/products/product_list.html`,
  `templates/products/add_product_modal.html`,
  `templates/products/edit_product_modal.html`,
  `templates/products/filter_products_modal.html`,
  the TomSelect CDN asset,
  or inline modal scripts.

## Step 0: Confirm The Starting Point
Run these commands before you make any Phase 5 changes:

```bash
pwd
git status --short
sed -n '1,260p' src/routes/products.rs
sed -n '1,260p' src/routes/api.rs
sed -n '1,320p' src/services/products.rs
sed -n '1,320p' src/forms/products.rs
sed -n '1,260p' src/dto/products.rs
sed -n '1,260p' templates/products/index.html
sed -n '1,260p' templates/products/product_list.html
sed -n '1,260p' templates/products/add_product_modal.html
sed -n '1,320p' templates/products/edit_product_modal.html
sed -n '1,220p' templates/products/filter_products_modal.html
```

Expected result before Phase 5 starts:
- `GET /products` still renders through Tera
- product add, edit, and upload routes still redirect with flash messages
- there is no `GET /api/v1/products/{product_id}`
- there is no built `frontend/app/products.html`
- the products page still depends on inline modal scripts and TomSelect
- `src/forms/products.rs` still owns English error strings and does not expose
  field-addressable Russian validation errors for React forms

## Task 1: Define Canonical Product API Contracts
Goal:
introduce resource-style product contracts without inventing a page-shaped API.

### 1.1 Keep React API DTOs In `src/dto/api.rs`
Expand [../src/dto/api.rs](../src/dto/api.rs) with product-facing DTOs.
Do not grow [../src/dto/products.rs](../src/dto/products.rs) with new React API
contracts; that module may remain for legacy query and template helpers.

Add at least:
- `ProductListItemDto`
- `ProductTagDto`
- `ProductPriceLevelRateDto`
- `ProductCollectionFiltersDto`
- `ProductCollectionDto`
- `ProductEditorOptionsDto`
- `ProductDetailsDto`
- mutation DTOs aligned with the existing React pattern:
  field error DTO,
  mutation error DTO,
  product mutation success DTO,
  upload success DTO

### 1.2 Collection Contract Requirements
`GET /api/v1/products` must remain resource-style and reusable.
Do not add `/api/v1/products-page`.

The collection payload should expose at least:
- `items`
- `pagination`
- `active_filters`
- product-scoped editor options needed by this page while later phases are not
  migrated yet:
  categories,
  tags,
  price levels,
  and vendors where allowed by role

Each list item should include the fields the current page actually renders:
- `id`
- `name`
- `sku`
- rendered or display-ready description
- `units`
- `amount`
- `currency`
- `is_archived`
- `category`
- `vendor`
- `updated_at`
- image preview URLs
- tag summaries
- price-level summaries

### 1.3 Detail Contract Requirements
`GET /api/v1/products/{product_id}` should power the edit modal.

It must expose:
- the editable product fields
- current price-level values
- tag ids
- image URLs
- category and vendor identifiers
- the same role-scoped editor options needed by the modal

Requirements:
- vendor-scoped users must never receive another vendor’s product details
- admin-only vendor assignment options must stay admin-only
- do not leak raw template context or modal-specific HTML

### 1.4 DTO Tests
Add focused DTO tests covering:
- one product list-item conversion
- one product collection DTO construction
- one product details DTO construction
- mutation error DTO mapping for field-addressable validation errors

## Task 2: Move Product Validation Ownership Into `src/forms/products.rs`
Goal:
make the forms layer own Russian validation copy and typed payload conversion,
the same way as the migrated services.

### 2.1 Localize `src/forms/products.rs`
Update [../src/forms/products.rs](../src/forms/products.rs) so:
- validator annotations carry Russian messages directly in
  `#[validate(..., message = "...")]`
- form error enum variants carry Russian messages directly in
  `#[error("...")]`
- field-level validation errors can be returned in a stable field-addressable
  format for React forms

### 2.2 Add Strongly Typed Payload Counterparts
For React-owned mutations, form boundary types must convert into strongly typed
payloads before services run.

That applies to:
- add product
- edit product
- CSV upload

Requirements:
- keep the common `ServiceError` pattern
- do not introduce a local service error type
- keep or restore `From<ProductFormError> for ServiceError` conversions as
  needed
- routes should perform `Form -> Payload` conversion at the boundary

### 2.3 Upload Error Shape
CSV upload errors must remain user-actionable in React.

Requirements:
- file-level failures should map to a stable field such as `csv`
- row-specific failures should carry Russian copy and stable paths when
  practical
- malformed multipart or form-encoding failures must not collapse into
  unhelpful generic parse errors

## Task 3: Add Product API Services And Routes
Goal:
put the React API workflow in the established `services::api` plus
`routes::api` shape rather than mixing page composition into route modules.

### 3.1 Edit `src/services/api.rs`
Add product-focused API composition helpers such as:
- `get_product_collection_data(...)`
- `get_product_details_data(...)`

Requirements:
- keep API composition in `src/services/api.rs`
- keep business rules and persistence in `src/services/products.rs`
- reuse existing repository and service helpers rather than duplicating product
  lookup logic

### 3.2 Edit `src/routes/api.rs`
Add canonical product API routes:
- `GET /api/v1/products`
- `GET /api/v1/products/{product_id}`
- `POST /api/v1/products`
- `PUT /api/v1/products/{product_id}`
- `POST /api/v1/products/upload`

Requirements:
- GET handlers return typed DTOs
- mutation handlers return structured JSON responses
- `422` responses use the field-addressable mutation error shape
- auth and not-found behavior align with the existing React-owned routes
- mutation helpers must keep handling redirected or non-JSON auth responses in
  the frontend layer

### 3.3 Access And Scoping Rules
Preserve current backend authority:
- users without products access must not get collection or detail data
- vendor users must see only vendor-scoped products
- vendor users must not assign arbitrary vendors through the frontend
- admin users may assign or clear vendor ownership where the current service
  contract allows it

## Task 4: Cut Over `GET /products` To Built HTML
Goal:
make the full products page React-owned without preloading template context on
the server.

### 4.1 Add A Built Frontend Document
Create:
- `frontend/app/products.html`
- a Vite input entry for it in `frontend/vite.config.ts`
- `frontend/src/entries/products.tsx`

### 4.2 Extend Frontend Asset Constants
Add a products document constant in
[../src/frontend.rs](../src/frontend.rs), aligned with the current index,
no-access, and order document constants.

### 4.3 Edit `src/routes/products.rs`
Update [../src/routes/products.rs](../src/routes/products.rs) so
`GET /products`:
- still uses the existing authenticated route scope
- still enforces the current access model
- redirects unauthorized users to `/na`
- serves the built frontend document instead of rendering
  `templates/products/index.html`
- returns a clear `503 Service Unavailable` response when the frontend document
  is missing
- does not preload the full product collection only to throw it away

### 4.4 Lightweight Access Check Rule
Do not keep the current pattern of calling `load_products_page(...)` only to
decide whether the page may open.

Instead:
- add a small service helper such as `ensure_products_page_access(...)`
- keep collection loading inside the JSON API path

## Task 5: Build The React Products Page And Local Components
Goal:
replace the current template-driven products UI with React while preserving the
current supported workflow and look.

### 5.1 Create `frontend/src/pages/ProductsPage.tsx`
The new page must:
- mount inside the shared orders shell
- fetch from `GET /api/v1/products`
- read current filters from the URL
- keep native full-page navigation for search and pagination
- preserve the current Bootstrap-oriented list layout and archived styling
- show explicit loading, empty, and fatal states

### 5.2 Preserve Current Supported Filters Only
The current supported contract is:
- `search`
- `page`
- `show_archived`

In Phase 5:
- keep the show-archived modal or equivalent React filter UI
- do not invent category, tag, vendor, or price-level filters unless you first
  extend the canonical backend contract and docs

### 5.3 Replace Add/Edit Modal Behavior With React
Implement React-owned product modal components for:
- add product
- edit product
- filters
- CSV upload trigger or panel

Requirements:
- row interaction must still open editing affordances
- edit modal data must come from typed React state and the product detail API,
  not data attributes or server-rendered HTML fragments
- modal submit state must reset cleanly on success and on close
- successful mutations should update the visible list without requiring a full
  hard refresh

### 5.4 Preserve Markdown Editing Pattern
The current product forms use the shared markdown macro pattern with
editor/preview tabs.

The React replacement should preserve:
- Russian tab labels
- textarea plus preview layout
- hidden-form-field behavior replaced by typed React state
- display close to the current Bootstrap output

### 5.5 Replace TomSelect With A React Multi-Select Pattern
The tag selector must not keep depending on the TomSelect CDN script.

Use a React-owned dropdown-style multi-select pattern aligned with the migrated
Pushkind services so it can later move to `pushkind-common`.

Requirements:
- multiple selected tags remain visible and removable
- clearing selections is explicit and stable
- modal open/close cycles must not leak stale selection state

## Task 6: Implement Product Mutation Helpers And UI Error Handling
Goal:
make product mutations follow the same frontend mutation pattern as the other
migrated services.

### 6.1 Edit `frontend/src/lib/models.ts` And `frontend/src/lib/api.ts`
Add typed models and helpers for:
- product collection
- product details
- add product mutation
- edit product mutation
- upload products mutation
- product field errors and mutation errors

### 6.2 Mutation Helper Requirements
Mutation helpers must:
- parse structured JSON success and error payloads
- preserve redirected or non-JSON auth handling
- avoid throwing secondary errors inside catch blocks
- support field-level errors for add and edit forms
- support actionable upload errors for the CSV flow

### 6.3 UI Error Ownership
React forms must own and render their own validation state.

Requirements:
- field-level errors render next to fields
- form-level errors render in the same alert style used by the migrated
  services
- errors clear predictably after correction or successful submit
- the UI must not remain stuck in a disabled or grayed-out state after
  success or failure

## Task 7: Tests, Docs, And Exit Checklist
Goal:
close Phase 5 with parity checks and documentation aligned to runtime.

### 7.1 Backend Tests
Extend Rust coverage for:
- `GET /api/v1/products`
- `GET /api/v1/products/{product_id}`
- `POST /api/v1/products`
- `PUT /api/v1/products/{product_id}`
- `POST /api/v1/products/upload`
- vendor-scoped authorization and not-found behavior
- field-addressable validation responses

### 7.2 Frontend Tests
Add or extend tests for:
- product payload parsing
- filter URL helpers
- modal state reset
- tag multi-select behavior
- mutation error rendering
- upload success and upload error handling

### 7.3 Documentation Updates
Update [../SPEC.md](../SPEC.md) and [../README.md](../README.md) to reflect:
- `GET /products` is React-backed
- canonical `/api/v1/products` resource APIs
- canonical product JSON mutation routes
- product-scoped supporting data strategy
- remaining Tera pages that are intentionally still unmigrated

### 7.4 Verification Commands
Run all required checks before you consider Phase 5 complete:

```bash
cargo build --all-features --verbose
cargo test --all-features
cargo clippy --all-features --tests -- -Dwarnings
cargo fmt --all -- --check
cd frontend && npm run typecheck
cd frontend && npm run test
cd frontend && npm run build
```

### 7.5 Manual Verification Checklist
Manually verify at least:
- opening `/products` as an admin
- opening `/products` as a vendor-scoped user
- search and pagination URL behavior
- show-archived toggle behavior
- add product success
- edit product success
- CSV upload success
- field-level validation on invalid product input
- auth-expiry handling during a product mutation

## Exit Criteria
Phase 5 is complete only when all of the following are true:

- `GET /products` is served from built React HTML
- the React products page initializes entirely from typed `/api/v1/products...`
  contracts
- product create, edit, and upload flows no longer depend on flash-driven
  redirects
- runtime behavior no longer depends on Tera product templates, TomSelect, or
  inline modal scripts
- all required verification commands pass
