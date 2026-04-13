# React Frontend Migration Preserving Existing Orders UI

## Status
Stable

## Date
2026-04-10

## Summary
Migrate the current Tera-based hub UI in `pushkind-orders` to React-managed UI
components while preserving the existing route structure, Bootstrap styling,
Russian copy, operator and vendor workflows, and backend-owned business rules.
The migration MUST follow the same stable pattern already implemented in
`pushkind-auth`, `pushkind-files`, `pushkind-crm`, `pushkind-emailer`, and
`pushkind-todo`:
server-routed pages,
Vite-built static frontend documents for React-owned pages,
typed client data APIs under `/api/v1/`,
resource-style GET endpoints where practical,
and structured JSON mutation responses with form-owned validation copy.

`pushkind-orders` MUST NOT become a SPA.

This migration applies to the authenticated hub UI only. The customer-facing
Store API under `/api/v1/store/{hub_id}` remains backend-owned JSON surface and
is out of scope for this migration.

## Problem
The current orders hub UI is split across Tera templates, flash-message
redirects, Bootstrap modal fragments, and template-owned interaction logic.
That makes order review, approval changes, product management, vendor
assignment, category/tag maintenance, and price-level administration harder to
compose, test, and evolve.

The service currently mixes several frontend ownership models:
- server-rendered full pages via Tera
- HTML fragment rendering for edit modals
- flash-driven POST/redirect UX
- a small hub JSON API surface that is not yet sufficient for React-owned pages

That fragmentation is the main reason to migrate.

## Goals
- Introduce React as the component model for orders hub pages.
- Preserve the current Bootstrap-based design, URLs, semantics, and Russian
  user-visible copy.
- Preserve current backend authorization, validation, pricing, vendor-scoping,
  approval, persistence, and store-facing business rules.
- Replace Tera-owned interactive behavior with React-owned components and typed
  data contracts as pages are migrated.
- Keep `pushkind-orders` server-routed and non-SPA.
- Align frontend architecture with the migration pattern already established in
  the other migrated Pushkind services.

## Non-Goals
- Introducing client-side routing.
- Redesigning the UI or replacing Bootstrap.
- Moving validation, authorization, pricing, vendor-scoping, approval, or
  persistence rules into the browser.
- Changing the customer-facing Store API contract, storefront auth contract, or
  order domain semantics beyond what React needs for hub UI parity.
- Replacing the auth/session model with browser token storage.

## In Scope
- The authenticated hub pages at:
  - `GET /`
  - `GET /order/{order_id}`
  - `GET /products`
  - `GET /categories`
  - `GET /tags`
  - `GET /price-levels`
  - `GET /vendors`
- Shared shell concerns currently handled by Tera layout/navigation.
- Order edit, product approval, category edit, tag edit, price-level edit, and
  vendor edit interactions currently driven by Tera, modal HTML, or flash-based
  redirects.
- Vendor user assignment and unassignment flows.
- Frontend asset build and delivery needed to run React in production and local
  development.
- A local React-backed no-access page for `pushkind-orders`.

## Out Of Scope
- Customer-facing Store API endpoints under `/api/v1/store/{hub_id}/*`.
- Storefront auth ownership split with `pushkind-crm`.
- Schema redesign, repository redesign, or vendor/order workflow redesign.
- Public third-party API design beyond the internal React client-data layer.

## Functional Requirements

### 1. Rendering Model
- The application MUST keep the existing server-owned route model.
- The application MUST NOT introduce client-side routing for `/`,
  `/order/{order_id}`, `/products`, `/categories`, `/tags`,
  `/price-levels`, or `/vendors`.
- React MUST be introduced as page-level or island-level components mounted on
  the existing URLs.
- The target state for migrated pages MUST be React-owned page markup served
  from Vite-built static HTML documents after backend access checks.

### 2. Frontend Document Ownership
- React-owned full pages SHOULD be authored in the frontend workspace and built
  by Vite into static HTML documents under `assets/dist/`.
- Rust MUST continue to own authentication and authorization checks before
  serving those documents.
- Page initialization data MUST NOT remain embedded into server-generated HTML
  in the target state.
- Tera MAY remain only as a temporary migration wrapper until a page is fully
  React-backed.

### 3. Markup And Style Preservation
- Migrated React components MUST preserve the current Bootstrap-based layout,
  table/card structure, modal structure, navigation hierarchy, and class
  conventions unless a deviation is explicitly documented.
- User-visible Russian copy SHOULD remain unchanged except for bug fixes or
  accessibility improvements.
- Existing Bootstrap JS behaviors such as dropdowns, modals, tabs, and
  collapses MUST continue to work.

### 4. Behavioral Parity
- `GET /` MUST continue to present the orders dashboard with current search,
  pagination, and access model.
- `GET /order/{order_id}` MUST continue to present order details, customer and
  shipping data, line items, approval editing, and order edit behavior.
- `GET /products` MUST continue to present product search/filtering, add/edit
  flows, upload flow, and price-level-related product configuration.
- `GET /categories` MUST continue to present category tree management and edit
  behavior.
- `GET /tags` MUST continue to present tag list management and edit behavior.
- `GET /price-levels` MUST continue to present price-level listing, create/edit
  behavior, default handling, and delete behavior.
- `GET /vendors` MUST continue to present vendor listing, create/edit/delete
  behavior, local user creation, vendor-user assignment, and vendor-user
  unassignment behavior.
- Current modal HTML fragment flows MUST be replaced by typed JSON data and
  React-owned modal rendering before those interactions are considered fully
  migrated.

### 5. Client Data API Model
- React-owned page initialization MUST prefer typed GET APIs under `/api/v1/`
  rather than HTML-embedded bootstrap payloads or HTML fragment rendering.
- The target state MUST prefer reusable resource-style APIs over page-shaped
  bootstrap endpoints.
- Shared shell data such as current user, home URL, navigation, and auth-driven
  user-menu items SHOULD be exposed through a typed shell API.
- Expected resource-style GET APIs for the target state include:
  - `/api/v1/iam`
  - `/api/v1/orders`
  - `/api/v1/orders/{order_id}`
  - `/api/v1/products`
  - `/api/v1/categories`
  - `/api/v1/tags`
  - `/api/v1/price-levels`
  - `/api/v1/vendors`
  - `/api/v1/users`
  - `/api/v1/client-price-levels`
  - `/api/v1/no-access`
- Edit-modal data loads SHOULD also follow resource-style routes under the same
  `/api/v1/` surface instead of HTML modal endpoints.
- The migration MUST NOT introduce new page-named bootstrap APIs when a
  resource-style route can express the same data.

### 6. Mutation And Validation Semantics
- React-owned mutation flows SHOULD use structured JSON success/error responses
  instead of flash-message-driven redirects or HTML partial rendering.
- Field-level validation errors MUST be addressable so React can render them
  inline.
- Validation copy for React-owned forms MUST be owned by `src/forms`, following
  the same pattern used in the already migrated services.
- Russian validation strings MUST be defined directly on form field
  `#[validate(..., message = "...")]` annotations and on `#[error("...")]`
  annotations for `FormError` enum variants, rather than assembled in routes or
  services.
- Routes SHOULD convert `Form -> Payload` at the boundary before calling
  services, so services can continue using the common `ServiceError` pattern.
- Upload-style or download-style endpoints MAY remain only where they are still
  the correct transport.

### 7. Backend Boundary
- Authorization, validation, pricing, vendor-scoping, approval logic, and
  persistence MUST remain in Rust services and repositories.
- Routes MUST expose typed DTOs or UI-ready payloads to React rather than
  leaking template contexts directly.
- Legacy HTML fragment endpoints SHOULD be replaced by typed JSON data APIs
  before the corresponding interaction is considered fully migrated.
- The existing Store API under `/api/v1/store/{hub_id}` MUST remain
  backend-owned and MUST NOT be reshaped by this hub UI migration.

### 8. Shared Navigation And User Menu
- The top navigation SHOULD follow the same reusable React pattern already used
  in the migrated Pushkind services.
- The user dropdown MUST always include `Домой` and logout.
- Orders-local dropdown items, if any, MUST render before items fetched from
  the auth menu API.
- Additional menu items SHOULD come from the auth menu API.
- Failure to load auth-driven menu items MUST NOT make `pushkind-orders`
  unavailable.
- Logout MUST always render as the final dropdown action even if fetched menu
  items change.

### 9. No-Access Surface
- `pushkind-orders` MUST own its own no-access page the same way as the other
  migrated services.
- The target state MUST use a local React-backed `/na` page and
  `/api/v1/no-access` payload rather than depending on the shared
  `not_assigned` route implementation.

### 10. Frontend Tooling
- The repository MUST gain a supported frontend toolchain for React and
  TypeScript source code.
- Production builds MUST emit versioned static assets and required static HTML
  documents that can be served by the Rust application.
- The server MUST serve the compiled frontend assets directly.
- Local development MUST support efficient frontend iteration without manual
  asset copying.

## Migration Requirements
- The migration MUST be incremental.
- The migration SHOULD converge on the same stable shape used in the already
  migrated services:
  Vite-built static HTML for React-owned full pages,
  typed `/api/v1/...` client data APIs,
  resource-style GET endpoints,
  structured JSON mutation responses,
  form-owned validation messages,
  and a local React-backed `/na` surface.
- Shared React shell components SHOULD be introduced early for navigation,
  user-menu behavior, and common mutation handling.
- Tera MUST be removable as a runtime dependency once all migrated pages are
  fully React-owned.
- `actix-web-flash-messages` MUST be removable as a direct runtime dependency
  once React-owned mutation flows replace flash-driven redirects.
- Inline JavaScript, template-owned interaction code, and HTML modal fragments
  SHOULD be removed only after equivalent React behavior is verified.
- Regression verification SHOULD rely on backend contract tests, frontend
  component or integration tests, and targeted manual checks for
  authentication-dependent flows.

## Acceptance Criteria
- The same URLs continue to serve the corresponding orders hub pages and
  actions.
- Visual appearance remains substantially unchanged for navigation, orders,
  order details, products, categories, tags, price levels, vendors, and modal
  interactions.
- React-owned pages are served from Vite-built frontend documents after backend
  access checks.
- Page data comes from typed client data APIs rather than HTML-embedded
  bootstrap payloads.
- GET APIs exposed for React follow the resource-style `/api/v1/...` pattern
  rather than page-named bootstrap endpoints.
- React-owned mutations return structured success/error responses with
  field-addressable validation errors.
- Russian validation strings are owned by form field validator annotations and
  `FormError` enum annotations, not by routes or services.
- The shared user dropdown behaves consistently with the already migrated
  services.
- `pushkind-orders` owns a local React-backed `/na` surface.
- No backend business rule is moved to the client.
- Direct `tera` and `actix-web-flash-messages` dependencies are removed from
  `pushkind-orders` once the migration is complete.
- The Store API behavior remains unchanged by the hub UI migration.
- The React frontend builds reproducibly and its assets are served by the
  application runtime.
- Regression coverage exists for backend page-data contracts and critical
  frontend behavior.

## Risks
- React markup can drift from the current templates unless parity is checked
  explicitly.
- Orders has several legacy modal and flash-driven workflows, which increases
  the chance of leaving mixed interaction ownership during an incremental
  migration.
- Vendor-scoped access rules and admin-only mutations need explicit regression
  coverage so React data APIs do not accidentally broaden or narrow access.
