## The production lowering composition fold_split→gj_split is exercised by zero tests in its non-identity form

incoherence | low | CONFIRMED | gj-split-live
outcome: fixed fa06d11b

### Summary

Every aggregate-head rule is lowered through the composed pipeline `fold_split` → `gj_split` and the result is trusted with a prepare-time `expect`, but the interaction between the two passes — a probe lookup splitting ACROSS the fold boundary so one piece lands in the group-prefix node — is exercised by no test anywhere. Each pass is unit-tested strictly alone, the randomized production-lowering differential omits `fold_split` entirely, and every e2e aggregate test uses a body shape where `gj_split` is the identity (or, in the bench estate's one cyclic aggregate, where `fold_split` is the no-op).

### Evidence (verified)

- **Production composition and the trust point** — `crates/bumbledb/src/api/prepared/build.rs:859` (`fold_split(&mut fj, &group_key, &mut estimates)`), `:861` (`gj_split(&mut fj)`), `:869` (`.expect("binary2fj + factor + fold_split + gj_split construct valid plans")`).
- **Isolated unit tests** — `crates/bumbledb/src/plan/fj/tests/build.rs`: `gj_split` tests at lines 45-141, `fold_split` tests at lines 148-206; no test in the file calls both passes on one plan.
- **Differential omits fold_split** — `crates/bumbledb/src/exec/run/tests/correctness.rs:330-333` lowers with `binary2fj` + `factor` + `gj_split` only, validates with empty sink vars, and drives a projection sink; no aggregate arm, no group key, no `fold_split`.
- **E2e aggregate estate is composition-trivial** — `api/prepared/tests/aggregates.rs` (read in full): every rule body is a single atom; `folded.rs`, `measure.rs`, `tests/edge.rs`, `tests/alloc_gate.rs`, `tests/alloc_census.rs`, `tests/api.rs` aggregate heads are single-atom or acyclic 2-atom joins — shapes on which `gj_split` is the identity (`gj_split.rs:33`). The bench estate's only cyclic aggregate, `bumbledb-bench/src/scenarios/graph.rs:215` (`triangles_from`, a global Count), has an empty group key, so `fold_split` no-ops (`fold_split.rs:27`) — even outside `cargo test` the non-trivial composition is produced nowhere.
- **The straddle shape is real and currently valid (empirical)** — running the exact production pass sequence on `Q(x, Count(z)) :- R(x,y), S(y,z), T(z,x)` with group `{x}` (throwaway probe, since reverted): after `fold_split` the nodes are `[[R(x)], [R(y),S(y)], [S(z),T(z,x)], [T()]]`; `gj_split`'s first-bound map, computed over this post-split node list (`gj_split.rs:21-28`), then splits `T(z,x)` and lands the `x`-piece in the fold-prefix node: `[[R(x),T(x)], [R(y),S(y)], [S(z),T(z)], [T()]]`. `validate` accepts with aggregate-style all-vars sink vars. So today the composition is correct — the finding is a missing pin, not a live bug.

### Failure scenario / impact

A future edit to either pass that stays green under both passes' isolated tests and under the fold-split-free differential — e.g. `fold_split` reordering subatoms or changing where the node's lookups ride, or `gj_split` changing the first-bound scan — can break only the composed case. The first user aggregate over a cyclic body (`Q(g, Count(c)) :- R(g,x), S(x,c), T(c,g)`) then hits the `expect` at `build.rs:869` as a prepare-time panic, or worse, silently violates the scan-fold pushdown's group-constant-prefix precondition (`exec/sink/aggregate/sink.rs` via the `fold_split.rs` doc contract) and produces wrong folds. No existing test constructs the shape, so CI cannot catch it.

### Suggested fix

1. Add a composed case to `plan/fj/tests/build.rs`: run `fold_split` then `gj_split` on the aggregate triangle above, asserting the exact post-split node list (including the `T(x)` lookup landing in the group-prefix node), the duplicated estimates, and that `validate` accepts with the expected covers/trie schemas.
2. Extend the `correctness.rs` production-lowering differential with an aggregate arm: draw a group key, run `fold_split` before `gj_split` (exactly the `build.rs:844-861` tail), and hold the results against the brute-force oracle — ideally through the aggregate sink so the scan-fold pushdown itself is under the differential.

One precision note on the original claim: the two passes *do* run composed inside every grouped-aggregate e2e test (any mixed opening subatom fires `fold_split`; `gj_split` then runs as the identity). The verified gap is exactly the non-identity composition — the fold-boundary straddle — which no test constructs.