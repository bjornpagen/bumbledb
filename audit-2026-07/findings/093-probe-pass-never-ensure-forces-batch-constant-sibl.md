## probe_pass never ensure_forces batch-constant first-appearance sibling cursors: dead prefetch sweep and force time misbooked as Probe

category: perf | severity: low | verdict: CONFIRMED | finder: engine:run
outcome: fixed 0f13feff

### Summary

`run_node`'s sibling loop force-builds a sibling's COLT map under its own `JoinPhase::Force` before hashing and prefetching (`crates/bumbledb/src/exec/run/run_node.rs:190-192`). Its explicitly line-parallel twin, `probe_pass`, never calls `ensure_forced` at all. Per-element carried cursors genuinely cannot be batch-forced, but in the `carried_col = None` case — a sibling occurrence's first appearance at this node — the cursor is the batch-constant `colts[occ].start()` (`crates/bumbledb/src/exec/run/probe_pass.rs:183-184`), which is a `Cursor::Node` born `Unforced`. Two consequences on that sibling's first batch:

1. The phase-1.5 prefetch sweep (`probe_pass.rs:186-199`) runs its full survivor loop against a node `prefetch_bucket` silently no-ops on (`crates/bumbledb/src/exec/colt/prefetch.rs:10-13`: non-`Forced` ⇒ early return) — the entire sweep is pure overhead, plus a spurious `PREFETCH_PASS` obs event for a pass that prefetched nothing.
2. The force fires lazily inside phase 2's first `get_prehashed` (`crates/bumbledb/src/exec/colt/probe.rs:49-50` → `force.rs:7`, the O(positions) single-pass ingest), between `phase_start(node_idx, JoinPhase::Probe)` at `probe_pass.rs:201` and `phase_end` at `:234`. The identical event in `run_node` is attributed to `JoinPhase::Force`, whose own doc comment (`crates/bumbledb/src/exec/run.rs:175-179`) calls the force "the single biggest non-amortized cost a node entry can pay" — the exact cost class the phase exists to isolate.

### Evidence (all verified in source)

- `probe_pass.rs` (entire file, 593 lines): zero `ensure_forced` calls. `grep -rn ensure_forced src/exec` finds only `run_node.rs:191` and `anti_probe.rs:150` as production call sites.
- `probe_pass.rs:183-184, 195, 219`: `let carried = tables.carried_col[node_idx][occ]; let start_cursor = colts[occ].start();` — `carried.map_or(start_cursor, ...)` in both the prefetch and probe loops.
- `colt/select.rs:195-198` + `colt/new.rs:26,51`: `start()` returns a `Cursor::Node` whose `NodeState` starts `Unforced` (root or post-selection union node).
- `colt/prefetch.rs:10-13`: `let NodeState::Forced { map } = ... else { return; }` — unforced node ⇒ every prefetch call in the sweep is a wasted load-and-branch.
- `colt/probe.rs:49-50` and `colt/force.rs:7-75`: the lazy force inside `get_prehashed`, an O(count) ingest of every position under the cursor.
- Nothing forces the node earlier: `pump.rs:78` calls `key_count`, and `colt/count.rs:56-58` is documented and implemented as "never forces"; pump's `iter_batch` touches only the cover occurrence.
- Reachability: `execute.rs:449` drives ALL non-leaf nodes through `pump` → `probe_pass` (`run_node.rs:30` debug_assert: "run_node is the leaf pass; middle nodes pump"). `pipe_tables.rs:32-39`: `carried_col = None` exactly when the occurrence has no subatom before this node — first appearance. Any plan where a relation first appears as a non-cover subatom at a non-leaf node hits this (e.g. the triangle family: the second node's sibling S).
- Twin doctrine: `probe_pass.rs:47-50` and `run_node.rs:166-168` both declare the two sibling loops line-parallel — "a change here needs its mirror there". This is a standing divergence.
- Paper check: Free Join `docs/free-join-paper/arXiv-2301.10841v2/tex/04-optimizations.tex` § 4.3 (COLT, Column-Oriented Lazy Trie) mandates building subtries lazily on first `get` — correctness is untouched; the issue is purely the wasted prefetch sweep and the per-phase attribution the repo's own introspection layer (docs/architecture/60-validation.md phase table, `run.rs:158-180`) exists to keep honest.

### Bench impact

Small and bounded: one wasted prefetch sweep (up to one batch of no-op `prefetch_bucket` calls, floor 4 per `run.rs:421`) plus one misattributed O(positions) force per (execution, non-leaf node, first-appearance sibling). Steady-state passes are unaffected because the node is Forced after the first probe. The real cost is introspection honesty: under the counting `Counters`, the largest non-amortized event a middle node pays lands in `Probe` in the pipelined executor but in `Force` at the leaf/single-node executor, skewing exactly the per-(node, phase) attribution the twins are documented to keep line-parallel.

### Suggested fix

In `probe_pass`'s sibling loop, when `carried.is_none()`, mirror the twin: wrap `colts[occ].ensure_forced(start_cursor, s_level)` in `phase_start/phase_end(node_idx, JoinPhase::Force)` before the hash phase (or immediately before the phase-1.5 gate). This is never wasted work — with survivors nonempty, phase 2's first `get_prehashed` forces the same node anyway. Per-element carried cursors keep the lazy behavior they must have (they may be pinned `Cursor::Row`s that never need a map). Update both twins' loop-head comments to record the mirrored line.
