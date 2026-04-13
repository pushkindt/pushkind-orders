# Plan: React Frontend Migration

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)

## Objective
Introduce React for the `pushkind-orders` hub frontend while preserving the
current route structure, Bootstrap styling, Russian copy, operator and vendor
workflows, and backend-owned authorization, validation, pricing, vendor
scoping, approval, and persistence rules. The migration remains server-routed
and non-SPA, and converges on:
Vite-built static HTML for React-owned full pages,
typed `/api/v1/...` client data APIs,
resource-style GET endpoints,
structured JSON mutation responses,
and form-owned validation copy.

The customer-facing Store API under `/api/v1/store/{hub_id}` is not part of
this migration and must remain behaviorally unchanged.

## Current-State Notes
- `src/lib.rs` currently wires Tera, `actix-web-flash-messages`, shared
  `not_assigned`, static assets from `/assets`, the hub UI routes, the small
  hub JSON API surface, and the separate Store API surface.
- The current hub UI is rendered from Tera templates under `templates/` for:
  orders dashboard,
  order details,
  products,
  categories,
  tags,
  price levels,
  and vendors.
- Several interactions still depend on HTML fragment endpoints:
  `GET /order/{order_id}/modal`,
  `GET /category/{category_id}/modal`,
  `GET /tag/{tag_id}/modal`,
  `GET /price-level/{price_level_id}/modal`,
  `GET /vendor/{vendor_id}/modal`.
- Most hub mutations still use flash-message redirects and form posts rather
  than structured JSON responses.
- The current hub JSON API only exposes:
  `GET /api/v1/orders`,
  `GET /api/v1/client-price-levels`,
  `PUT /api/v1/client-price-levels`.
- Vendor-specific access rules already exist in Rust services and must remain
  authoritative during and after migration.

## Fixed Implementation Decisions
- Frontend source code WILL live in `frontend/`.
- Production frontend build output WILL live in `assets/dist/`.
- The React toolchain WILL use `npm`, React, TypeScript, and Vite.
- The backend WILL continue to own routing, authentication, authorization,
  validation, pricing, vendor scoping, approval logic, redirects where still
  applicable, and persistence.
- The application server WILL continue to serve compiled frontend assets from
  the existing `/assets` path.
- Vite WILL own the static HTML documents for:
  `GET /`,
  `GET /order/{order_id}`,
  `GET /products`,
  `GET /categories`,
  `GET /tags`,
  `GET /price-levels`,
  `GET /vendors`,
  and `GET /na`
  once those routes are React-owned.
- React page initialization WILL fetch typed JSON data from backend endpoints;
  page data WILL NOT remain embedded into server-generated HTML in the target
  state.
- New GET endpoints introduced for React-owned page data WILL be versioned
  under `/api/v1/`.
- Those GET endpoints MUST prefer reusable resource-style contracts over
  page-shaped bootstrap endpoints.
- React-owned mutation flows SHOULD also move to `/api/v1/...` JSON endpoints
  so flash-driven UI routes can be removed cleanly at the end of the migration.
- Validation copy for React-owned forms WILL live in `src/forms`.
- Russian validation strings WILL be defined directly in
  `#[validate(..., message = "...")]` annotations on form fields and in
  `#[error("...")]` annotations on `FormError` enum variants.
- Routes SHOULD convert `Form -> Payload` at the boundary before calling
  services so services can continue using the common `ServiceError` pattern.
- The shared navbar and user dropdown WILL align with the React pattern
  already used in the migrated Pushkind services.
- Orders-local dropdown items, if introduced, WILL render before auth-fetched
  menu items.
- Logout WILL remain the final dropdown action even when auth-fetched menu
  items change or fail to load.
- `pushkind-orders` WILL own a local React-backed `/na` route and
  `/api/v1/no-access` payload.
- Tera MAY remain only as a temporary migration wrapper while a page is being
  cut over, and MUST be removable once all migrated pages are React-owned.
- `tera` and `actix-web-flash-messages` MUST be removable from direct
  `pushkind-orders` dependencies by the end of the migration.
- The Store API under `/api/v1/store/{hub_id}` WILL remain backend-owned and
  MUST NOT be reshaped during this frontend migration.
- Regression verification WILL rely on backend contract tests, frontend
  component or integration tests, and targeted manual checks for
  authentication-dependent flows.

## Repository Layout
The implementation SHOULD create and use the following structure:

```text
frontend/
  package.json
  package-lock.json
  tsconfig.json
  vite.config.ts
  src/
    entries/
    components/
    pages/
    styles/
    lib/
assets/
  dist/
src/
  dto/
  forms/
  routes/
  services/
  frontend.rs
templates/
```

Directory intent:
- `frontend/src/entries/`:
  entrypoints for the hub pages and `/na`.
- `frontend/src/components/`:
  reusable shell, navbar, user-menu, modal, form, table, filter, pagination,
  and assignment components.
- `frontend/src/pages/`:
  page-level React components for orders, order details, products,
  categories, tags, price levels, vendors, and no-access.
- `frontend/src/lib/`:
  typed payload readers, API clients, endpoint builders, Bootstrap adapters,
  and cross-service menu helpers.
- `frontend/src/styles/`:
  CSS imports preserving the current Bootstrap-based output.
- `assets/dist/`:
  compiled JavaScript, CSS, static HTML, and manifest output.
- `src/frontend.rs`:
  backend helpers for loading Vite manifest entries and serving built frontend
  HTML documents after route-level access checks.

## Toolchain And Build Outputs

### Frontend Package Management
- Use `npm` as the package manager.
- Commit `frontend/package-lock.json`.
- Do not introduce `pnpm`, `yarn`, or an alternative JavaScript runtime.

### Build Tool
- Use Vite to build the React frontend.
- Configure Vite to emit compiled assets into `assets/dist/`.
- Configure Vite to emit a manifest file at `assets/dist/manifest.json`.
- Configure explicit entrypoints for:
  the orders dashboard,
  the order details page,
  the products page,
  the categories page,
  the tags page,
  the price levels page,
  the vendors page,
  and the no-access page.

### Required `package.json` Scripts
The frontend package MUST expose at least these scripts:
- `dev`
- `build`
- `preview`
- `test`
- `lint`
- `typecheck`

### Source Control Hygiene
- Update `.gitignore` to exclude `frontend/node_modules/`.
- Add `assets/dist/` to `.gitignore` unless deployment later requires
  committed build artifacts.

## Backend Integration

### Asset Serving
- Keep Actix static serving for `/assets` and ensure it covers `assets/dist/`.

### Built HTML Serving
- Add a backend helper that serves the built Vite HTML entry for each
  React-owned full-page route after authentication and authorization checks.
- Align that helper with the thin frontend-loading pattern already used in the
  migrated Pushkind services.
- Rust MUST stop assembling full-page HTML at request time once a route has
  been fully migrated.

### Client Data APIs
- Add typed DTOs under `src/dto/` for reusable orders client data APIs.
- Prefer specific resource-style endpoints under `/api/v1/` over page-shaped
  bootstrap endpoints.
- The target GET surface SHOULD include:
  `GET /api/v1/iam`,
  `GET /api/v1/orders`,
  `GET /api/v1/orders/{order_id}`,
  `GET /api/v1/products`,
  `GET /api/v1/products/{product_id}`,
  `GET /api/v1/categories`,
  `GET /api/v1/categories/{category_id}`,
  `GET /api/v1/tags`,
  `GET /api/v1/tags/{tag_id}`,
  `GET /api/v1/price-levels`,
  `GET /api/v1/price-levels/{price_level_id}`,
  `GET /api/v1/vendors`,
  `GET /api/v1/vendors/{vendor_id}`,
  `GET /api/v1/users`,
  `GET /api/v1/client-price-levels`,
  `GET /api/v1/no-access`.
- `GET /api/v1/orders` SHOULD evolve from the currently mounted list route into
  the canonical orders collection contract for React.
- `GET /api/v1/client-price-levels` SHOULD be retained and aligned into the
  React client-data model rather than duplicated.
- Resource detail endpoints SHOULD replace current modal HTML data transport.
- `GET /api/v1/users` SHOULD replace template-owned user lists used by vendor
  assignment and local user creation flows.
- `GET /api/v1/iam` SHOULD expose the shell data React needs:
  current user identity,
  service-local navigation items,
  auth home URL,
  logout target,
  and any auth-menu fetch URL or menu DTOs needed for dropdown hydration.
- `GET /api/v1/no-access` SHOULD return the local content required by the
  React `/na` page.
- Do not expose raw template contexts directly to the frontend.

### Structured Mutation Responses
- Introduce typed JSON mutation response DTOs for React-owned orders
  interactions.
- The initial JSON mutation surface SHOULD cover:
  update order,
  update approved quantities,
  create product,
  edit product,
  upload products,
  create category,
  edit category,
  delete category,
  create tag,
  edit tag,
  delete tag,
  create price level,
  edit price level,
  delete price level,
  assign client price level,
  create vendor,
  edit vendor,
  delete vendor,
  create local user,
  assign vendor user,
  clear vendor user.
- Field errors SHOULD use a stable field-addressable shape.
- Success responses SHOULD return either the updated resource, a stable success
  marker, or a redirect target only when the client genuinely needs it.
- Legacy redirect-plus-flash handlers MAY coexist temporarily, but React-owned
  pages MUST migrate to JSON request/response handling before the flash
  middleware is removed.

### Form Boundary Ownership
- Move React-owned validation copy into `src/forms`.
- Update category, tag, price-level, vendor, product, order-edit, approval, and
  vendor-assignment form helpers so field-level validation messages are
  authored on validator annotations and `FormError` annotations in Russian.
- Keep sanitization and type construction at the form boundary.
- Keep services free of HTTP-specific validation formatting.

### Local No-Access Ownership
- Replace usage of `pushkind_common::routes::not_assigned` with a local orders
  route for `/na`.
- Keep backend authorization redirects intact, but send unauthorized orders
  traffic to the local React-backed no-access surface.

### Server-Rendered Shell During Migration
- During migration, the backend MAY render a minimal HTML shell that:
  includes the React entrypoint,
  includes compiled CSS,
  provides the mount node for React.
- Any such shell is transitional only. The target state for a migrated page is
  a Vite-built static HTML document, not a Rust-rendered page shell.

## Frontend Runtime Requirements

### Shared Shell And Navigation
- Implement a shared React shell for navbar, layout wiring, user-menu
  behavior, Bootstrap lifecycle integration, and common loading/error
  presentation.
- The shared shell SHOULD align with the reusable dropdown/menu approach
  already used in the migrated Pushkind services.
- Auth menu loading MUST happen after required page data is available so auth
  slowness does not blank the orders page.
- Failure to load auth-driven menu items MUST still leave `Домой` and logout
  available.

### Bootstrap Integration
- Keep Bootstrap CSS and Bootstrap Icons in the rendered output.
- Preserve Bootstrap JS behavior for dropdowns, modals, tabs, tooltips, and
  collapses used by the current UI.
- Move inline Bootstrap lifecycle code into React-safe helpers under
  `frontend/src/lib/`.

### Orders Dashboard Requirements
- The React `GET /` page MUST preserve:
  order rows,
  current search and pagination behavior,
  current role-based visibility,
  and navigation into order details or order editing flows.

### Order Details Requirements
- The React `GET /order/{order_id}` page MUST preserve:
  order header and metadata,
  customer and shipping display,
  line items,
  approved quantity editing,
  order edit behavior,
  vendor-aware visibility,
  and current access constraints.
- The current order edit modal MUST become a typed React-owned modal before
  `/order/{order_id}/modal` can be removed.

### Catalog And Admin Page Requirements
- The React `GET /products` page MUST preserve filters, create/edit behavior,
  price-level-related fields, and CSV upload.
- The React `GET /categories` page MUST preserve tree management semantics and
  edit/delete flows.
- The React `GET /tags` page MUST preserve list management and edit/delete
  flows.
- The React `GET /price-levels` page MUST preserve default handling,
  create/edit/delete flows, and client price-level assignment behavior.
- The React `GET /vendors` page MUST preserve vendor list management, local
  user creation, vendor-user assignment, and vendor-user clearing behavior.
- Resource-scoped modals or drawers SHOULD replace HTML fragment modals once
  the corresponding page is React-owned.

### Data Loading
- React-owned full pages MUST fetch typed JSON data after the static HTML
  document loads.
- The frontend SHOULD use shared API helpers that compose page state from
  narrower resource endpoints.
- React MUST render explicit loading and fatal error states for required data
  fetches.

### Form And Action Handling
- React-owned mutation flows SHOULD use structured JSON request/response
  handling instead of redirect-plus-flash patterns.
- Multipart upload MAY remain `multipart/form-data`, but the React-owned
  response contract MUST still be structured JSON.
- Native browser navigation SHOULD remain in place for full-page route changes;
  React MUST NOT introduce client-side routing.

## Migration Sequence

### Phase 1: Foundation
Deliverables:
- `frontend/` directory with React, TypeScript, and Vite configured.
- Build output emitted to `assets/dist/`.
- Backend helpers for loading frontend manifest entries and serving built HTML.
- `.gitignore` updated for frontend dependencies and generated assets.
- Developer documentation for installing Node and building frontend assets.

Exit criteria:
- `npm run build` succeeds.
- The server can resolve one Vite-built frontend document and its compiled
  assets.

### Phase 2: Shared Shell, Navigation, And Local No-Access
Deliverables:
- Shared React shell for navbar, common layout wiring, and Bootstrap lifecycle
  integration.
- Reusable React user dropdown aligned with the migrated auth/files/crm/todo
  pattern.
- Local React-backed `/na` page and typed `/api/v1/no-access` endpoint.
- Auth menu hydration after initial page render, with resilient fallback to
  `Домой` and logout.

Exit criteria:
- Shared shell behavior no longer depends on inline JavaScript in a base
  template.
- Unauthorized users land on a local orders no-access page rather than the
  shared `not_assigned` implementation.

### Phase 3: Full-Page Document Serving And Base Client Data APIs
Deliverables:
- Vite-managed HTML entries for early-migration orders pages.
- Typed `/api/v1/...` shell and base resource APIs.
- Typed frontend payload readers and API clients.

Exit criteria:
- At least one orders hub page can be served from a Vite-built HTML document
  and initialize entirely from typed client data APIs.

### Phase 4: Orders Dashboard And Order Details Migration
Deliverables:
- React-backed `GET /` page preserving dashboard behavior, search, pagination,
  and role-scoped visibility.
- React-backed `GET /order/{order_id}` page preserving order details,
  approval editing, and order edit behavior.
- Typed JSON replacement for order edit and approval mutation flows.
- React replacement for the current order modal HTML fragment behavior.

Exit criteria:
- Orders dashboard and order details work end to end through React-owned UI
  without depending on Tera page markup or HTML modal fragments.

### Phase 5: Products Migration
Deliverables:
- React-backed `GET /products` page preserving search/filtering, create/edit
  behavior, and CSV upload.
- Typed APIs and JSON mutation responses for product list, product detail, add,
  edit, and upload flows.

Exit criteria:
- The products page works end to end through React-owned UI without depending
  on flash-driven redirects.

### Phase 6: Categories, Tags, And Price Levels Migration
Deliverables:
- React-backed `GET /categories`, `GET /tags`, and `GET /price-levels` pages.
- Typed APIs and JSON mutation responses for create/edit/delete flows.
- Typed React replacement for category, tag, and price-level modal HTML
  rendering.
- React handling for client price-level assignment aligned with the existing
  `/api/v1/client-price-levels` contract.

Exit criteria:
- Categories, tags, and price levels pages work end to end through React-owned
  UI without depending on Tera modal fragments or flash-driven redirects.

### Phase 7: Vendors Migration
Deliverables:
- React-backed `GET /vendors` page preserving vendor list behavior, local user
  creation, vendor-user assignment, and vendor-user clearing.
- Typed APIs and JSON mutation responses for vendor and vendor-user workflows.
- Typed React replacement for vendor modal HTML rendering.

Exit criteria:
- The vendors page works end to end through React-owned UI without depending
  on Tera modal fragments or flash-driven redirects.

### Phase 8: Legacy Frontend Removal
Deliverables:
- Remove obsolete Tera page templates and fragments no longer used for React
  pages.
- Remove inline scripts and template-owned interaction code no longer needed at
  runtime.
- Remove temporary migration wrappers once all targeted pages are React-backed.
- Remove direct `tera` and `actix-web-flash-messages` dependencies from
  `pushkind-orders`.

Exit criteria:
- No targeted orders hub page depends on Tera-owned page markup, flash-message
  middleware, or page-specific inline scripts at runtime.
- The Store API continues to behave the same way as before the hub migration.

## Verification Strategy
- Add backend tests for built-HTML route selection, page-data DTOs, structured
  JSON mutation responses, and resource authorization boundaries.
- Add frontend unit tests for payload parsing, API clients, Bootstrap helpers,
  and local interactive UI behavior.
- Add frontend component or integration tests for orders, order details,
  products, categories, tags, price levels, vendors, no-access, and user-menu
  behavior.
- Use targeted manual verification for flows coupled to authentication,
  vendor-scoped access, product upload, and order approval editing.
- Explicitly verify that Store API routes remain unchanged by the hub UI
  migration.

## Required Commands
- `cargo build --all-features --verbose`
- `cargo test --all-features`
- `cargo clippy --all-features --tests -- -Dwarnings`
- `cargo fmt --all -- --check`
- `cd frontend && npm run typecheck`
- `cd frontend && npm run test`
- `cd frontend && npm run build`
