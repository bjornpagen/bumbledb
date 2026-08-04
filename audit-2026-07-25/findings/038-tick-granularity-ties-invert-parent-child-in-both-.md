## Tick-granularity ties invert parent/child in both containment sweeps

bug | low | CONFIRMED | cross-branching-new
outcome: fixed ce8e12b8

### Summary
When a nested span pair's open stamps land on the same 41.67 ns counter tick and their close stamps land on the same tick, both `TraceEvent`s carry identical `(start_ns, end_ns)`. The recorder's drop order puts the child earlier in the buffer, the stable sort in both containment sweeps preserves that order, and the stack walk then parents the real parent UNDER its own child — the folded path is inverted and the parent's self time is charged at the wrong leaf name.

### Evidence (verified against code, and reproduced)
- `crates/bumbledb/src/obs.rs:542-556` — `impl Drop for SpanGuard` calls `record(...)` at drop, so an inner guard records before its enclosing guard: buffer order for nested pairs is child-before-parent.
- `crates/bumbledb-bench/src/trace_out/fold.rs:21` and `crates/bumbledb-bench/src/trace_out/flame_summary.rs:19` — identical sweep: `spans.sort_by_key(|e| (e.start_ns, std::cmp::Reverse(e.start_ns + e.dur_ns)))`. Rust's `sort_by_key` is stable, so an equal-start, equal-end pair keeps child-first order. There is no tertiary tiebreaker.
- Walk (`fold.rs:27-42`, `flame_summary.rs:22-34`): pop condition is `top.end <= event.start`. Child processed first becomes a root; for the parent, `child.end > parent.start` (dur > 0), so nothing pops and the parent is pushed as the child's child.
- **Reproduced empirically** against the real functions with a drop-order fixture `[child("normalize_fold", start 1000, dur 42), parent("normalize", start 1000, dur 42)]`:
  - `fold_stacks` emitted `normalize_fold;normalize 42` (and `normalize_fold 0`) — correct output is `normalize;normalize_fold 42`.
  - `FlameSummary::compute` charged `self=42` to `normalize`, `self=0` to `normalize_fold` — the self-time charge is on the wrong name.
- Ties are routine, per the module's own spec: `obs.rs:423-435` calls the 24 MHz / 41.67 ns tick granularity "the real limit" for the "sub-500 ns regions" these spans time, and `NORMALIZE_FOLD` / `PLACE_COMPARISONS` (`obs.rs` names block, ~99-108) are per-rule sub-spans directly under `NORMALIZE`. A tie only requires the parent's overhead outside the child to fit within one tick on each side. Equal ticks convert to equal ns at drain (`finish_capture`, obs.rs:494-504), so the artifact pipeline sees exact ties.
- No existing fixture pins tie orientation: `trace_out/tests.rs` has no equal-stamp events.

### Failure scenario / impact
A traced run over trivial single-rule queries: `normalize` and its one `normalize_fold` sub-pass open within one tick and close within one tick. Both the `.folded` artifact and the flame summary table invert the stack (`prepare;normalize_fold;normalize`) and attribute normalize's self time to the wrong name, so a perf pass reading TALLY/flame artifacts chases the wrong sub-pass. Artifact-only; no engine correctness impact.

### Suggested fix
Use recording order as the final sort key, descending. This tiebreak is provably correct: the buffer is thread-local, and two dur>0 spans on one thread cannot have identical `(start, end)` ticks unless nested (a sequential sibling's open stamp cannot precede the prior sibling's close tick), and for a nested pair the later-recorded event is the ancestor by the drop-order invariant. Concretely, in both sweeps sort enumerated indices by `(start_ns, Reverse(end_ns), Reverse(buffer_index))` instead of sorting the `Vec<&TraceEvent>` directly. Land with a fixture of two equal-stamp events in drop order pinning `parent;child` orientation and the child-leaf self-time charge (the reproduction above is the fixture, with assertions).