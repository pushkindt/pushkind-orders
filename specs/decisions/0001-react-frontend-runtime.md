# ADR 0001: Adopt Incremental React Frontend With Vite-Built Documents

## Status
Proposed

## Context
`pushkind-orders` currently renders its authenticated hub pages with Tera
templates and augments that markup with Bootstrap behaviors, modal fragments,
and flash-driven redirect flows.

The approved frontend migration goal is to move the hub UI to React while
preserving:
- the existing server-routed URLs
- the non-SPA navigation model
- Bootstrap styling
- Russian copy
- backend-owned authorization, validation, pricing, vendor scoping, approval,
  and persistence rules

The customer-facing Store API under `/api/v1/store/{hub_id}` is not part of
this migration and must remain backend-owned.

## Decision
- Keep Actix routes and server-side request handling as the source of truth for
  navigation, redirects, authentication, and authorization.
- Introduce React incrementally on the existing hub URLs.
- Do not introduce client-side routing.
- Place frontend source code under `frontend/`.
- Build frontend assets and HTML documents with Vite into `assets/dist/`.
- Let Rust serve built HTML documents after performing route-level access
  checks.
- Move hub page initialization to typed `/api/v1/...` JSON APIs instead of
  embedding more page data into server-generated HTML.
- Keep Tera only as a migration wrapper until React equivalents are shipped and
  verified.
- Keep flash-message middleware only until React-owned mutation flows replace
  redirect-based feedback.
- Leave the Store API contract and Store API routing model unchanged.

## Consequences

### Positive
- React can be introduced without rewriting the backend architecture.
- The migration can proceed incrementally by page and interaction.
- Built frontend artifacts are served directly by the Rust application.
- The final runtime model becomes clearer: Rust owns routes and APIs, React
  owns page UI.

### Negative
- The service will temporarily carry both Tera and React concerns.
- A Node-based frontend toolchain becomes part of local development and CI.
- Some endpoints and flows will temporarily exist in both legacy and migrated
  forms during rollout.

## Rejected Alternatives
- Full SPA rewrite:
  rejected because it conflicts with the approved spec and would widen scope
  beyond a frontend migration.
- Continue with Tera + modal fragments + flash redirects:
  rejected because it does not achieve the approved React migration target.
- Expand this migration to include the Store API:
  rejected because the approved spec keeps storefront behavior out of scope.
- Keep Rust assembling HTML document shells permanently:
  rejected because the target state explicitly gives frontend document
  ownership to Vite-built static HTML.
