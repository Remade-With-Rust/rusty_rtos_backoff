#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
//! `rusty_rtos_backoff-core` — `backoffAlgorithm` remade in Rust.
//!
//! Exponential backoff with jitter, as FreeRTOS's `backoffAlgorithm` v1.4.2
//! computes it: 103 lines of C whose whole job is to answer "how long should I
//! wait before the next attempt, and may I have another".
//!
//! `no_std`, no allocator, no clock and no randomness of its own — the caller
//! supplies the random value, exactly as the C does, which is what makes the
//! two comparable at all.
//!
//! # Four things here are the C's, not a design
//!
//! Each is a place a reimplementation would drift, and the differential is
//! what would catch it:
//!
//! 1. **The jitter range is INCLUSIVE.** `randomValue % ( nextJitterMax + 1 )`
//!    — the `+ 1` means `nextJitterMax` itself is a reachable delay. Writing
//!    `% nextJitterMax` loses exactly one value at the top of every range.
//! 2. **The doubling test uses INTEGER division.** `nextJitterMax <
//!    maxBackoffDelay / 2` truncates, so with an odd `maxBackoffDelay` the
//!    clamp arrives one step earlier than "half" would suggest.
//! 3. **The clamp is to `maxBackoffDelay`, not to double it.** When the test
//!    fails the next jitter maximum becomes the ceiling exactly, so every
//!    later attempt draws from the same full range.
//! 4. **`attemptsDone` moves only on success.** An exhausted call leaves the
//!    context untouched, so asking again after exhaustion is idempotent
//!    rather than climbing.
//!
//! # What this crate does NOT do
//!
//! It does not sleep, and it does not generate the random value. The C does
//! neither either: `BackoffAlgorithm_GetNextBackoff` takes `randomValue` as an
//! argument because the entropy source is the application's business. Keeping
//! that boundary is what lets the differential drive both arms from one
//! sequence.

use rusty_rtos_core::error::{Error, Result};

/// `BACKOFF_ALGORITHM_RETRY_FOREVER`: retry without limit.
pub const RETRY_FOREVER: u32 = u32::MAX;

/// `BackoffAlgorithmContext_t`.
///
/// Every field is the C's, at the C's width. `nextJitterMax` and
/// `maxBackoffDelay` are 16 bits and `attemptsDone` and `maxRetryAttempts` are
/// 32, which is not tidy and is not ours to tidy: the widths decide where the
/// arithmetic saturates and a wider field would diverge at the edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    /// `nextJitterMax`: the top of the range the next delay is drawn from.
    next_jitter_max: u16,
    /// `maxBackoffDelay`: the ceiling the jitter maximum climbs to.
    max_backoff_delay: u16,
    /// `attemptsDone`.
    attempts_done: u32,
    /// `maxRetryAttempts`, or [`RETRY_FOREVER`].
    max_retry_attempts: u32,
}

impl Backoff {
    /// `BackoffAlgorithm_InitializeParams`.
    ///
    /// `base` is the first jitter maximum, `max` the ceiling it climbs to, and
    /// `attempts` how many are allowed before exhaustion — or
    /// [`RETRY_FOREVER`].
    #[must_use]
    pub const fn new(base: u16, max: u16, attempts: u32) -> Self {
        Self {
            next_jitter_max: base,
            max_backoff_delay: max,
            attempts_done: 0,
            max_retry_attempts: attempts,
        }
    }

    /// `BackoffAlgorithm_GetNextBackoff`.
    ///
    /// `random` is the caller's random value, as it is in the C — this crate
    /// has no entropy source and should not have one.
    ///
    /// # Errors
    ///
    /// [`Error::Timeout`] is `BackoffAlgorithmRetriesExhausted`: the attempts
    /// are used up. The context is left UNTOUCHED, so asking again answers the
    /// same way rather than climbing — which matters for a caller that retries
    /// its own retry loop.
    pub fn next_backoff(&mut self, random: u32) -> Result<u16> {
        // `maxRetryAttempts == RETRY_FOREVER || attemptsDone < maxRetryAttempts`
        if self.max_retry_attempts != RETRY_FOREVER && self.attempts_done >= self.max_retry_attempts
        {
            return Err(Error::Timeout);
        }

        // `randomValue % ( nextJitterMax + 1U )`, and the `+ 1` is INCLUSIVE:
        // `next_jitter_max` itself is a reachable delay. The C promotes to
        // 32 bits for the addition, so 65,535 + 1 does not wrap; `u32::from`
        // is that promotion written out.
        let span = u32::from(self.next_jitter_max).saturating_add(1);
        let delay = random.checked_rem(span).unwrap_or(0);

        self.attempts_done = self.attempts_done.saturating_add(1);

        // `nextJitterMax < maxBackoffDelay / 2U` — INTEGER division, so an odd
        // ceiling clamps one step earlier than half would suggest.
        if self.next_jitter_max < self.max_backoff_delay / 2 {
            self.next_jitter_max = self.next_jitter_max.saturating_mul(2);
        } else {
            // To the ceiling exactly, not to double it.
            self.next_jitter_max = self.max_backoff_delay;
        }

        Ok(u16::try_from(delay).unwrap_or(u16::MAX))
    }

    /// `attemptsDone`.
    #[must_use]
    pub const fn attempts_done(&self) -> u32 {
        self.attempts_done
    }

    /// `nextJitterMax`: the top of the range the next delay will be drawn from.
    #[must_use]
    pub const fn next_jitter_max(&self) -> u16 {
        self.next_jitter_max
    }

    /// `maxBackoffDelay`.
    #[must_use]
    pub const fn max_backoff_delay(&self) -> u16 {
        self.max_backoff_delay
    }

    /// `maxRetryAttempts`.
    #[must_use]
    pub const fn max_retry_attempts(&self) -> u32 {
        self.max_retry_attempts
    }

    /// Whether the next call would be refused.
    #[must_use]
    pub const fn exhausted(&self) -> bool {
        self.max_retry_attempts != RETRY_FOREVER && self.attempts_done >= self.max_retry_attempts
    }
}

/// The names a firmware wants in scope.
pub mod prelude {
    pub use super::{Backoff, RETRY_FOREVER};
    pub use rusty_rtos_core::prelude::*;
}

/// Crate version, for manifests and logs.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
