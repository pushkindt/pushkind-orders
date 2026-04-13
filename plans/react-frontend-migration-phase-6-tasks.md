# Tasks: React Frontend Migration Phase 6

## Scope
This task file covers only Phase 6 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- cut over `GET /categories`, `GET /tags`, and `GET /price-levels` to
  Vite-built React documents
- add canonical typed APIs for categories, tags, and price levels under
  `/api/v1/...`
- add structured JSON mutation responses for create, edit, delete, and
  price-level assignment flows
- replace the current Tera-owned modal fragment behavior with React-owned UI
- preserve the existing `/api/v1/client-price-levels` workflow while aligning
  it with the React mutation and shell patterns already used in the migrated
  pages

Do not start Phase 7 or Phase 8 in this file.
Phase 6 is complete only when categories, tags, and price levels work end to
end through React-owned UI and active runtime behavior no longer depends on
Tera modal fragments, inline modal scripts, or flash-driven redirects for
those surfaces.

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)
- Migration plan:
  [../plans/react-frontend-migration.md](../plans/react-frontend-migration.md)
- Phase 5 task file:
  [../plans/react-frontend-migration-phase-5-tasks.md](../plans/react-frontend-migration-phase-5-tasks.md)
- Current category routes and services:
  [../src/routes/categories.rs](../src/routes/categories.rs)
  [../src/services/categories.rs](../src/services/categories.rs)
  [../src/forms/categories.rs](../src/forms/categories.rs)
- Current tag routes and services:
  [../src/routes/tags.rs](../src/routes/tags.rs)
  [../src/services/tags.rs](../src/services/tags.rs)
  [../src/forms/tags.rs](../src/forms/tags.rs)
- Current price-level routes and services:
  [../src/routes/price_levels.rs](../src/routes/price_levels.rs)
  [../src/services/price_levels.rs](../src/services/price_levels.rs)
  [../src/forms/price_levels.rs](../src/forms/price_levels.rs)
- Current API routes and DTOs:
  [../src/routes/api.rs](../src/routes/api.rs)
  [../src/services/api.rs](../src/services/api.rs)
  [../src/dto/api.rs](../src/dto/api.rs)
- Current frontend shell and helpers:
  [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts)
  [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts)
  [../frontend/src/components/OrdersShell.tsx](../frontend/src/components/OrdersShell.tsx)
- Current Tera pages and fragments:
  [../templates/categories/index.html](../templates/categories/index.html)
  [../templates/categories/edit_category_modal.html](../templates/categories/edit_category_modal.html)
  [../templates/tags/index.html](../templates/tags/index.html)
  [../templates/tags/edit_tag_modal.html](../templates/tags/edit_tag_modal.html)
  [../templates/price_levels/index.html](../templates/price_levels/index.html)
  [../templates/price_levels/edit_price_level_modal.html](../templates/price_levels/edit_price_level_modal.html)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-orders`.
- Treat the feature spec and migration plan as the source of truth.
- Assume Phase 5 is already complete:
  `GET /`,
  `GET /na`,
  `GET /order/{order_id}`,
  and `GET /products` are React-backed,
  and the shared shell, mutation DTO pattern, and frontend API helpers already
  exist in `frontend/src/`.
- `GET /categories`, `GET /tags`, and `GET /price-levels` still render through
  Tera at the start of Phase 6.
- `GET /category/{category_id}/modal`,
  `GET /tag/{tag_id}/modal`,
  and `GET /price-level/{price_level_id}/modal`
  still exist as HTML fragment routes at the start of Phase 6.
- `POST /categories/add`,
  `POST /category/{category_id}/edit`,
  `POST /category/{category_id}/delete`,
  `POST /tags/add`,
  `POST /tags/edit`,
  `POST /tags/{tag_id}/delete`,
  `POST /price-levels/add`,
  `POST /price-level/{price_level_id}/edit`,
  and `POST /price-level/{price_level_id}/delete`
  still use redirect-plus-flash semantics at the start of Phase 6.
- Keep `GET /vendors` on the current Tera runtime path in this phase.
- Keep the Store API under `/api/v1/store/{hub_id}` unchanged.
- Do not introduce page-shaped bootstrap routes such as
  `/api/v1/categories-page`,
  `/api/v1/tags-page`,
  or `/api/v1/price-levels-page`.
- Do not introduce client-side routing. All full-page navigation must remain
  native.
- Do not remove `tera` or `actix-web-flash-messages` in this phase.

## What You Will Change In Phase 6
You will change only these repository areas:

- edit `src/dto/api.rs`
- edit `src/forms/categories.rs`
- edit `src/forms/tags.rs`
- edit `src/forms/price_levels.rs`
- edit `src/error_conversions.rs`
- edit `src/services/api.rs`
- edit `src/services/categories.rs`
- edit `src/services/tags.rs`
- edit `src/services/price_levels.rs`
- edit `src/routes/api.rs`
- edit `src/routes/categories.rs`
- edit `src/routes/tags.rs`
- edit `src/routes/price_levels.rs`
- edit `src/lib.rs`
- edit `src/frontend.rs`
- edit `frontend/vite.config.ts`
- create `frontend/app/categories.html`
- create `frontend/app/tags.html`
- create `frontend/app/price-levels.html`
- edit `frontend/src/lib/models.ts`
- edit `frontend/src/lib/api.ts`
- edit `frontend/src/lib/api.test.ts`
- create `frontend/src/entries/categories.tsx`
- create `frontend/src/entries/tags.tsx`
- create `frontend/src/entries/price-levels.tsx`
- create `frontend/src/pages/CategoriesPage.tsx`
- create `frontend/src/pages/TagsPage.tsx`
- create `frontend/src/pages/PriceLevelsPage.tsx`
- create frontend tests for these pages or their helpers
- create any small categories-only, tags-only, or price-level-only React
  components needed under `frontend/src/components/`
- edit `tests/api.rs`
- edit `README.md`
- edit `SPEC.md`

If you find yourself editing vendor page routes, vendor-user flows, local user
creation flows, or Store API behavior, stop. That belongs to Phase 7 or is out
of scope for this migration.

## Deliverables
- `GET /categories`, `GET /tags`, and `GET /price-levels` are served from
  built frontend documents and rendered by React.
- `GET /api/v1/categories` and `GET /api/v1/categories/{category_id}` return
  canonical typed category collection and detail resources.
- `GET /api/v1/tags` and `GET /api/v1/tags/{tag_id}` return canonical typed tag
  collection and detail resources.
- `GET /api/v1/price-levels` and
  `GET /api/v1/price-levels/{price_level_id}` return canonical typed
  price-level collection and detail resources.
- Category, tag, and price-level create/edit/delete flows use structured JSON
  mutation responses rather than flash-message redirects.
- Client price-level assignment is handled from React and remains aligned with
  the existing `/api/v1/client-price-levels` contract rather than being
  duplicated under a page-shaped API.
- Validation copy for categories, tags, and price levels is owned by
  `src/forms/*` with Russian strings on validator annotations and
  `#[error("...")]` variants.
- These pages no longer depend at runtime on:
  `templates/categories/index.html`,
  `templates/categories/edit_category_modal.html`,
  `templates/tags/index.html`,
  `templates/tags/edit_tag_modal.html`,
  `templates/price_levels/index.html`,
  `templates/price_levels/edit_price_level_modal.html`,
  or inline modal scripts for those screens.

## Step 0: Confirm The Starting Point
Run these commands before you make any Phase 6 changes:

```bash
pwd
git status --short
sed -n '1,260p' src/routes/categories.rs
sed -n '1,260p' src/routes/tags.rs
sed -n '1,320p' src/routes/price_levels.rs
sed -n '1,260p' src/routes/api.rs
sed -n '1,260p' src/services/categories.rs
sed -n '1,260p' src/services/tags.rs
sed -n '1,320p' src/services/price_levels.rs
sed -n '1,240p' src/forms/categories.rs
sed -n '1,240p' src/forms/tags.rs
sed -n '1,320p' src/forms/price_levels.rs
sed -n '1,260p' templates/categories/index.html
sed -n '1,240p' templates/categories/edit_category_modal.html
sed -n '1,260p' templates/tags/index.html
sed -n '1,220p' templates/tags/edit_tag_modal.html
sed -n '1,320p' templates/price_levels/index.html
sed -n '1,260p' templates/price_levels/edit_price_level_modal.html
```

Expected result before Phase 6 starts:
- the categories, tags, and price-level pages still render through Tera
- modal editing still depends on HTML fragment endpoints
- create/edit/delete flows still redirect with flash messages
- there are no resource-style GET endpoints for category, tag, or price-level
  details
- there are no built `frontend/app/categories.html`,
  `frontend/app/tags.html`,
  or `frontend/app/price-levels.html` documents

## Task 1: Define Canonical Resource-Style API Contracts
Goal:
introduce reusable React-facing category, tag, and price-level contracts
without inventing page-shaped bootstrap endpoints.

### 1.1 Expand `src/dto/api.rs`
Add or extend React-facing DTOs in
[../src/dto/api.rs](../src/dto/api.rs) for:

- category collection and detail resources
- tag collection and detail resources
- price-level collection and detail resources
- mutation success DTOs for create/edit/delete flows
- field-addressable mutation error DTOs where missing
- client price-level assignment list and mutation DTOs only if the current
  payload shape is not yet aligned with the React mutation pattern

Do not grow legacy Tera DTO modules with new React API contracts.

### 1.2 Collection Contract Requirements
Collection GET endpoints must stay resource-style and reusable.
Do not add `/api/v1/categories-page`,
`/api/v1/tags-page`,
or `/api/v1/price-levels-page`.

The collection payloads should expose:
- `items`
- `pagination` where the underlying screen is paginated
- `active_filters` where the screen currently supports query filters
- any editor support data the page needs while later phases remain unmigrated

Preserve existing supported behavior only:
- categories: hierarchy, archive visibility, parent relationships, description
- tags: search and pagination behavior
- price levels: list behavior, default marker, modifier-related inputs, and
  client assignment support

### 1.3 Detail Contract Requirements
Add detail GET endpoints for modal replacement:

- `GET /api/v1/categories/{category_id}`
- `GET /api/v1/tags/{tag_id}`
- `GET /api/v1/price-levels/{price_level_id}`

These endpoints must expose the fields needed to render the React edit UI and
must not leak raw template context or fragment HTML.

### 1.4 DTO Tests
Add focused DTO tests covering:
- category collection/detail conversion
- tag collection/detail conversion
- price-level collection/detail conversion
- mutation error DTO mapping for category, tag, and price-level field errors

## Task 2: Move Validation Ownership Into `src/forms/*`
Goal:
make the forms layer own Russian validation copy and typed payload conversion,
the same way as the already migrated services and earlier orders phases.

### 2.1 Localize Category, Tag, And Price-Level Forms
Update:
- [../src/forms/categories.rs](../src/forms/categories.rs)
- [../src/forms/tags.rs](../src/forms/tags.rs)
- [../src/forms/price_levels.rs](../src/forms/price_levels.rs)

Requirements:
- validator annotations carry Russian messages directly in
  `#[validate(..., message = "...")]`
- form error enum variants carry Russian messages directly in
  `#[error("...")]`
- field-level validation errors can be returned in a stable field-addressable
  format for React forms

### 2.2 Add Strongly Typed Payload Counterparts
For React-owned mutations, route boundary types must convert into strongly
typed payloads before services run.

That applies to:
- create category
- edit category
- delete category where a typed request body is needed
- create tag
- edit tag
- delete tag where a typed request body is needed
- create price level
- edit price level
- delete price level where a typed request body is needed
- assign or clear a client price level

Do not keep validation or payload normalization buried in route handlers or DTO
helpers.

### 2.3 Keep Common `ServiceError`
Do not introduce a local service error type.
Keep the same pattern already used in the migrated services and earlier orders
phases:

- routes convert `Form -> Payload`
- services accept typed payloads
- services continue returning the common `ServiceError`

## Task 3: Add Typed JSON Mutation Endpoints
Goal:
replace redirect-plus-flash semantics for React-owned category, tag, and
price-level interactions with structured JSON responses.

### 3.1 Categories
Add JSON routes for:
- create category
- edit category
- delete category

Requirements:
- success returns typed JSON, not redirects
- validation failures return `{ message, field_errors }`
- unauthorized access remains backend-owned and consistent with current role
  checks

### 3.2 Tags
Add JSON routes for:
- create tag
- edit tag
- delete tag

Preserve current supported search and pagination behavior.

### 3.3 Price Levels
Add JSON routes for:
- create price level
- edit price level
- delete price level

Preserve current behavior around:
- default price-level handling
- modifier application rules
- include/exclude product/category behavior

### 3.4 Client Price-Level Assignment
Do not duplicate the existing assignment workflow with a new page-specific API.
Instead:
- keep using `GET /api/v1/client-price-levels`
- keep using `PUT /api/v1/client-price-levels` or evolve its response shape if
  needed for React mutation handling
- align error and success handling with the established React mutation pattern
  without breaking the resource-style direction

## Task 4: Cut Over Full-Page Routes To Built HTML
Goal:
make categories, tags, and price levels React-owned the same way orders and
products are already React-owned.

### 4.1 Add Built Frontend Documents
Create:
- `frontend/app/categories.html`
- `frontend/app/tags.html`
- `frontend/app/price-levels.html`

Add matching Vite entries in `frontend/vite.config.ts` and matching frontend
entry modules under `frontend/src/entries/`.

### 4.2 Extend Frontend Asset Constants
Update [../src/frontend.rs](../src/frontend.rs) with constants for the built
documents and keep the helper pattern aligned with the already migrated pages.

### 4.3 Edit Full-Page Routes
Update:
- [../src/routes/categories.rs](../src/routes/categories.rs)
- [../src/routes/tags.rs](../src/routes/tags.rs)
- [../src/routes/price_levels.rs](../src/routes/price_levels.rs)

Requirements:
- keep current authenticated access rules
- redirect unauthorized users to `/na`
- serve the built React document instead of rendering Tera
- return a clear `503 Service Unavailable` response if the built document is
  missing
- do not preload full Tera context just to throw it away

## Task 5: Replace Modal Fragment Flows With React UI
Goal:
remove active runtime dependence on HTML fragment endpoints and inline modal
scripts for these pages.

### 5.1 Categories Page
Create `frontend/src/pages/CategoriesPage.tsx`.

The React page must preserve:
- category tree rendering
- create and edit flows
- delete behavior
- archive-related display if currently supported

Do not keep `/category/{category_id}/modal` as a runtime dependency for the
React page.

### 5.2 Tags Page
Create `frontend/src/pages/TagsPage.tsx`.

The React page must preserve:
- tag search
- pagination
- create, edit, and delete flows

Do not keep `/tag/{tag_id}/modal` as a runtime dependency for the React page.

### 5.3 Price Levels Page
Create `frontend/src/pages/PriceLevelsPage.tsx`.

The React page must preserve:
- list behavior
- create, edit, and delete flows
- default-level display
- modifier and include/exclude controls
- client price-level assignment workflow

Do not keep `/price-level/{price_level_id}/modal` as a runtime dependency for
the React page.

### 5.4 Frontend Helper And Model Updates
Extend:
- `frontend/src/lib/models.ts`
- `frontend/src/lib/api.ts`
- `frontend/src/lib/api.test.ts`

Add:
- typed parsers for the new collection/detail/mutation payloads
- typed helpers for create/edit/delete operations
- assignment helpers for client price-level changes
- defensive handling for auth redirects and malformed responses, following the
  same pattern already used in the migrated pages

## Task 6: Preserve Established UX And Styling Patterns
Goal:
keep the new pages aligned with the migration pattern already used in
`pushkind-auth`, `pushkind-crm`, `pushkind-emailer`, and the completed orders
pages.

Requirements:
- reuse `OrdersShell` and existing shared dropdown/menu behavior
- keep Russian copy and Bootstrap-oriented styling close to the current Tera UI
- keep loading, empty, and fatal states explicit
- keep form-owned inline validation behavior
- keep native page navigation and native query parameters
- avoid introducing a new visual language or SPA behavior for these pages

## Task 7: Tests, Docs, And Exit Conditions

### 7.1 Backend Tests
Extend [../tests/api.rs](../tests/api.rs) with route-level coverage for:
- category collection/detail APIs
- tag collection/detail APIs
- price-level collection/detail APIs
- JSON mutation success responses
- JSON mutation validation failures
- authorization boundaries where they differ by role

### 7.2 Frontend Tests
Add focused frontend tests for:
- payload parsing helpers
- page-level empty/loading/error states
- modal replacement behavior or its pure helpers
- client price-level assignment state handling

### 7.3 Documentation
Update:
- [../SPEC.md](../SPEC.md)
- [../README.md](../README.md)

Reflect that:
- `GET /categories`, `GET /tags`, and `GET /price-levels` are React-backed
- canonical React data and mutation APIs are resource-style under `/api/v1/...`
- category/tag/price-level mutation flows are JSON-based for the React pages
- vendors are still pending migration in Phase 7

### 7.4 Required Verification Commands
Run all of the following before considering Phase 6 complete:

```bash
make check
cd frontend && npm run typecheck
cd frontend && npm run test
cd frontend && npm run build
```

### 7.5 Exit Checklist
Phase 6 is complete only when all of the following are true:

- `GET /categories` works end to end through React-owned UI
- `GET /tags` works end to end through React-owned UI
- `GET /price-levels` works end to end through React-owned UI
- category, tag, and price-level edit flows no longer depend on modal HTML
  fragment endpoints at runtime
- price-level assignment works from React without introducing a duplicate
  page-shaped API
- structured field-level validation errors reach the React forms
- `SPEC.md` and `README.md` match the implemented Phase 6 behavior
- vendor pages and vendor-user flows remain untouched for Phase 7
