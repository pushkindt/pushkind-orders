# Vendor Scoping & Ownership — Implementation Phases

This plan covers implementing vendor-scoped hub access for users with `VENDOR_ACCESS_ROLE` and linking products + orders to vendors.

## Goal Summary

- Hub users with `VENDOR_ACCESS_ROLE` are assigned to a vendor entity (`vendors` + `vendor_user`).
- Products can be owned by a vendor (`products.vendor_id`).
- Orders that contain vendor-owned products are linked to vendors (`vendor_order`).
- Vendor users can only see/manage products and orders associated with their vendor.
- Store customers see product vendor names and can filter products by vendor.
- Vendor users can view tags/categories/price-levels read-only (no create/update/delete).

## Non-Goals (for the first iteration)

- Restricting Store catalog access by vendor (store customers still browse/order the full catalog; vendor fields + vendor filtering are additive).
- Adding vendor-specific pricing rules (beyond existing price levels).
- Cross-hub vendor sharing (vendors are strictly hub-scoped).

## Preconditions / Dependencies

- Migration exists: `migrations/2026-01-22-103021_add-vendor`.
- Roles exist in the auth service:
  - `SERVICE_ACCESS_ROLE` = `orders`
  - `VENDOR_ACCESS_ROLE` = `orders_vendor`
- Requirement: there can be only one vendor per vendor user (enforce in DB and/or service layer; schema as-is allows multiples).

## Phase 0 — Decisions & UX Semantics (spec-level)

**Decisions to lock down before coding:**

1. **Vendor membership cardinality**
   - One vendor per vendor user (required): vendor users must be assigned to exactly one vendor and must not be assigned to multiple vendors.
2. **Order vendor constraint**
   - Orders must not contain products from multiple vendors.
   - An order is either vendorless (all `vendor_id` null) or belongs to exactly one vendor (all `vendor_id` equal).
3. **Vendor management permissions**
   - Only hub operators with `SERVICE_ACCESS_ROLE` can create vendors and assign/unassign users to vendors.
4. **Product vendor assignment rules**
   - Vendor users: vendor forced to their vendor on create; cannot change on edit.
   - Admin users: can set/clear vendor on create/edit.
5. **Store catalog vendor UX**
   - Product cards/detail should display `vendorName` when present.
   - Vendor filtering is by `vendorId` (not by vendor name).
   - Vendor list source: `GET /api/v1/store/{hub_id}/vendors`.
6. **Vendor user permissions surface**
   - Vendor users are product/order scoped for write operations.
   - Vendor users can view (read-only) the tags/categories/price-level pages, but must not be able to create/update/delete them.

**Exit criteria**

- These decisions are recorded in `SPEC.md` (or explicitly referenced from this plan).

## Phase 1 — Database & Diesel Schema Alignment

**Why:** The migration introduces `vendor_order`, but Diesel schema and down migration need to be consistent so code/tests can compile and migrations can be applied/rolled back in CI.

**Tasks**

- Ensure `vendor_order` exists in `src/schema.rs` (it is currently missing).
- Add Diesel joinables/allow-tables for `vendor_order`.
- Add Diesel models for:
  - `vendor_user` (mapping table)
  - `vendor_order` (mapping table)
- Enforce one-vendor-per-user at the DB layer:
  - Add a unique constraint/index on `vendor_user.user_id` (so a user cannot be assigned to multiple vendors).
- Enforce one-vendor-per-order at the DB layer:
  - Add a unique constraint/index on `vendor_order.order_id` (so an order cannot be linked to multiple vendors).
- Update migration rollback:
  - `migrations/2026-01-22-103021_add-vendor/down.sql` should drop `vendor_order` as well.
- Extend `src/models/product.rs` insert/update payloads to support persisting `vendor_id` (DB column already exists).

**Exit criteria**

- `cargo build --all-features` succeeds.
- Integration test harness can apply migrations cleanly (up + down where used).

## Phase 2 — Repository Layer (Diesel)

**Why:** Vendor scoping must be enforced at query time; services should not do “filter in memory” for authorization.

**Tasks**

- Define repository traits in `src/repository/mod.rs` and implement in `src/repository/vendor.rs`:
  - `VendorReader` / `VendorWriter` for CRUD vendors (`vendors` table).
  - `VendorUserReader` / `VendorUserWriter` for assigning/unassigning users (`vendor_user` table).
  - `VendorOrderWriter` (and optionally reader) for managing associations (`vendor_order` table).
  - Helper query: resolve vendor scope for a hub user:
    - `get_vendor_for_user(hub_id, user_id)` → `Option<VendorId>` (error if multiple rows exist).
- Extend product + order queries with a vendor filter:
  - `list_products(query)` should optionally constrain by `products.vendor_id`.
  - `list_orders(query)` should optionally constrain to orders present in `vendor_order` for the vendor(s).
- Ensure Store product reads can expose vendor names efficiently:
  - `GET /api/v1/store/{hub_id}/products` and `GET /api/v1/store/{hub_id}/products/{product_id}` should return `vendorId` and `vendorName` (without N+1 vendor lookups).
  - `GET /api/v1/store/{hub_id}/vendors` lists vendors for filter UI.
- Ensure hub scoping is enforced via joins:
  - `vendors.hub_id` must match `users.hub_id` when resolving assignments.
  - `products.hub_id` must match `vendors.hub_id` when assigning vendor_id.

**Exit criteria**

- Repository functions return correct vendor-filtered results with hub scoping guarantees.
- Repository-level tests (unit/integration) cover:
  - “user in hub A cannot resolve vendor in hub B”
  - “vendor filter excludes non-owned products/orders”

## Phase 3 — Service Layer: Authorization + Business Rules

**Why:** Current services call `ensure_role(user, SERVICE_ACCESS_ROLE)` and will block vendor users; we need a unified access model and vendor scoping rules.

**Tasks**

- Introduce a service-level access resolver, e.g.:
  - `HubAccessScope::Admin` when user has `SERVICE_ACCESS_ROLE`
  - `HubAccessScope::Vendor { vendor_id }` when user has `VENDOR_ACCESS_ROLE` (and vendor membership exists)
- Update services to accept both admin and vendor scopes where appropriate:
  - `src/services/products.rs`
    - List products: apply vendor filter for vendor users.
    - Create/edit product:
      - Vendor users: force `vendor_id` to their vendor; reject attempts to set/clear vendor.
      - Admin users: allow setting/clearing vendor_id.
  - `src/services/orders.rs`
    - List orders: apply vendor filter for vendor users.
    - Order details: ensure the order belongs to the user’s vendor (orders are single-vendor, so all line items are safe to show).
    - Approvals/edits: restrict vendor users to orders belonging to their vendor.
- Ensure Store order creation links orders to vendors:
  - In the “create store order” service, validate vendor consistency across ordered products (must be vendorless or exactly one vendor).
  - When vendor-owned, insert the single `vendor_order` row for `(vendor_id, order_id)` (idempotent).
- Add clear error responses:
  - Vendor user with no vendor assignment should receive a deterministic error (e.g. `403 Forbidden` or “not assigned”) rather than an internal error.
  - Vendor user attempting to create/update/delete tags/categories/price levels should receive `403 Forbidden` (admin-only).

**Exit criteria**

- Service unit tests (mock repo) cover:
  - Admin sees everything
  - Vendor sees only vendor-owned data
  - Vendor cannot mutate non-owned data
  - Order creation populates vendor_order for vendor-owned products

## Phase 4 — HTTP: Routes, Templates, and Hub JSON API

**Why:** Vendor users need an entry point and a usable UI/API surface; handlers should remain thin and delegate scoping to services.

**Tasks**

- Add vendor management UI (admin only; `SERVICE_ACCESS_ROLE`):
  - Vendors list/create/edit.
  - Assign/unassign hub users to vendors.
  - UI should prevent assigning a user to multiple vendors.
- Update existing pages to behave correctly under vendor scope:
  - `/products`: vendor users see only their products; forms hide vendor selectors.
  - `/` and `/order/{id}`: vendor users see only orders for their vendor; order detail contains only that vendor’s products (orders are single-vendor).
- Keep hub-wide configuration writes admin-only:
  - Vendor users can view tags (`/tags`), categories (`/categories`), and price levels (`/price-levels`) read-only.
  - Hide/disable create/edit/delete controls in templates and enforce write restrictions in the service layer.
- Store API catalog:
  - Add `vendorId` filter to `GET /api/v1/store/{hub_id}/products`.
  - Include `vendorId` + `vendorName` in `StoreProduct` responses.
  - Add `GET /api/v1/store/{hub_id}/vendors` to populate vendor filter options.
- Hub JSON API:
  - Ensure vendor users can call `GET /api/v1/orders` but get vendor-filtered results.
  - Explicitly keep admin-only endpoints (e.g. price-level assignment) restricted to `SERVICE_ACCESS_ROLE`.

**Exit criteria**

- Manual flow works end-to-end:
  1. Admin creates vendor and assigns a vendor user.
  2. Vendor user logs in and sees only their products/orders.
  3. Store customer places an order containing vendor product → vendor user can see that order.

## Phase 5 — Backfill & Data Migration Strategy

**Why:** Existing deployments may have products/orders created before vendor ownership existed.

**Tasks**

- Product ownership backfill:
  - Decide how to assign `vendor_id` for existing products (script/manual/admin UI).
- Order/vendor association backfill:
  - One-time job that inserts `vendor_order` rows by joining `order_products.product_id` → `products.vendor_id` (where product_id exists and vendor_id is non-null).
  - Detect and handle orders that would violate the single-vendor constraint (multiple distinct `vendor_id` values across line items), for example by:
    - refusing to backfill those orders and reporting them for manual remediation, or
    - adjusting product vendor assignments to eliminate mixed-vendor orders.
  - Handle historical rows where product_id is null or product deleted.
- Make the backfill idempotent (safe to re-run).

**Exit criteria**

- A documented, repeatable procedure exists for migrating an existing hub to vendor scoping.

## Phase 6 — Test Coverage, QA, and Rollout

**Tasks**

- Add/extend tests:
  - Unit tests for access resolver and service filtering.
  - Integration tests covering migrations + vendor scoping behavior.
- Run quality gates:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-features --tests -- -Dwarnings`
  - `cargo test --all-features --verbose`
- Rollout checklist:
  - Apply DB migration.
  - Provision role `orders_vendor` in auth service.
  - Create vendors and assign users.
  - (Optional) run backfill job.
  - Verify vendor user access in staging.

**Exit criteria**

- All checks pass and a rollback plan is documented (including DB down migration constraints).
