## Flame fold/summary containment inverts parent/child on equal-tick span pairs

bug | medium | CONFIRMED | obs-estate
outcome: fixed ce8e12b8

### Summary

Both containment sweeps — `fold_stacks` and `FlameSummary::compute` — sort spans by `(start_ns, Reverse(end_ns))` with Rust's **stable** `sort_by_key`. When a parent span and its direct child share both start and end ticks, the sort keys tie and buffer order decides. Buffer order is Drop order, and by the guard discipline the **child records before its encloser** — so at a tie the child lands first, and the stack walk nests the parent under its own child, charging self time to the wrong frame and emitting an inverted folded stack.

### Evidence (verified against code)

- `crates/bumbledb-bench/src/trace_out/fold.rs:21` — `spans.sort_by_key(|e| (e.start_ns, std::cmp::Reverse(e.start_ns + e.dur_ns)))`; identical key at `crates/bumbledb-bench/src/trace_out/flame_summary.rs:19`. `slice::sort_by_key` is stable, so tied keys keep buffer order.
- `crates/bumbledb/src/obs.rs:542-556` — `SpanGuard::drop` is the record site: an inner guard drops (records) before its enclosing guard, so a tied child precedes its parent in the buffer.
- `fold.rs:27-42` stack walk: with child(100, end 150) first and parent(100, end 150) second, the pop condition `spans[top].end <= event.start` is `150 <= 100` → false, the child stays on the stack, and the parent's path becomes `child;parent`. Same walk in `flame_summary.rs:22-34` charges the parent's duration to the child's `child_ns`, giving the parent all the self time as a leaf.
- Ties are physically real: `obs.rs:423-435` documents the 24 MHz `cntvct` tick (41.67 ns granularity), and the drain conversion at `obs.rs:499-503` maps equal ticks to equal ns.
- Concrete producer: `crates/bumbledb/src/api/prepared/fixpoint.rs:324-335` opens the STRATUM span and the round-0 FIXPOINT_ROUND span back to back (same tick plausible). For a stratum converging immediately after round 0, the round guard drops at `fixpoint.rs:381-383` and the stratum guard at `fixpoint.rs:408-411`, separated only by `rounds = 0` and a one-`usize`-read-per-member frontier check — comfortably inside one 41.67 ns tick.
- **Empirically executed**: a tied pair `[child(start 1000, dur 42) recorded first, parent(1000, 42)]` folds to `normalize_fold 0` + `normalize_fold;normalize 42`, and `FlameSummary` reports parent self=42, child self=0 — the truth is the reverse (parent self 0, child self 42). (Run via a temporary scratch test, removed after verification.)

### Failure scenario / impact

A traced recursive query whose stratum converges right after round 0: the `.folded` artifact — and therefore the I3 flame SVG and any diff SVGs built from it — shows `stratum_N` as a child of `fixpoint_round` with all self time on `stratum_N`. A flamediff between two runs where the tie flips (one run ties, the other doesn't) reports a phantom red/blue swap between the two frames. The same inversion applies to any parent/child pair whose open stamps and close stamps each land in one tick.

### Suggested fix

Break the tie by record order descending: sort by `(start, Reverse(end), Reverse(buffer_index))`. At an equal `(start, end)` pair the later-recorded event is necessarily the encloser under the Drop discipline, so this restores correct nesting deterministically. One key change at each of the two sort sites (`fold.rs:21`, `flame_summary.rs:19` — e.g. sort an enumerated `(index, &TraceEvent)` vector), plus one fixture with a tied parent/child pair asserting the fold line is `parent;child` and self time is charged to the child.