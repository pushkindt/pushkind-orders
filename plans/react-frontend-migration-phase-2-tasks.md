# Tasks: React Frontend Migration Phase 2

## Scope
This task file covers only Phase 2 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- introduce the shared React shell and user-menu foundation
- add typed `GET /api/v1/iam` and `GET /api/v1/no-access`
- replace the shared `not_assigned` usage with a local React-backed `/na`
- serve only the orders no-access page from a built Vite HTML document
- keep all current Tera-owned hub pages on their existing runtime path

Do not start Phase 3, Phase 4, Phase 5, Phase 6, Phase 7, or Phase 8 in this
file. Phase 2 is complete only when `/na` is served from a Vite-built document
with a React-owned shell, while `GET /`, `GET /order/{order_id}`, `GET
/products`, `GET /categories`, `GET /tags`, `GET /price-levels`, and `GET
/vendors` still render through the current Tera templates.

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)
- Migration plan:
  [../plans/react-frontend-migration.md](../plans/react-frontend-migration.md)
- Phase 1 task file:
  [../plans/react-frontend-migration-phase-1-tasks.md](../plans/react-frontend-migration-phase-1-tasks.md)
- Current backend asset helper:
  [../src/frontend.rs](../src/frontend.rs)
- Current server wiring:
  [../src/lib.rs](../src/lib.rs)
- Current Tera navbar contract to preserve:
  [../templates/components/navigation.html](../templates/components/navigation.html)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-orders`.
- Treat the feature spec and migration plan as the source of truth.
- Assume Phase 1 is already complete:
  `frontend/` exists,
  Vite builds into `assets/dist/`,
  and `src/frontend.rs` can open built HTML documents.
- Keep the current auth and authorization behavior for all existing hub pages.
- Keep the Store API under `/api/v1/store/{hub_id}` unchanged.
- Do not add orders dashboard resource APIs, order details resource APIs, or
  JSON mutation endpoints in this phase.
- Do not remove Tera templates, modal fragments, or flash-message middleware
  in this phase.

## What You Will Change In Phase 2
You will change only these repository areas:

- create `src/dto/api.rs`
- create `src/services/api.rs`
- create `src/routes/aux.rs`
- edit `src/dto/mod.rs`
- edit `src/services/mod.rs`
- edit `src/routes/mod.rs`
- edit `src/routes/api.rs`
- edit `src/frontend.rs`
- edit `src/lib.rs`
- create `frontend/app/no-access.html`
- create shared shell files under `frontend/src/components/`
- create shared API and shell helper files under `frontend/src/lib/`
- create `frontend/src/entries/no-access.tsx`
- create `frontend/src/pages/NoAccessPage.tsx`
- append shell styles to `frontend/src/styles/foundation.css`
- update `frontend/vite.config.ts`
- update `README.md`

If you find yourself editing `src/routes/main.rs`, `src/routes/orders.rs`,
`src/routes/products.rs`, `src/routes/categories.rs`, `src/routes/tags.rs`,
`src/routes/price_levels.rs`, `src/routes/vendors.rs`, or any Tera template
other than using them as a reference for current labels and layout behavior,
stop. That belongs to later phases.

## Deliverables
- `GET /api/v1/iam` exists and returns typed shell data for authenticated
  users.
- `GET /api/v1/no-access` exists and returns typed page data for the local
  no-access page.
- `GET /na` is owned by `pushkind-orders`, not by
  `pushkind_common::routes::not_assigned`.
- `GET /na` is served from `assets/dist/app/no-access.html` after auth.
- The no-access page uses a shared React shell with a reusable dropdown that
  matches the migrated Pushkind services.
- Auth menu loading happens after shell data is available, and auth-menu
  failure still leaves `Домой` and logout available.
- `GET /`, `GET /order/{order_id}`, `GET /products`, `GET /categories`, `GET
  /tags`, `GET /price-levels`, and `GET /vendors` still render through Tera.

## Step 0: Confirm The Starting Point
Run these commands before making Phase 2 changes:

```bash
pwd
git status --short
find frontend/src -maxdepth 3 -type f | sort
sed -n '1,220p' src/frontend.rs
sed -n '1,240p' src/routes/api.rs
sed -n '1,220p' templates/components/navigation.html
sed -n '1,220p' README.md
```

Expected result before Phase 2 starts:
- `frontend/` exists from Phase 1
- there is no `frontend/app/no-access.html`
- there is no `frontend/src/entries/no-access.tsx`
- there is no `src/routes/aux.rs`
- there is no `src/dto/api.rs`
- there is no `src/services/api.rs`
- `/na` is still provided by `pushkind_common::routes::not_assigned`

## Task 1: Extend The Frontend Asset Helper For The No-Access Document

### 1.1 Edit `src/frontend.rs`
Open [../src/frontend.rs](../src/frontend.rs).

Add a second built-document constant next to `FRONTEND_INDEX_DOCUMENT`:

```rust
/// Built HTML document backing `GET /na`.
pub const FRONTEND_NO_ACCESS_DOCUMENT: &str = "app/no-access.html";
```

Requirements:
- Do not rename or remove the Phase 1 helpers.
- Do not change `FRONTEND_DIST_DIR`.
- Do not add manifest-specific no-access logic yet; the same generic helpers
  already support multiple HTML documents.

### 1.2 Sanity-check The Helper
Run:

```bash
rg -n "FRONTEND_(INDEX|NO_ACCESS)_DOCUMENT" src/frontend.rs
```

Expected result:
- both built HTML document constants exist in `src/frontend.rs`

## Task 2: Add Typed Shell And No-Access DTOs

### 2.1 Create `src/dto/api.rs`
Create [../src/dto/api.rs](../src/dto/api.rs).

It must define these serializable DTOs:

- `CurrentUserDto`
- `NavigationItemDto`
- `IamDto`
- `NoAccessPageDto`

Required fields:

`CurrentUserDto`
- `email: String`
- `name: String`
- `hub_id: i32`
- `roles: Vec<String>`

`NavigationItemDto`
- `name: &'static str`
- `url: &'static str`

`IamDto`
- `current_user: CurrentUserDto`
- `home_url: String`
- `navigation: Vec<NavigationItemDto>`
- `local_menu_items: Vec<NavigationItemDto>`

`NoAccessPageDto`
- `current_user: CurrentUserDto`
- `home_url: String`
- `required_role: &'static str`

Implementation requirements:
- add `impl From<&AuthenticatedUser> for CurrentUserDto`
- keep field names aligned with the migrated services:
  `current_user`,
  `home_url`,
  `local_menu_items`,
  and `hub_id`
- do not add `alerts`
- do not add `logout_action`; keep logout handling aligned with the existing
  React pattern in the migrated services

Navigation requirements:
- preserve the current Tera navbar labels and order from
  [../templates/components/navigation.html](../templates/components/navigation.html)
- use exactly these labels and URLs for the main hub tabs:
  `Заказы` -> `/`
  `Товары` -> `/products`
  `Категории` -> `/categories`
  `Цены` -> `/price-levels`
  `Теги` -> `/tags`
- include `Поставщики` -> `/vendors` only when the user has
  `crate::ADMIN_ACCESS_ROLE`
- keep `local_menu_items` empty in Phase 2

Add unit tests that cover:
- `CurrentUserDto::from(&AuthenticatedUser)`
- the admin-only presence of the `Поставщики` navigation item
- the absence of service navigation for users who do not have
  `crate::SERVICE_ACCESS_ROLE`

### 2.2 Edit `src/dto/mod.rs`
Open [../src/dto/mod.rs](../src/dto/mod.rs).

Add:

```rust
pub mod api;
```

Do not remove any existing DTO exports in this phase.

## Task 3: Add The Backend Service Layer For Shell And No-Access Data

### 3.1 Create `src/services/api.rs`
Create [../src/services/api.rs](../src/services/api.rs).

This file must expose:

- `get_shell_data(user: &AuthenticatedUser, common_config: &CommonServerConfig) -> ServiceResult<IamDto>`
- `get_no_access_data(user: &AuthenticatedUser, common_config: &CommonServerConfig) -> NoAccessPageDto`

Implementation requirements:
- `get_shell_data` must intentionally work for authenticated users who do not
  have the `orders` role, because the local `/na` page also needs shell data
- `get_shell_data` must not call repository methods in Phase 2
- `get_shell_data` must only compute shell data from:
  the authenticated user,
  roles,
  and `CommonServerConfig`
- `home_url` must come from `common_config.auth_service_url`
- `local_menu_items` must be `Vec::new()` in Phase 2
- `navigation` must match the rules defined in Task 2
- `required_role` in `get_no_access_data` must be `crate::SERVICE_ACCESS_ROLE`

Add unit tests that cover:
- service-role user gets the expected shell navigation
- admin user with the service role gets the `Поставщики` navigation item
- authenticated user without the service role still gets shell data, but with
  an empty navigation list
- no-access DTO returns the current user, auth home URL, and required role

### 3.2 Edit `src/services/mod.rs`
Open [../src/services/mod.rs](../src/services/mod.rs).

Add:

```rust
pub mod api;
```

Do not change existing access helpers in this phase.

## Task 4: Add The Backend Routes For `/na`, `/api/v1/iam`, And `/api/v1/no-access`

### 4.1 Edit `src/routes/api.rs`
Open [../src/routes/api.rs](../src/routes/api.rs).

Add two new handlers:

- `GET /v1/iam`
- `GET /v1/no-access`

Behavior requirements:
- `GET /api/v1/iam` returns the `IamDto` from `services::api::get_shell_data`
- `GET /api/v1/no-access` returns the `NoAccessPageDto` from
  `services::api::get_no_access_data`
- both endpoints must use `AuthenticatedUser`
- both endpoints must use `web::Data<CommonServerConfig>`
- both endpoints must return JSON
- both endpoints must log and return `500` on unexpected server failure
- neither endpoint should require `SERVICE_ACCESS_ROLE`

Keep the existing endpoints unchanged in this phase:
- `GET /api/v1/orders`
- `GET /api/v1/client-price-levels`
- `PUT /api/v1/client-price-levels`

Do not reshape any existing orders resource payloads in Phase 2.

### 4.2 Create `src/routes/aux.rs`
Create [../src/routes/aux.rs](../src/routes/aux.rs).

This file must define the local `GET /na` handler for `pushkind-orders`.

Behavior requirements:
- the route name can remain `not_assigned` for continuity with existing
  wiring, but it must now live in `pushkind-orders`
- the handler must use the built HTML helper from `src/frontend.rs`
- the handler must serve `FRONTEND_NO_ACCESS_DOCUMENT`
- missing built file must return `503 Service Unavailable` with a clear message
  telling the developer to run `cd frontend && npm run build`
- unexpected file-open errors must be logged and return `500`
- keep the handler thin; do not introduce a wrapper service function for a
  single `open_frontend_html` call

### 4.3 Edit `src/routes/mod.rs`
Open [../src/routes/mod.rs](../src/routes/mod.rs).

Add:

```rust
pub mod aux;
```

### 4.4 Edit `src/lib.rs`
Open [../src/lib.rs](../src/lib.rs).

Make these changes:

1. Stop importing `not_assigned` from `pushkind_common::routes`.
2. Import the new local route from `crate::routes::aux::not_assigned`.
3. Import `api_v1_iam` and `api_v1_no_access` from `crate::routes::api`.
4. Keep the Store API scope exactly as it is.
5. Keep the Tera-backed hub routes exactly as they are.
6. Register the new API endpoints inside the existing `/api` scope.
7. Register the local `not_assigned` service instead of the shared one.

Do not do any other server refactor in this phase.
In particular:
- do not split `ServerConfig` here
- do not change the `run` function shape here
- do not remove Tera or flash middleware here

### 4.5 Verify The Backend Surface
Run:

```bash
rg -n "api_v1_iam|api_v1_no_access|not_assigned" src
cargo test --all-features services::api::tests -- --nocapture
cargo build --all-features --verbose
```

Expected result:
- `/api/v1/iam` and `/api/v1/no-access` exist
- `src/routes/aux.rs` exists and exports the local `/na`
- the backend compiles before you touch the React page

## Task 5: Add The Shared React Shell Foundation
This shell is introduced on `/na` first. The main orders pages will reuse it
in later phases.

### 5.1 Create `frontend/app/no-access.html`
Create [../frontend/app/no-access.html](../frontend/app/no-access.html).

Requirements:
- follow the same structure as `frontend/app/index.html`
- keep Bootstrap CSS and Bootstrap Icons includes
- keep the Bootstrap bundle script
- mount into `<div id="react-root"></div>`
- load `frontend/src/entries/no-access.tsx`
- set a distinct document title such as `Orders No Access`

### 5.2 Update `frontend/vite.config.ts`
Open [../frontend/vite.config.ts](../frontend/vite.config.ts).

Add a second HTML entry to `rollupOptions.input`:

```ts
"app/no-access.html": resolve(__dirname, "app/no-access.html")
```

Do not change the build output directory, manifest location, or asset naming
pattern in this phase.

### 5.3 Create `frontend/src/lib/models.ts`
Create [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts).

Define the frontend models aligned with the migrated services:

- `NavigationItem`
- `UserMenuItem`
- `CurrentUser`
- `ShellData`
- `NoAccessData`

Field requirements:
- `CurrentUser.hubId` must map from backend `hub_id`
- `ShellData.localMenuItems` must map from backend `local_menu_items`
- `ShellData.currentUser` must map from backend `current_user`
- `NoAccessData.requiredRole` must map from backend `required_role`

### 5.4 Create `frontend/src/lib/api.ts`
Create [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts).

Keep it aligned with the shared pattern already used in the migrated services.

It must provide:

- strict response parsing helpers
- `fetchShellData()`
- `fetchNoAccessData()`
- `fetchHubMenuItems(authBaseUrl: string, hubId: number)`

Parsing requirements:
- throw explicit errors like:
  `Invalid API response: expected string at ...`
  and `Invalid API response: expected number at ...`
- map `hub_id` to `hubId`
- map `current_user` to `currentUser`
- map `local_menu_items` to `localMenuItems`
- map `required_role` to `requiredRole`

Request requirements:
- send `Accept: application/json`
- use `credentials: "include"`
- use `cache: "no-store"`
- detect redirected non-JSON auth responses before JSON parsing and navigate to
  the redirect target instead of throwing a generic JSON parse error
- keep the auth menu endpoint derived from
  `${homeUrl}/api/v1/hubs/${hubId}/menu-items`

### 5.5 Create `frontend/src/lib/useOrdersShell.ts`
Create [../frontend/src/lib/useOrdersShell.ts](../frontend/src/lib/useOrdersShell.ts).

It must:
- fetch shell data first
- expose loading, ready, and error states
- start auth menu hydration only after the shell data is ready
- keep rendering possible even if auth menu hydration fails
- log a warning and fall back to local items when auth menu loading fails

This is important:
- auth slowness or auth-menu failure must not blank the orders no-access page
- the page should render as soon as the local shell data is ready

### 5.6 Create `frontend/src/components/UserMenuDropdown.tsx`
Create [../frontend/src/components/UserMenuDropdown.tsx](../frontend/src/components/UserMenuDropdown.tsx).

This component must be reusable and aligned with the existing migrated
services.

Behavior requirements:
- local items render first
- fetched auth-menu items render next
- logout is always last
- items that match the logout URL must not appear twice
- `Домой` should use a house icon
- `Настройки` should use a gear icon
- fallback icon for other items can remain the generic grid icon
- no spacer lines between ordinary menu items
- keep the compact Bootstrap dropdown look from the existing services

Also add a unit test, for example
[../frontend/src/components/UserMenuDropdown.test.tsx](../frontend/src/components/UserMenuDropdown.test.tsx),
covering:
- local items before fetched items
- logout still last even if fetched items contain a logout item

### 5.7 Create `frontend/src/components/OrdersNavbar.tsx`
Create [../frontend/src/components/OrdersNavbar.tsx](../frontend/src/components/OrdersNavbar.tsx).

This navbar must preserve the current Tera navbar contract from
[../templates/components/navigation.html](../templates/components/navigation.html).

Requirements:
- brand label remains `Orders`
- nav item order remains:
  `Заказы`,
  `Товары`,
  `Категории`,
  `Цены`,
  `Теги`,
  and optionally `Поставщики`
- `Поставщики` only renders when present in the `navigation` payload
- keep support for an optional `search` slot even though `/na` does not use it
- pass `[{ name: "Домой", url: homeUrl }, ...localMenuItems]` into the user
  dropdown so `Домой` always remains available even if auth menu hydration
  fails

### 5.8 Create `frontend/src/components/OrdersShell.tsx`
Create [../frontend/src/components/OrdersShell.tsx](../frontend/src/components/OrdersShell.tsx).

Responsibilities:
- render the shared navbar
- expose a reusable shell wrapper for later orders pages
- initialize Bootstrap popovers if present
- expose `window.showFlashMessage` using the same Bootstrap modal pattern used
  by the other migrated services

Requirements:
- keep the implementation React-safe
- clean up any Bootstrap objects on unmount
- use the same alert styling direction as the migrated services, not a custom
  one-off implementation

### 5.9 Create `frontend/src/components/OrdersShellFatalState.tsx`
Create [../frontend/src/components/OrdersShellFatalState.tsx](../frontend/src/components/OrdersShellFatalState.tsx).

This component should render a centered fatal-state card with:
- a small `Orders` eyebrow label
- a short title
- the supplied error message

Keep it intentionally small and reusable; it will be reused by later pages.

## Task 6: Add The React No-Access Page

### 6.1 Create `frontend/src/pages/NoAccessPage.tsx`
Create [../frontend/src/pages/NoAccessPage.tsx](../frontend/src/pages/NoAccessPage.tsx).

This page must:
- load shell state through `useOrdersShell`
- load page data through `fetchNoAccessData`
- render nothing while required data is still loading
- render `OrdersShellFatalState` if shell loading fails
- render `OrdersShellFatalState` if no-access data loading fails
- render the no-access card inside `OrdersShell` once both are ready

Content requirements:
- Russian copy
- show the current user name
- show the current user email
- show the missing required role
- include a `Домой` button linking to `homeUrl`
- include a logout form posting to `/logout`

Keep the page visually aligned with the no-access pages already used in the
other migrated Pushkind services.

### 6.2 Create `frontend/src/entries/no-access.tsx`
Create [../frontend/src/entries/no-access.tsx](../frontend/src/entries/no-access.tsx).

It should only mount `NoAccessPage` using the existing `mountPage` helper.

### 6.3 Append Shell Styles To `frontend/src/styles/foundation.css`
Open [../frontend/src/styles/foundation.css](../frontend/src/styles/foundation.css).

Append only the styles needed for:
- the centered fatal-state shell
- the small eyebrow label
- the shared shell content area height

Use service-local class names such as:
- `.orders-foundation-shell`
- `.orders-foundation-card`
- `.orders-foundation-eyebrow`
- `.orders-shell-content`

Do not remove the Phase 1 placeholder styles yet.

## Task 7: Update Documentation For The New `/na` Runtime

### 7.1 Edit `README.md`
Open [../README.md](../README.md).

Update the frontend/runtime section so it explains:
- Phase 2 introduces a built React document for `GET /na`
- the main hub pages still render through Tera in Phase 2
- `cargo run` still starts the service without a frontend build, but `/na`
  depends on `assets/dist/app/no-access.html`
- if the frontend build has not been run, `/na` returns a clear error instead
  of silently falling back to Tera

Do not claim that `GET /` already depends on built frontend assets in Phase 2.

## Task 8: Verify Phase 2
Run these commands from `pushkind-orders` unless noted otherwise:

1. `cd frontend && npm run typecheck`
2. `cd frontend && npm run test`
3. `cd frontend && npm run build`
4. `cargo build --all-features --verbose`
5. `cargo test --all-features --verbose`
6. `cargo clippy --all-features --tests -- -Dwarnings`
7. `cargo fmt --all -- --check`

Manual checks:
- open `/na` as an authenticated user without the `orders` role and confirm it
  renders the local React page
- confirm the page still shows `Домой` and `Выйти` if auth menu loading fails
- confirm the user dropdown keeps local items first and logout last
- confirm `/`, `/order/{order_id}`, `/products`, `/categories`, `/tags`,
  `/price-levels`, and `/vendors` still use the existing Tera runtime path
- confirm the Store API under `/api/v1/store/{hub_id}` behaves the same as
  before

## Phase 2 Exit Checklist
Mark Phase 2 done only if all of the following are true:

- `GET /api/v1/iam` exists and returns typed shell data.
- `GET /api/v1/no-access` exists and returns typed no-access page data.
- `GET /na` is owned by `pushkind-orders`, not `pushkind_common`.
- `/na` is served from a Vite-built HTML document.
- The no-access page uses the shared React shell and reusable dropdown.
- Local menu items render before fetched auth-menu items.
- Logout is always last in the dropdown.
- `Домой` remains available even when auth menu hydration fails.
- The main orders hub pages are still rendered by Tera.
- README documents the Phase 2 split correctly.

## Explicit Non-Goals For This Task File
Do not do these here:

- cut `GET /` over to a built HTML document
- add `GET /api/v1/orders/{order_id}`
- add `GET /api/v1/products`, `GET /api/v1/categories`, `GET /api/v1/tags`,
  `GET /api/v1/price-levels`, `GET /api/v1/vendors`, or `GET /api/v1/users`
- migrate any Tera page template to React
- migrate any HTML fragment modal to React
- convert any POST/PUT hub mutations to JSON responses
- remove `tera`
- remove `actix-web-flash-messages`
- change the Store API contract
- refactor server settings or split `ServerConfig` in this phase
