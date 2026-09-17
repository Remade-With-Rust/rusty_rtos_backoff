//! `Backoff` against `backoff_algorithm.c`, call for call.
//!
//! The C arm is `oracle/backoff_driver.c` driving `backoff_algorithm.c`
//! compiled VERBATIM from the pinned checkout (v1.4.2 at `14f4c88`), and its
//! trace is checked in — so this runs in CI with no C toolchain, the same
//! arrangement the kernel corpus and the three heap differentials use.
//!
//! **This is K7's first differential, and proving the harness generalises is
//! half its job.** Nothing in this family had been diffed against a C
//! *library* rather than the kernel before it; `backoffAlgorithm` is 103 lines
//! with no I/O, no clock and no transport, which is why it went first — the
//! same reason `heap_1` was built before `heap_5`.
//!
//! # The branch guard
//!
//! A backoff context has four interesting branches: the jitter maximum
//! doubling, the jitter maximum clamping, attempts exhausted, and
//! `RETRY_FOREVER` never exhausting. A run that misses any of them proves
//! correspondingly less, so [`the_workload_reaches_every_branch`] fails when
//! one is not seen — the third shape of that guard in this repository, after
//! heap_4's (too few refusals) and heap_1's (never exhausted).

// A test asserts; the workspace's deny-by-default is written for library code
// where a panic is a defect.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use rusty_rtos_backoff_core::{Backoff, RETRY_FOREVER};
use rusty_rtos_core::error::Error;

/// The C driver's table, in the same order. Each row is a branch this
/// differential exists to reach.
const CASES: [(u16, u16, u32); 8] = [
    (500, 10_000, 5),           // the coreMQTT defaults: doubles, then clamps
    (1, 1, 8),                  // base == ceiling: clamps on the first call
    (1, 65_535, 20),            // the full u16 range, doubling all the way up
    (100, 999, 10),             // an ODD ceiling: integer division clamps early
    (0, 1000, 6),               // a zero base: the range is a single value
    (1000, 100, 6),             // ceiling BELOW base: clamps down immediately
    (250, 4000, 0),             // zero attempts: exhausted before the first call
    (250, 4000, RETRY_FOREVER), // retry forever
];

const CALLS: u32 = 24;

/// The C driver's LCG, written out identically. `rand()` is
/// implementation-defined and would make the two arms incomparable by
/// construction.
struct Lcg(u32);

impl Lcg {
    const fn new() -> Self {
        Self(12345)
    }

    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}

#[test]
fn our_backoff_matches_the_c_librarys_call_for_call() {
    let oracle = include_str!("../../../oracle/backoff.trace");
    let mut lines = oracle.lines();

    // The geometry is a premise, not decoration: a driver built from a
    // different table would make every later comparison meaningless.
    assert_eq!(
        lines.next().expect("a geometry line"),
        format!(
            "geometry cases={} calls={CALLS} forever={RETRY_FOREVER}",
            CASES.len()
        ),
        "the two arms are not running the same workload"
    );

    let mut rng = Lcg::new();

    for (c, (base, max, attempts)) in CASES.iter().copied().enumerate() {
        assert_eq!(
            lines.next().expect("a case line"),
            format!("case {c} {base} {max} {attempts}"),
            "case {c} differs between the arms"
        );

        let mut backoff = Backoff::new(base, max, attempts);

        for n in 0..CALLS {
            let random = rng.next();
            let (status, delay) = match backoff.next_backoff(random) {
                Ok(delay) => ("ok", delay),
                Err(Error::Timeout) => ("exhausted", 0),
                Err(other) => panic!("case {c} call {n}: unexpected {other:?}"),
            };

            // Every observable the context has: the status, the delay, and
            // the two fields carrying state forward. A transcription that got
            // the delay right and the jitter climb wrong would pass on the
            // first and fail on the second.
            let ours = format!(
                "next {n} {random} {status} {delay} {} {}",
                backoff.next_jitter_max(),
                backoff.attempts_done()
            );
            let theirs = lines
                .next()
                .unwrap_or_else(|| panic!("the C trace ran out at case {c} call {n}"));
            assert_eq!(
                ours, theirs,
                "case {c} call {n} diverged — ours {ours:?}, the C {theirs:?}"
            );
        }
    }

    assert_eq!(lines.next().expect("an end line"), "end");
    assert!(lines.next().is_none(), "the C trace has lines left over");
}

/// The guard: every branch a backoff context has must actually be reached.
///
/// heap_4's guard fails when the workload refuses too few requests; heap_1's
/// fails when the arena is never exhausted; this one fails when a branch is
/// never taken. All three say the same thing — a differential whose workload
/// cannot fail is a differential about nothing.
#[test]
fn the_workload_reaches_every_branch() {
    let mut rng = Lcg::new();
    let (mut doubled, mut clamped, mut exhausted, mut forever_calls) = (0u32, 0u32, 0u32, 0u32);

    for (base, max, attempts) in CASES {
        let mut backoff = Backoff::new(base, max, attempts);
        for _ in 0..CALLS {
            let before = backoff.next_jitter_max();
            match backoff.next_backoff(rng.next()) {
                Ok(_) => {
                    let after = backoff.next_jitter_max();
                    if after == before.saturating_mul(2) && after != max {
                        doubled = doubled.saturating_add(1);
                    }
                    if after == max {
                        clamped = clamped.saturating_add(1);
                    }
                    if attempts == RETRY_FOREVER {
                        forever_calls = forever_calls.saturating_add(1);
                    }
                }
                Err(_) => exhausted = exhausted.saturating_add(1),
            }
        }
    }

    assert!(doubled > 10, "the jitter maximum barely doubled: {doubled}");
    assert!(clamped > 10, "the jitter maximum barely clamped: {clamped}");
    assert!(
        exhausted > 10,
        "attempts were barely exhausted: {exhausted}"
    );
    assert_eq!(
        forever_calls, CALLS,
        "RETRY_FOREVER must never exhaust, so all {CALLS} calls must succeed"
    );
}

/// The jitter range is INCLUSIVE, and that is one character in the C.
///
/// `randomValue % ( nextJitterMax + 1 )` — so `nextJitterMax` itself is a
/// reachable delay. Writing `% nextJitterMax` loses exactly one value at the
/// top of every range, which a differential would find only when the LCG
/// happened to land there.
#[test]
fn the_jitter_range_includes_its_maximum() {
    // A ceiling equal to the base, so the range never moves: every draw is
    // `random % (base + 1)`, and `base` itself must be reachable.
    let base = 7u16;
    let mut seen_top = false;
    for random in 0..64u32 {
        let mut backoff = Backoff::new(base, base, RETRY_FOREVER);
        let delay = backoff.next_backoff(random).expect("forever never refuses");
        assert!(delay <= base, "a delay outside the range: {delay} > {base}");
        if delay == base {
            seen_top = true;
        }
    }
    assert!(
        seen_top,
        "the maximum was never drawn — the range is exclusive"
    );
}

/// An exhausted context is left UNTOUCHED, so asking again is idempotent.
///
/// The C increments `attemptsDone` inside the success arm only. A
/// transcription that incremented before the check would climb for ever on a
/// caller that retries its own retry loop.
#[test]
fn exhaustion_does_not_move_the_context() {
    let mut backoff = Backoff::new(100, 1000, 2);
    assert!(backoff.next_backoff(1).is_ok());
    assert!(backoff.next_backoff(2).is_ok());

    let before = backoff;
    for random in 0..16 {
        assert_eq!(backoff.next_backoff(random), Err(Error::Timeout));
    }
    assert_eq!(backoff, before, "a refused call changed the context");
    assert!(backoff.exhausted());
}

/// `RETRY_FOREVER` never exhausts, however many attempts are made.
#[test]
fn retry_forever_never_exhausts() {
    let mut backoff = Backoff::new(1, 64, RETRY_FOREVER);
    for random in 0..10_000u32 {
        assert!(
            backoff.next_backoff(random).is_ok(),
            "RETRY_FOREVER refused at attempt {random}"
        );
    }
    assert!(!backoff.exhausted());
    assert_eq!(backoff.attempts_done(), 10_000);
}

/// The doubling test uses INTEGER division, so an odd ceiling clamps early.
///
/// `nextJitterMax < maxBackoffDelay / 2` truncates. With a ceiling of 999 the
/// test is `< 499`, not `< 499.5` — so a jitter maximum of 499 does NOT
/// double, it clamps. One character of C, and a whole step of behaviour.
#[test]
fn an_odd_ceiling_clamps_a_step_early() {
    let mut backoff = Backoff::new(499, 999, RETRY_FOREVER);
    backoff.next_backoff(0).expect("first call");
    assert_eq!(
        backoff.next_jitter_max(),
        999,
        "499 < 999/2 == 499 is false, so it must clamp rather than double"
    );

    // One less, and it doubles instead.
    let mut backoff = Backoff::new(498, 999, RETRY_FOREVER);
    backoff.next_backoff(0).expect("first call");
    assert_eq!(backoff.next_jitter_max(), 996, "498 < 499, so it doubles");
}
