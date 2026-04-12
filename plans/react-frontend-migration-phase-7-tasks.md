# Tasks: React Frontend Migration Phase 7

## Scope
This task file covers only Phase 7 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- cut over `GET /vendors` to a Vite-built React document
- add canonical typed vendor, vendor-user, and local-user APIs under
  `/api/v1/...`
- add structured JSON mutation responses for vendor create, edit, delete,
  vendor-user assignment, vendor-user clearing, and local user creation
- replace the current Tera-owned vendor modal fragment behavior with
  React-owned UI

Do not start Phase 8 in this file.
Phase 7 is complete only when the vendors page works end to end through
React-owned UI and active runtime behavior no longer depends on vendor modal
fragments, inline vendor-page scripts, or flash-driven redirects for vendor
workflows.

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)
- Migration plan:
  [../plans/react-frontend-migration.md](../plans/react-frontend-migration.md)
- Phase 6 task file:
  [../plans/react-frontend-migration-phase-6-tasks.md](../plans/react-frontend-migration-phase-6-tasks.md)
- Current vendor routes and services:
  [../src/routes/vendors.rs](../src/routes/vendors.rs)
  [../src/services/vendors.rs](../src/services/vendors.rs)
  [../src/forms/vendors.rs](../src/forms/vendors.rs)
- Current vendor DTOs:
  [../src/dto/vendors.rs](../src/dto/vendors.rs)
  [../src/dto/api.rs](../src/dto/api.rs)
- Current shared API layer:
  [../src/routes/api.rs](../src/routes/api.rs)
  [../src/services/api.rs](../src/services/api.rs)
- Current frontend shell and helpers:
  [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts)
  [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts)
  [../frontend/src/components/OrdersShell.tsx](../frontend/src/components/OrdersShell.tsx)
- Current vendor templates:
  [../templates/vendors/index.html](../templates/vendors/index.html)
  [../templates/vendors/add_vendor_modal.html](../templates/vendors/add_vendor_modal.html)
  [../templates/vendors/edit_vendor_modal.html](../templates/vendors/edit_vendor_modal.html)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-orders`.
- Treat the feature spec and migration plan as the source of truth.
- Assume Phase 6 is already complete:
  `GET /`,
  `GET /na`,
  `GET /order/{order_id}`,
  `GET /products`,
  `GET /categories`,
  `GET /tags`,
  and `GET /price-levels` are React-backed.
- `GET /vendors` still renders through Tera at the start of Phase 7.
- `GET /vendor/{vendor_id}/modal` still exists as an HTML fragment route at
  the start of Phase 7.
- `POST /vendors/add`,
  `POST /vendors/edit`,
  `POST /vendors/{vendor_id}/delete`,
  `POST /vendors/assign`,
  `POST /vendors/clear`,
  and `POST /users/add`
  still use redirect-plus-flash semantics at the start of Phase 7.
- Keep the Store API under `/api/v1/store/{hub_id}` unchanged.
- Do not introduce page-shaped bootstrap endpoints such as
  `/api/v1/vendors-page`
  or `/api/v1/vendor-management`.
- Do not introduce client-side routing. All full-page navigation must remain
  native.
- Do not remove `tera` or `actix-web-flash-messages` in this phase.

## What You Will Change In Phase 7
You will change only these repository areas:

- edit `src/dto/api.rs`
- edit `src/forms/vendors.rs`
- edit `src/services/api.rs`
- edit `src/services/vendors.rs`
- edit `src/routes/api.rs`
- edit `src/routes/vendors.rs`
- edit `src/lib.rs`
- edit `src/frontend.rs`
- edit `frontend/vite.config.ts`
- create `frontend/app/vendors.html`
- edit `frontend/src/lib/models.ts`
- edit `frontend/src/lib/api.ts`
- edit `frontend/src/lib/api.test.ts`
- create `frontend/src/entries/vendors.tsx`
- create `frontend/src/pages/VendorsPage.tsx`
- create any small vendor-only React components needed under
  `frontend/src/components/`
- edit `tests/api.rs`
- edit `README.md`
- edit `SPEC.md`

If you find yourself removing legacy vendor templates or deleting
`actix-web-flash-messages`/`tera`, stop. That belongs to Phase 8.

## Deliverables
- `GET /vendors` is served from a built frontend document and rendered by
  React.
- `GET /api/v1/vendors` and `GET /api/v1/vendors/{vendor_id}` return canonical
  typed vendor collection and detail resources.
- `GET /api/v1/users` returns the typed local-user list needed for vendor-user
  assignment and user creation flows.
- Vendor create/edit/delete flows use structured JSON mutation responses rather
  than flash-message redirects.
- Vendor-user assignment and clearing use structured JSON mutation responses
  rather than flash-message redirects.
- Local user creation from the vendors page uses a structured JSON mutation
  response rather than flash-message redirects.
- Validation copy for vendor forms is owned by `src/forms/vendors.rs` with
  Russian strings on validator annotations and `#[error("...")]` variants.
- The vendors page no longer depends at runtime on:
  `templates/vendors/index.html`,
  `templates/vendors/add_vendor_modal.html`,
  `templates/vendors/edit_vendor_modal.html`,
  or modal HTML fragment endpoints.

## Step 0: Confirm The Starting Point
Run these commands before you make any Phase 7 changes:

```bash
pwd
git status --short
sed -n '1,260p' src/routes/vendors.rs
sed -n '1,320p' src/services/vendors.rs
sed -n '1,260p' src/forms/vendors.rs
sed -n '1,320p' src/routes/api.rs
sed -n '1,320p' src/services/api.rs
sed -n '1,260p' src/dto/api.rs
sed -n '1,260p' src/dto/vendors.rs
sed -n '1,260p' templates/vendors/index.html
sed -n '1,220p' templates/vendors/add_vendor_modal.html
sed -n '1,220p' templates/vendors/edit_vendor_modal.html
```

Expected result before Phase 7 starts:
- the vendors page still renders through Tera
- vendor editing still depends on the modal HTML fragment route
- vendor and vendor-user mutations still redirect with flash messages
- there is no canonical `GET /api/v1/vendors/{vendor_id}` detail resource
- there is no canonical `GET /api/v1/users` resource for React vendor flows
- there is no built `frontend/app/vendors.html` document

## Task 1: Define Canonical Resource-Style Vendor Contracts
Goal:
introduce reusable React-facing vendor and user contracts without inventing
page-shaped bootstrap endpoints.

### 1.1 Expand `src/dto/api.rs`
Add or extend React-facing DTOs in
[../src/dto/api.rs](../src/dto/api.rs) for:

- vendor collection and detail resources
- local user list items used by vendor assignment UI
- vendor-user assignment views if the current vendor collection contract should
  expose them directly
- mutation success DTOs for vendor, vendor-user, and local-user workflows
- field-addressable mutation error DTOs where missing

Do not grow legacy Tera DTO modules with new React API contracts.

### 1.2 Collection And Detail Contract Requirements
Add canonical GET endpoints:

- `GET /api/v1/vendors`
- `GET /api/v1/vendors/{vendor_id}`
- `GET /api/v1/users`

These contracts must expose the fields needed by the React vendors page to:

- render the vendor list with current search and pagination behavior
- render local users with current vendor assignment state
- open edit UI for a specific vendor
- populate assignment controls without relying on template context or HTML

Do not add `/api/v1/vendors-page` or similar page-shaped endpoints.

### 1.3 DTO Tests
Add focused DTO tests covering:

- vendor collection/detail conversion
- local user list conversion for vendor assignment
- mutation error DTO mapping for vendor form errors

## Task 2: Move Validation Ownership Into `src/forms/vendors.rs`
Goal:
make the forms layer own Russian validation copy and typed payload conversion,
aligned with the already migrated services and previous orders phases.

### 2.1 Localize Vendor Forms
Update [../src/forms/vendors.rs](../src/forms/vendors.rs) so that:

- validator macros define Russian messages directly on fields
- `FormError` variants use `#[error("...")]` with Russian strings
- add, edit, assign, clear, and local-user forms all convert into strongly
  typed payloads
- field-level errors can be surfaced into the shared API mutation DTO shape

### 2.2 Keep The Route Boundary Pattern
Routes should continue to convert:

- `AddVendorForm -> AddVendorPayload`
- `EditVendorForm -> EditVendorPayload`
- `AssignVendorUserForm -> AssignVendorUserPayload`
- `ClearVendorUserForm -> ClearVendorUserPayload`
- `AddUserForm -> AddUserPayload`

before calling services.

Do not introduce a local service error type.

## Task 3: Add Backend Service And API Support For React Vendors
Goal:
add the reusable backend API surface while preserving existing authorization
rules and hub scoping.

### 3.1 Extend Vendor Services
Update [../src/services/vendors.rs](../src/services/vendors.rs) to expose:

- a lightweight access check for `GET /vendors`
- payload-based create, edit, assign, clear, and local-user service helpers
- vendor detail loading for `GET /api/v1/vendors/{vendor_id}`
- any collection helpers needed so the React API path does not depend on Tera
  page service DTOs

Preserve current admin-only authorization semantics for vendor workflows.

### 3.2 Extend Shared API Services
Update [../src/services/api.rs](../src/services/api.rs) to expose canonical:

- vendor collection data
- vendor detail data
- local user list data for vendor assignment

These should be reusable resource contracts, not vendor-page bootstrap
payloads.

### 3.3 Extend API Routes
Update [../src/routes/api.rs](../src/routes/api.rs) to add:

- `GET /api/v1/vendors`
- `GET /api/v1/vendors/{vendor_id}`
- `GET /api/v1/users`
- `POST /api/v1/vendors`
- `PUT /api/v1/vendors/{vendor_id}`
- `DELETE /api/v1/vendors/{vendor_id}`
- `POST /api/v1/vendors/assignments`
- `DELETE /api/v1/vendors/assignments/{user_id}` or an equally clear
  resource-style clear route
- `POST /api/v1/users`

Mutation endpoints must return structured JSON:

- success: `{ message, ... }`
- validation failure: `{ message, field_errors }`

Do not keep React dependent on redirecting HTML routes.

## Task 4: Cut Over `GET /vendors` To Built React HTML
Goal:
make the vendors page follow the same full-page serving pattern already used
for the other migrated orders pages.

### 4.1 Backend Cutover
Update:

- [../src/routes/vendors.rs](../src/routes/vendors.rs)
- [../src/frontend.rs](../src/frontend.rs)
- [../src/lib.rs](../src/lib.rs)

to:

- add `FRONTEND_VENDORS_DOCUMENT`
- serve `GET /vendors` from built `app/vendors.html` after a lightweight access
  check
- keep legacy vendor POST routes wired during the migration, but make the new
  React page independent from them

### 4.2 Vite Entry
Add:

- [../frontend/app/vendors.html](../frontend/app/vendors.html)
- [../frontend/src/entries/vendors.tsx](../frontend/src/entries/vendors.tsx)

and register them in
[../frontend/vite.config.ts](../frontend/vite.config.ts).

## Task 5: Build The React Vendors Page
Goal:
replace Tera-owned vendor management with a React page using the shared shell
and typed APIs.

### 5.1 Frontend API Layer
Extend:

- [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts)
- [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts)

with typed support for:

- vendor collection/detail resources
- local user resources
- vendor create/edit/delete
- vendor-user assignment and clearing
- local user creation

Keep auth-redirect handling aligned with the current frontend API helpers.

### 5.2 React Page Requirements
Create [../frontend/src/pages/VendorsPage.tsx](../frontend/src/pages/VendorsPage.tsx)
that preserves the current vendor page behavior:

- vendor search and pagination
- add vendor flow
- edit vendor flow
- delete vendor flow
- local user creation from the vendors page
- vendor-user assignment
- vendor-user clearing

The page must:

- use `OrdersShell`
- preserve Russian copy
- keep native full-page navigation
- own modal state in React
- consume only typed JSON responses
- avoid any runtime dependency on HTML fragments or inline template scripts

### 5.3 Frontend Tests
Add focused tests for:

- vendor API parsers/helpers
- vendor page behavior or extracted helpers
- assignment and clearing flows at the component/helper level

## Task 6: Update Docs
Goal:
make the service docs reflect the post-Phase-7 runtime accurately.

Update:

- [../SPEC.md](../SPEC.md)
- [../README.md](../README.md)

so they state that:

- `GET /vendors` is React-owned
- vendor and local-user React-facing APIs are resource-style under `/api/v1/...`
- vendor, vendor-user, and local-user mutations use structured JSON responses
- the remaining legacy vendor HTML endpoints are transitional and no longer
  required by the active vendors page runtime

## Task 7: Verify The Phase
Run the required checks after implementation:

```bash
cargo build --all-features --verbose
cargo test --all-features
cargo clippy --all-features --tests -- -Dwarnings
cargo fmt --all -- --check
cd frontend && npm run typecheck
cd frontend && npm run test
cd frontend && npm run build
```

Manual verification checklist:

- open `/vendors` as an admin user
- verify the page renders through the React shell
- create a vendor
- edit a vendor
- delete a vendor
- create a local user
- assign a local user to a vendor
- clear a local user’s vendor assignment
- confirm field-level validation errors render inline for invalid submissions
- confirm no vendor flow depends on a full-page refresh caused by flash-message
  redirects

## Exit Checklist
Phase 7 is done only when all of the following are true:

- `GET /vendors` serves built React HTML
- the vendors page no longer depends on `GET /vendor/{vendor_id}/modal`
- React uses typed `/api/v1/vendors`, `/api/v1/vendors/{vendor_id}`, and
  `/api/v1/users` contracts rather than page-shaped bootstrap payloads
- vendor, vendor-user, and local-user mutations return structured JSON
- backend and frontend tests covering the new vendor path are in place
- `SPEC.md` and `README.md` match the implemented runtime
- required verification commands pass
