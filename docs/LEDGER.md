# rusty_rtos_backoff — the ledger

Every number this package claims, with the run that produced it. A row
without a method is not a number. Counters before clocks; an external oracle
before a self-metric; the method line names the machine, the pinning, the arm
order and the null-arm floor for anything timed.

## Conformance (2026-09-16)

A count is a number and belongs here, with its method. This is the number the
README quotes, and it was missing from this file until the JSON package's
ledger was written and the omission showed up by comparison.

| quantity | value | method |
|---|---|---|
| calls agreeing with `backoff_algorithm.c` | **192 / 192** | `cargo test -p rusty_rtos_backoff-core`. 8 contexts x 24 calls. Our arm: `Backoff::next_backoff`. C arm: `oracle/backoff_driver.c` driving `backoff_algorithm.c` compiled verbatim from the pinned checkout (v1.4.2 at `14f4c88`), captured once into `oracle/backoff.trace`. Both arms are driven by the same LCG, written out identically, because `rand()` is implementation-defined and would make them incomparable by construction. |
| observables compared per call | **5** | status, delay, next jitter maximum, attempts done -- and the geometry line, which fails the run if the two arms are not executing the same table. |
| branches the workload must reach | **4** | jitter doubling, jitter clamping, attempts exhausted, `RETRY_FOREVER` never exhausting. A standing test fails if any is not reached. |
| poison rows | **2** | the inclusive jitter range (dropping the `+ 1`) and the integer-division doubling test (making the comparison exact); each fails the differential and a dedicated test. |

No speed number: the whole algorithm is one modulo, one compare and one add.

## The build fact (2026-09-09)

| gate | result |
|---|---|
| `cargo check --workspace` on the host | passes at scaffold |
| `cargo check -p rusty_rtos_backoff-core --no-default-features` and `--features alloc` on `thumbv7em-none-eabihf`, `thumbv8m.main-none-eabihf`, `riscv32imac-unknown-none-elf`, `riscv32imafc-unknown-none-elf` | passes at scaffold (`kairos check`) |
| `cargo clippy --workspace --all-targets -- -D warnings` under the workspace lint policy | clean at scaffold |
| `cargo deny check` | see the hardening plan's H-08 row |

No speed number, no size number: nothing here has been measured. Nothing has
run on a chip.
