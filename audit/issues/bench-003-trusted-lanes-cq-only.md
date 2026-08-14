# bench-003: the stamp, the fuzz, the seeded corpus, and the contradiction knob consume only `random_query`

- **Severity:** high
- **Tree:** bench
- **Status:** OPEN
- **Source:** audit/bench.md F3
- **Depends on:** engine-020 (one generator entry — these callers update mechanically once `random_query` draws the class)

## The bug

engine-020's side entry is not a comment. Every trusted consumer that "draws a random Query" calls `querygen::random_query` and never `random_reach_query`:

- `verify/run.rs:145` — `random_lane`, the stamp that gates timing
- `corpus_gen/opgen.rs:90` — the lifecycle fuzz query pool
- `conformance.rs:1527,1663` — the 246 seeded cases
- `querygen/contradict.rs:21` — the contradiction-fold differential
- also `differential/tests/{closed,fold}.rs`, `corpus_gen/rng.rs:130`

Rec coverage lives in a parallel world: closure's inline gate, `differential/tests/recursive.rs`, the reach conformance arm.

## Why it's wrong

Insight 2: a side entry means reach queries only appear where a caller remembered the second function. The stamp is the law that nothing is timed without oracle agreement; its randomized half never sees interiors/rec. A shared misreading of the new IR can pass verify forever — the dual-oracle blind spot the Lean lane exists to close, reopened by the grammar's product (Insight 1).

## The fix

Per engine-020: one `random_query` draws `QueryClass`. These callers do not grow a second loop; they keep calling the one entry. Until engine-020 lands, do not paper over the hole with a second `random_reach_query` pass in verify (that would cement the sidecar). Coverage must not regress: once the one entry exists, the stamp's randomized half must include interiors/rec draws (weights recorded, assertions not lowered).

C1: do not regenerate the 246 seeded files as a side effect of this caller change. The conformance *builder* drawing through the one entry is allowed to start producing reach-shaped *in-memory* Queries; writing them over checked-in `seeded-*.json` is forbidden. Reach cases stay `reach-*.json` (existing arm).

## Acceptance criteria

- [ ] After engine-020: `rg -n 'random_query' crates/bumbledb-bench/src/verify/run.rs crates/bumbledb-bench/src/corpus_gen/opgen.rs crates/bumbledb-bench/src/conformance.rs crates/bumbledb-bench/src/querygen/contradict.rs` still calls the one entry; `rg -n 'random_reach_query' crates/bumbledb-bench/src --glob '!querygen/*'` → no external callers (engine-020's criterion, confirmed here).
- [ ] A verify randomized batch of the default case count includes at least one interiors-or-rec query (pin with a test or a counted report line — do not lower any existing coverage assertion).
- [ ] Unchanged: 268 checked-in cases byte-identical; stamp semantics otherwise identical.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench`; `./scripts/check.sh`; `./scripts/lean.sh`.

## Constraints

- Blocked on engine-020's one entry. Do not add a parallel reach loop in verify. Corpus frozen (C1). Assertions never weakened.
