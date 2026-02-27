# Plan: Store OTP IP Rate Limiter

Date: 2026-02-27
Status: Stable
Feature: `specs/features/store-otp-ip-rate-limiter.md`

## Scope

Implement an always-enabled, in-memory IP-based rate limiter for:

- `POST /api/v1/store/{hub_id}/auth/otp`

Controlled only by constants:

- `MAX_REQUESTS` (u32)
- `WINDOW_SECONDS` (u64)
- `TRUST_FORWARDED_HEADERS` (bool)

No changes to `ServerConfig`.

## Acceptance Criteria

- Requests exceeding the per-IP limit return:
  - `429 Too Many Requests`
  - `Retry-After: <seconds>` header (rounded up)
  - JSON body `{ "error": "rate limit exceeded" }`
- Requests under the limit proceed unchanged and still use the existing per-phone throttling.
- Client IP extraction:
  - Uses `peer_addr().ip()` when `TRUST_FORWARDED_HEADERS = false`
  - Uses `connection_info().realip_remote_addr()` when `TRUST_FORWARDED_HEADERS = true`
- No panics / `unwrap` in production code paths.
- Unit tests cover limiter behavior and `Retry-After` computation.

## Implementation Steps

1. Add a small rate limiter module under routes
   - Create `src/routes/rate_limit.rs`
   - Implement a thread-safe limiter (e.g., `Mutex<HashMap<IpAddr, VecDeque<Instant>>>`)
   - Provide `check(req: &HttpRequest) -> Result<(), RateLimitExceeded { retry_after_seconds }>`
   - Ensure state cleanup (drop entries older than the window) happens on each check.

2. Define constants (no config)
   - Decide constant values and place them near the limiter implementation (or a small dedicated module).
   - Default `TRUST_FORWARDED_HEADERS` to `false`.

3. Wire the limiter into the OTP request handler
   - Update `src/routes/store.rs::request_store_auth_otp`
   - Run limiter check at the beginning of the handler (after parsing `hub_id` is OK, before calling `request_store_otp`)
   - If exceeded, return `429` with `Retry-After` and JSON error body.

4. Register module
   - Ensure `src/routes/mod.rs` exports the new module if needed by `store.rs`.

5. Add tests
   - Unit tests in `src/routes/rate_limit.rs` (or `src/routes/rate_limit_test.rs`) to cover:
     - Allows up to `MAX_REQUESTS` within `WINDOW_SECONDS`
     - Rejects the next request and returns a sensible `Retry-After`
     - Accepts again after the window elapses
   - (Optional) Add an Actix handler test verifying 429 behavior if the repo already has route-level tests.

6. Formatting and verification
   - Run `cargo fmt --all`
   - Run `cargo test --all-features --verbose`
   - If clippy is part of the normal workflow, run `cargo clippy --all-features --tests -- -Dwarnings`

## Notes / Decisions

- ADR is not planned: the change stays within the route layer and does not introduce new infrastructure dependencies.
- Limiter is per-process (per instance). This is acceptable per the feature spec.
