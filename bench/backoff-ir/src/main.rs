//! Instruction counts for the retry backoff algorithm.
//!
//! A cross product of contexts against random values, each context driven all
//! the way to exhaustion. That reaches every arm the algorithm has: the jitter
//! draw, the doubling of the jitter maximum, the clamp onto the ceiling, and
//! the refusal once the attempts are spent.
//!
//! The random values are chosen, not generated. A generator inside the loop
//! would be measured along with the thing under test, and the C takes the
//! value from its caller for the same reason this crate does.
//!
//! A deterministic counter, not a clock. The verdict counts are the work
//! parity anchors: a change that moves any of them changed behaviour, and a
//! compiler that removed the work moves the checksum.

use rusty_rtos_backoff_core::{Backoff, RETRY_FOREVER};

/// Enough repetitions that process startup is noise in the total.
const REPS: u32 = 2000;

/// `(base, max, attempts)`, spanning the shapes the C's own tests use.
///
/// The last two are the edges: a ceiling equal to the base never doubles, and
/// `RETRY_FOREVER` never exhausts, so the loop below stops it by count.
const CONTEXTS: &[(u16, u16, u32)] = &[
    (500, 5_000, 5),
    (1_000, 10_000, 3),
    (100, 100, 4),
    (1, u16::MAX, 8),
    (u16::MAX, u16::MAX, 2),
    (250, 1_000, RETRY_FOREVER),
    (0, 0, 3),
    (4_000, 32_000, 6),
];

/// Random values the caller supplies, including the two that exercise the
/// modulo's edges.
const RANDOM: &[u32] = &[
    0,
    1,
    0x7FFF_FFFF,
    u32::MAX,
    12_345,
    0xDEAD_BEEF,
    65_535,
    65_536,
];

/// A context driven forever would not stop; this is the leash.
const MAX_STEPS: u32 = 12;

fn main() {
    let mut delays = 0u64;
    let mut exhausted = 0u64;
    let mut checksum = 0u64;

    for _ in 0..REPS {
        for (c, (base, max, attempts)) in CONTEXTS.iter().enumerate() {
            for (r, random) in RANDOM.iter().enumerate() {
                let mut backoff = Backoff::new(*base, *max, *attempts);
                let mut step = 0u32;
                loop {
                    match backoff.next_backoff(random.wrapping_add(step)) {
                        Ok(delay) => {
                            delays = delays.wrapping_add(1);
                            // The delay folds in, so a change that returned a
                            // different one shows as a moved checksum and not
                            // merely as the same verdict count.
                            checksum = checksum
                                .wrapping_add(u64::from(delay))
                                .wrapping_add((c.wrapping_mul(RANDOM.len())).wrapping_add(r) as u64);
                        }
                        Err(_) => {
                            exhausted = exhausted.wrapping_add(1);
                            break;
                        }
                    }
                    step = step.wrapping_add(1);
                    if step >= MAX_STEPS {
                        break;
                    }
                }
            }
        }
    }

    let pairs = (CONTEXTS.len() * RANDOM.len()) as u64;
    println!("checksum {checksum}");
    println!(
        "pairs {pairs} reps {REPS} delays {} exhausted {}",
        delays / u64::from(REPS),
        exhausted / u64::from(REPS)
    );
}
