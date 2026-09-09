#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
//! `rusty_rtos_backoff` — backoffAlgorithm remade in Rust: exponential backoff with jitter for network retries, no_std, forbid(unsafe), gated against the C library's own test vectors.
//!
//! This is the facade: it re-exports the `no_std` core. Depend on this crate;
//! reach into the sub-crates only when you are building a port or a backend.
//!
//! Part of Kairos (Remade With Rust). Plan: `docs/plans/rusty_rtos_backoff.md`.

pub use rusty_rtos_backoff_core::*;

/// The names a firmware wants in scope.
pub mod prelude {
    pub use rusty_rtos_backoff_core::prelude::*;
}
