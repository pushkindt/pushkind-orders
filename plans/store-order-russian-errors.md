# Plan: Store Service Russian Errors

1. Inspect `src/services/store.rs` and its tests to enumerate all user-facing
   form errors still in English.
2. Replace the remaining English literals with Russian messages while preserving
   behavior.
3. Update tests to assert the localized OTP and storefront-order messages.
4. Run focused Rust tests covering store service flows.
