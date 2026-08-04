## check_capacity fetches the parent F row eagerly even when ψ is empty and the bound is literal — and the doc describes the lazy design

perf | low | CONFIRMED | capacity-judge
outcome: fixed 2b1e87b0

### Summary

In the keyed-parent `ScalarProbe` arm of `check_capacity`, the `U` determinant probe is always followed by a `fact_by_row` `F` get (`crates/bumbledb/src/storage/commit/judgment.rs:1126-1130`), justified by the doc comment at judgment.rs:1083-1085: "ZERO extra descents: both arms bind the full target fact before the verdict — the ψ check and the violation's fact bytes require it anyway." For the commonest capacity shape — unit or field weight, empty ψ, literal or absent bound — that justification is false before the verdict: nothing on the accept path reads the fact. Meanwhile `docs/architecture/50-storage.md:384-385` specifies the lazy cost model: "one get, plus one `F` get where ψ is nonempty". The code is eager where the architecture doc says lazy — simultaneously a doc-vs-code drift and one avoidable B-tree descent per touched parent per commit on the judged surface.

### Evidence (verified)

- `crates/bumbledb/src/storage/commit/judgment.rs:1126-1131` — keyed (non-fresh-row) arm: `U` get → `decode_row_id` → unconditional `fact_by_row(self.data, self.txn, statement.target.relation, row_id)?`.
- `judgment.rs:1133` — the fetched fact's first use is `satisfies(&checks.target, layout, fact)`; `judgment.rs:344-345` — `SelectionCheck::Empty => true`, never touching the fact bytes. Empty ψ is a real reachable shape: `resolve_checks` returns `SelectionCheck::Empty` when the compiled check list is empty (`judgment.rs:306-308`), and capacity statements get per-id `SideChecks` (`judgment.rs:292`).
- `judgment.rs:1187-1193` — `resolve_hi`: `statement.hi == None` returns early; `CapacityBound::Lit(n)` ignores `parent_fact`. Only `TargetField`/`TargetDuration` read it.
- `judgment.rs:1232-1238` — `measure_children` does not take `parent_fact` at all; it measures via the `R` bucket walk (or closed-extension scan) keyed by `parent_key`.
- `judgment.rs:1170-1177` — the only remaining consumer of `parent_fact` is the `Violation::Capacity { fact: parent_fact.into(), .. }` payload, i.e. the cold conviction path.
- `docs/architecture/50-storage.md:384-385` — the stated cost model: "probes the target key's `U` determinant for its ψ-selected holder in the final state (one get, plus one `F` get where ψ is nonempty; a closed parent answers from the compiled member set)".
- Correctly out of scope: the fresh-row arm (`judgment.rs:1106-1118`) has no `U` tree — the single `F` get IS the probe; the `Closed` arm (`judgment.rs:1140-1156`) reads the fact from the in-memory extension, zero descents.

### Failure scenario / impact

Every commit touching N keyed parents of an empty-ψ, literal-window (or floor-only) capacity statement pays N unneeded `F` B-tree descents in phase 3, on the accept path — the hot path. This is a slice of the judged-surface overhead the C17 bench measured (+12.6..+14.1 µs on the commit_capacity lanes). No correctness impact; the verdict and witness are unchanged.

### Suggested fix

Defer `fact_by_row` in the keyed arm to the points that actually need the fact:
- run the ψ check only when `checks.target` is non-`Empty` (fetch then);
- resolve the fact-dependent bound only for `TargetField`/`TargetDuration` (fetch then, if not already);
- on conviction, fetch the holder fact for the violation payload — a re-fetch on the cold reject path is the repo's own established pattern (`applier.rs:294-296`: "Cold aborting path: name the incumbent by its fact bytes via row_id → F get").

This restores the doc's stated cost model at `50-storage.md:384-385` verbatim, and the comment at judgment.rs:1083-1088 should be rewritten to describe the lazy binding. Land with a test/bench pin on the empty-ψ literal-bound lane.