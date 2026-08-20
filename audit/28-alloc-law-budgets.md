# 28 — The allocation law, pinned as budget tests

- **Status:** **fixed this pass** — ordinary-tier `tests/alloc_budgets.rs`
  (`alloc_law_budgets`): `allocs_per_warm_query == 0` on store+heap
  scan/join/key_probe; `allocs_per_committed_fact <= 3` (`K` derived in
  the test); `allocs_per_point_read == 0` on both arms; admit peak via
  `AllocAbsolute::peak_live_bytes`. Harness: `CountingAllocator` /
  `snapshot`/`reset` always compile; lib `#[global_allocator]` stays
  `alloc-counter`. Gate: `cargo test -p bumbledb --test alloc_budgets`.
- **Severity:** performance law — the mechanism the rest of W3/W4 hangs on.

## The law

Zero allocations on steady-state hot paths; allocation belongs at
construction and at FFI crossings (the recorded one-copy law) only.
Steady-state hot paths, named: warm `execute` per emitted row; judgment per
fact beyond arena growth; point reads (`get`/`contains`); COLT
refill/advance; bind of an already-latched plan.

## The mechanism (exists, now honest)

`alloc_counter` gained the `AllocWindow` / `AllocAbsolute` split with a real
`peak_live_bytes` this pass. Budgets therefore become **tests**, not
reviews:

- `allocs_per_warm_query == 0` — window around the second execution of a
  prepared query on each scenario shape (first execution may build images).
- `allocs_per_committed_fact <= K` — window around a steady-batch commit,
  `K` a named constant with a comment deriving it from the arena design.
- `allocs_per_point_read == 0` — warm `get`/`contains` on store and heap
  arms both.
- Admission phase peaks stay gated by the proposal's peak equation
  (`A+I+R` / `A+R+F+J`) — already telemetried; pin the equation check as a
  test if it is not already.

## Acceptance

- The budget tests exist per lane and are green; each failure message names
  the window and the count.
- The budgets are wired into the same suite tier as the census gates (they
  run on every `cargo test --workspace`).
- 29 consumes these tests as its verdict oracle.

## Adjudication

`A+I+R` / `A+R+F+J` are not a named counter surface on `HeapStage` /
`admit_catalog` (those files are not this lane). The pin uses
`AllocAbsolute::peak_live_bytes` around `InstanceBuilder::admit` against
a four-chunk + `F` stand-in — the honest instrument this lane owns. The
equation itself stays a catalog-owned later pin if those quantities are
exported.
