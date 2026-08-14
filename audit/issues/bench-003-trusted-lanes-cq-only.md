# bench-003: the stamp, the fuzz, and the contradiction knob consume only CQ `random_query`

- **Severity:** high
- **Tree:** bench
- **Status:** OPEN
- **Source:** audit/bench.md F3
- **Depends on:** engine-020 (randomized entry), bench-004, bench-005 (or the mixed entry panics / mis-routes)

## The bug

engine-020's side entry is not a comment. Trusted consumers that "draw a random Query" call `querygen::random_query` and never `random_reach_query`:

- `verify/run.rs:145` — `random_lane`, the stamp that gates timing
- `corpus_gen/opgen.rs:90` — the lifecycle fuzz query pool
- `querygen/contradict.rs:21` — the contradiction-fold differential
- also `differential/tests/{closed,fold}.rs`, `corpus_gen/rng.rs:130`

Rec coverage lives in a parallel world: closure's inline gate, `differential/tests/recursive.rs`, the reach conformance arm.

**Not this hole (C1):** `conformance.rs:1527,1663` also call `random_query`, but those sites are the **seeded corpus reconstructer**. Replay rebuilds each `seeded-*.json` from `Rng::new(case_seed)` through that function. Changing its RNG stream (a class coin-flip, new `SHAPE_WEIGHTS` rows) changes every seed→query map and fails byte-identity against the 246 files. Reach corpus replay (`conformance/reach.rs:485,555`) likewise depends on today's `random_reach_query` `range(8)` mapping. Those paths staying CQ-only / reach-only is the frozen corpus, not a missed `Shape` row — the 22 `reach-*.json` cases already cover derived JSON.

## Why it's wrong

Insight 2: a side entry means reach queries only appear where a caller remembered the second function. The stamp is the law that nothing is timed without oracle agreement; its randomized half never sees interiors/rec. A shared misreading of the new IR can pass verify forever — the dual-oracle blind spot the Lean lane exists to close, reopened by the grammar's product (Insight 1). Retargeting the corpus reconstructer would "fix" the hole by breaking C1.

## The fix

Per engine-020's split: the **randomized** `random_query` draws `QueryClass`. Stamp, opgen, contradict, closed/fold differentials, and the rng digest keep calling that one entry — no second loop. Until that lands, do not paper over the hole with a parallel `random_reach_query` pass in verify.

`conformance.rs` seeded build/replay and `conformance/reach.rs` replay keep their **frozen** reconstructers (today's CQ stream / today's `random_reach_query`). Do not write reach-shaped queries through `render_case` into `seeded-*.json`.

## Acceptance criteria

- [ ] After engine-020: `verify/run.rs`, `corpus_gen/opgen.rs`, `querygen/contradict.rs` call the mixed randomized entry; a verify randomized batch of the default case count includes at least one interiors-or-rec query (pin with a test or a counted report line — do not lower any existing coverage assertion).
- [ ] Frozen: `conformance.rs:1527,1663` still use the CQ-only reconstructer; `conformance/reach.rs` still uses `random_reach_query` with the same `range(8)` mapping. `git diff --stat lean/conformance/cases` empty.
- [ ] Unchanged: 268 checked-in cases byte-identical; stamp semantics otherwise identical.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench`; `./scripts/check.sh`; `./scripts/lean.sh`.

## Constraints

- Blocked on engine-020's randomized entry + bench-004/005. Do not add a parallel reach loop in verify. Do not retarget corpus reconstructers. Corpus frozen (C1). Assertions never weakened.
