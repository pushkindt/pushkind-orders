# Feature: IP-Based Rate Limiting for Store OTP Requests

Date: 2026-02-27
Status: Stable

## Summary

Add an IP-based rate limiter to the Store API endpoint that requests an OTP:

- `POST /api/v1/store/{hub_id}/auth/otp`

This complements the existing per-phone throttling in the service layer by reducing SMS-cost abuse and noisy bot traffic.

## Motivation

The OTP request endpoint has real cost (SMS delivery). Today it is throttled per `(hub_id, phone)` in the service (`OTP_THROTTLE_MINUTES`), but attackers can:

- Rotate phone numbers to bypass per-phone throttling.
- Send high-volume traffic from a single IP, creating load and SMS spend.

An additional IP-based limiter provides a cheap, immediate defense and a clear operational lever (configurable limits).

## Goals

- Limit OTP request volume per client IP.
- Keep enforcement scoped to `POST /auth/otp` (do not affect other Store API routes).
- Always enable the limiter (no runtime config toggle).
- Return `429 Too Many Requests` with a usable retry hint.
- Avoid panics and avoid `unwrap` in production paths.

## Non-Goals

- Captcha or bot scoring.
- Distributed rate limiting across multiple service instances.
- Per-ASN / per-device fingerprinting.
- Protecting `POST /auth/otp/verify` (may be added later if needed).

## Proposed Design

### Enforcement point

Add a check in the Actix handler for OTP requests (route layer) before calling the service:

- `src/routes/store.rs::request_store_auth_otp`

If limited, short-circuit and return `429`.

Rationale: keeps service transport-agnostic and scopes the limiter to the one HTTP route where it matters.

### Rate limiting strategy

Use a sliding window counter per IP:

- Track timestamps of recent requests per IP in memory.
- Allow at most `N` requests within `window_seconds`.
- Compute `Retry-After` as remaining time until the oldest request exits the window.

### Client IP extraction

Use `req.peer_addr().ip()` (direct connection).

Reverse proxy deployments are supported via a compile-time constant:

- If `TRUST_FORWARDED_HEADERS` is `true`, use `req.connection_info().realip_remote_addr()` to honor `Forwarded` / `X-Forwarded-For`.
- If `TRUST_FORWARDED_HEADERS` is `false`, ignore forwarded headers.

This should default to `false` because forwarded headers are untrusted unless the app is behind a trusted proxy that strips/sets them.

### Constants

The limiter is controlled by constants (not `ServerConfig`):

- `MAX_REQUESTS` (u32)
- `WINDOW_SECONDS` (u64)
- `TRUST_FORWARDED_HEADERS` (bool)

The limiter is always enabled.

### Response behavior

On rate limit exceed:

- HTTP status: `429 Too Many Requests`
- Headers:
  - `Retry-After: <seconds>` (integer seconds, rounded up)
- JSON body:
  - `{ "error": "rate limit exceeded" }`

On missing client IP:

- Allow the request (fail-open) and rely on existing per-phone throttling.

### Observability

- Log rate-limit rejections at `info` (or `warn` if desired) including:
  - hub_id
  - client IP (when known)
  - retry-after seconds
- Do not log phone numbers.

## Implementation Notes (No Code Yet)

- Add a small helper module for rate limiting (e.g. `src/routes/rate_limit.rs`).
- Store limiter state in `web::Data<...>` and scope it to the Store API scope or the whole app.
- Wire the limiter into `request_store_auth_otp` with minimal handler changes.

## Security Considerations

- Forwarded header trust is a security boundary; default must be `false`.
- In-memory limiter is per-instance; multiple instances will each allow up to the configured rate.
- Attackers can distribute across many IPs; this is a baseline control, not a complete anti-abuse solution.

## Testing

- Unit tests for the limiter logic (windowing, limit exceeded, retry-after computation).
- Handler-level test (Actix test server) that verifies:
  - First requests succeed.
  - Exceeding limit returns `429` and sets `Retry-After`.

## Rollout Plan

- In production behind a proxy, set `TRUST_FORWARDED_HEADERS = true` only after confirming:
  - The proxy strips inbound forwarded headers from the Internet.
  - The proxy sets correct `X-Forwarded-For` / `Forwarded`.

## Open Questions

None.
