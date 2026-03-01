# Store Service Russian Errors

## Summary

Storefront service endpoints must return Russian-language form errors from
`src/services/store.rs`.

## Requirements

- Translate all user-facing `ServiceError::Form` messages emitted by
  `src/services/store.rs` to Russian.
- Preserve existing validation behavior and HTTP status codes.
- Keep the change scoped to `src/services/store.rs`; do not alter unrelated
  form modules unless the service already emits their messages directly.

## Acceptance Criteria

- OTP throttle failures return a Russian error message.
- Invalid or expired OTP submissions return a Russian error message.
- Unknown or invalid products return a Russian error message.
- Missing pricing returns a Russian error message.
- Non-positive quantities return a Russian error message.
- Mixed vendors and mixed currencies return Russian error messages.
- Arithmetic overflow during total calculation returns a Russian error message.
- Empty or invalid storefront order line payloads surfaced through
  `create_store_order` return Russian error messages.
