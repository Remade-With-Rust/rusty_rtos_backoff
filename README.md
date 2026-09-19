# rusty_rtos_backoff

[![Remade With Rust](https://img.shields.io/badge/Remade%20With-Rust-000?logo=rust&logoColor=fff)](https://github.com/remade-with-rust)
[![By Mata Network](https://img.shields.io/badge/by-Mata%20Network-5b2be0)](https://www.mata.network)
[![crates.io](https://img.shields.io/crates/v/rusty_rtos_backoff.svg)](https://crates.io/crates/rusty_rtos_backoff)
[![docs.rs](https://docs.rs/rusty_rtos_backoff/badge.svg)](https://docs.rs/rusty_rtos_backoff)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Exponential backoff with jitter, the Kairos remake of backoffAlgorithm. MIT OR Apache-2.0.

**K7's first library**, and the first thing in this family diffed against a C
*library* rather than the kernel.

- **Proven**: exponential backoff with jitter, diffed call-for-call against
  `backoff_algorithm.c` over 192 calls across 8 contexts chosen to reach every
  branch it has.
- **No clock and no randomness of its own.** The caller supplies the random
  value, exactly as `BackoffAlgorithm_GetNextBackoff` does — the entropy source
  is the application's business, and keeping that boundary is what lets both
  arms be driven from one sequence.

**Known gaps.** None in the algorithm. It does not sleep and it does not
generate randomness, because the C does neither.

- This package's plan: [docs/plans/rusty_rtos_backoff.md](https://github.com/Remade-With-Rust/rusty_rtos_backoff/blob/main/docs/plans/rusty_rtos_backoff.md)
- Every number: [docs/LEDGER.md](https://github.com/Remade-With-Rust/rusty_rtos_backoff/blob/main/docs/LEDGER.md)
- The family plan: Kairos [`docs/plans/rtos-mission.md`](https://github.com/Remade-With-Rust/kairos/blob/main/docs/plans/rtos-mission.md)

**Claims discipline:** this README makes no performance or capability claim that
is not backed by a test, a benchmark ledger entry, or a kill test recorded in
the plan. "Scaffold" means scaffold. "Sim only" means the sim port; "builds, not
flashed" means no chip has run it.

## Conformance

**192 calls across 8 contexts agree with `backoff_algorithm.c`**, compiled
verbatim from the pinned checkout (v1.4.2 at `14f4c88`) — on the status, the
delay, the next jitter maximum and the attempt count, after every call.

```sh
cargo test -p rusty_rtos_backoff-core
```

The C arm's trace is checked in, so this diffs with no C toolchain. Fetch the
oracle itself with `kairos oracle fetch --lib backoffAlgorithm`.

**The branch guard.** A backoff context has four interesting branches — the
jitter maximum doubling, the jitter maximum clamping, attempts exhausted, and
`RETRY_FOREVER` never exhausting — and a run that misses one proves
correspondingly less. A standing test fails when any is not reached. That is
the third shape of that guard here, after `heap_4`'s (too few refusals) and
`heap_1`'s (never exhausted), and all three say the same thing: a differential
whose workload cannot fail is a differential about nothing.

**Poison-proven twice, on the two places a reimplementation drifts:**

* the jitter range is **inclusive** — `randomValue % ( nextJitterMax + 1 )`, so
  the maximum itself is reachable. Dropping the `+ 1` fails the differential
  and a dedicated test;
* the doubling test uses **integer division** — `nextJitterMax <
  maxBackoffDelay / 2` truncates, so with an odd ceiling of 999 a jitter
  maximum of 499 **clamps** rather than doubling. Making that comparison exact
  fails the odd-ceiling test.

Two more of the C's decisions are pinned by tests: the clamp goes to the
ceiling exactly rather than to double it, and `attemptsDone` moves **only on
success**, so an exhausted context is left untouched and asking again is
idempotent.

## Using it

```rust
use rusty_rtos_backoff::{Backoff, RETRY_FOREVER};

// base, ceiling, attempts -- `BackoffAlgorithm_InitializeParams`.
let mut backoff = Backoff::new(500, 10_000, 5);

// The caller supplies the random value, as the C does.
match backoff.next_backoff(some_random_u32) {
    Ok(delay_ms) => { /* wait `delay_ms`, then try again */ }
    Err(_) => { /* BackoffAlgorithmRetriesExhausted */ }
}
```

With `Backoff::new(base, ceiling, RETRY_FOREVER)` it never exhausts.

## Performance

No rows, and none are wanted: the whole algorithm is one modulo, one compare
and one add. What matters about it is that it agrees with the C, which the
differential says.

## Portability

Builds `no_std` on host, `thumbv7m-none-eabi`,
`riscv32imac-unknown-none-elf` and `xtensa-esp32s3-none-elf`. A build claim, not
a behaviour claim.

## Layout

```text
crates/rusty_rtos_backoff          facade: re-exports + prelude; the crate you depend on
crates/rusty_rtos_backoff-core     no_std (+ alloc); forbid(unsafe); types, traits, algorithms
firmware/                per-chip example projects, excluded from the workspace
docs/plans/              this package's plan and its hardening audit
docs/LEDGER.md           every number, with its method line
```

## Build

```sh
cargo test --workspace                                   # host: the tests
cargo check -p rusty_rtos_backoff-core --no-default-features \
  --target thumbv7em-none-eabihf                         # Cortex-M4F class, no alloc
cargo check -p rusty_rtos_backoff-core --no-default-features --features alloc \
  --target riscv32imac-unknown-none-elf                  # ESP32-C6 class, with alloc
```

CI holds the core to `thumbv7em-none-eabihf`, `thumbv8m.main-none-eabihf`,
`riscv32imac-unknown-none-elf` and `riscv32imafc-unknown-none-elf`, with and
without `alloc`, plus `cargo deny check`. Firmware examples (Xtensa needs the
esp toolchain; Cortex-M and RISC-V work on stable) are built from their own
directories under `firmware/`.

## Part of Remade With Rust

This crate is part of **[Kairos](https://github.com/Remade-With-Rust/kairos)** —
FreeRTOS remade in memory-safe Rust, as independent packages that expose the API
a FreeRTOS developer already knows and prove every scheduling decision against
the C kernel's own trace. `rusty_rtos_backoff` is one of the K7 libraries, and is not started.

The family:
[`rusty_rtos_core`](https://crates.io/crates/rusty_rtos_core) (the shared vocabulary),
[`rusty_rtos_kernel`](https://crates.io/crates/rusty_rtos_kernel) (the scheduler),
[`rusty_rtos_port`](https://crates.io/crates/rusty_rtos_port) (the architecture seam),
[`rusty_rtos_heap`](https://crates.io/crates/rusty_rtos_heap) (the allocators),
[`rusty_rtos_json`](https://github.com/Remade-With-Rust/rusty_rtos_json) (coreJSON),
[`rusty_rtos_sntp`](https://github.com/Remade-With-Rust/rusty_rtos_sntp) (coreSNTP),
[`rusty_rtos_mqtt`](https://github.com/Remade-With-Rust/rusty_rtos_mqtt) (coreMQTT),
[`rusty_rtos_backoff`](https://github.com/Remade-With-Rust/rusty_rtos_backoff) (backoffAlgorithm),
[`rusty_rtos-capi`](https://github.com/Remade-With-Rust/rusty_rtos-capi) (the C ABI) and
[`rusty_rtos_demo`](https://github.com/Remade-With-Rust/rusty_rtos_demo) (the conformance corpus).
The last six are on GitHub and not yet on crates.io. Also check out
the rest of **[github.com/remade-with-rust](https://github.com/remade-with-rust)**.

## About Mata Network

<!-- ORG BOILERPLATE — keep identical across repos -->

[Mata Network](https://www.mata.network) builds sovereign, self-hostable
infrastructure. **Remade With Rust** is our open-source home for the
permissively-licensed building blocks that work depends on.

<!-- /ORG BOILERPLATE -->

## License

MIT OR Apache-2.0, at your option. FreeRTOS is MIT-licensed by Amazon.com,
Inc. or its affiliates; this crate remakes its API and behaviour from the
published sources and links no FreeRTOS code.

---

<!-- HARDENING-TABLE:BEGIN generated by use-protection-please — edit docs/plans/use-protection-please.md, not this block -->
## Hardening status

**Tier** critical-path · **Audited** 2026-09-16 (v0.1.0 release pass) · **v1.0.0 gates** 7/17 · [Full checklist](https://github.com/Remade-With-Rust/rusty_rtos_backoff/blob/main/docs/plans/use-protection-please.md)

`█████████░░░░░░░░░░░` **46%** &nbsp;·&nbsp; 11 Completed · 0 Scheduled · 13 Incomplete · 31 N/A

| Phase | ✅ Completed | 🗓 Scheduled | ⬜ Incomplete | · N/A |
|---|--:|--:|--:|--:|
| 0 — Threat modeling | 0 | 0 | 1 | 1 |
| 1 — Toolchain | 2 | 0 | 2 | 0 |
| 2 — Supply chain | 5 | 0 | 2 | 1 |
| 3 — Code level | 3 | 0 | 1 | 3 |
| 4 — Static analysis | 0 | 0 | 0 | 1 |
| 5 — Dynamic analysis | 0 | 0 | 1 | 2 |
| 6 — Fuzzing and properties | 0 | 0 | 1 | 3 |
| 7 — Formal verification | 0 | 0 | 0 | 1 |
| 8 — Build and binary | 0 | 0 | 1 | 1 |
| 9 — Runtime privilege | 0 | 0 | 0 | 1 |
| 10 — Cryptography | 0 | 0 | 0 | 3 |
| 11 — CI/CD, release, and operations | 1 | 0 | 4 | 0 |
| 12 — Compliance controls | 0 | 0 | 0 | 14 |
| **Total** | **11** | **0** | **13** | **31** |

Gates waived for 0.x are listed with their reasons in the plan's "v0.1.0 release decision" section — an Incomplete gate not listed there is an omission, not a decision.

**Architect** — [Tim Almond](https://github.com/Ttimmahlax) — accountable for this unit's security design; rendered
<!-- HARDENING-TABLE:END -->
