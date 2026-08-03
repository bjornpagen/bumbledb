## emit_pair claims to be the one shared traced-artifact fold, but read_family (and half of cmd_trace) hand-roll its body

unification | low | CONFIRMED | obs-estate
outcome: fixed 5d1f7e68 + 713c9b88

### Summary

`trace_out::emit_pair` (crates/bumbledb-bench/src/trace_out.rs:47-69) is documented as "The one traced-artifact fold every `--trace`-bearing lane shares" — split a capture into engine/harness streams, write the `<stem>.{json,folded}` pair, render the engine flame top-10 plus the phase table. `driver/read_family.rs:167-180` re-implements that exact body line for line instead of calling it, and `driver/trace.rs:60-82` re-implements the split+write half twice (warm and cold). The seam whose entire point is being singular has three copies.

### Evidence (verified)

- `crates/bumbledb-bench/src/trace_out.rs:56-69` — `emit_pair`: `split_harness(events)` → `write_trace_pair(dir, stem, &engine, &harness_events).map_err(|e| format!("trace: {e}"))` → `FlameSummary::compute(&engine).render_top(10)` → append `render_phase_table(&engine)` when `Some` → return the table.
- `crates/bumbledb-bench/src/driver/read_family.rs:166-185` — inside `if self.trace`: `harness::traced_sample` → `trace_out::split_harness(events)` → `trace_out::write_trace_pair(&self.trace_dir, &format!("{}.warm", spec.name), …)` with the identical `format!("trace: {e}")` mapping → `render_top(10)` → identical phase-table append → push `FlameEmbed`. Lines 168-180 are byte-equivalent to `let table = trace_out::emit_pair(&self.trace_dir, &format!("{}.warm", spec.name), events)?;` — the stem is already a parameter, so this is a mechanical collapse.
- Every other trace-bearing lane goes through the fold: `capacity.rs:396` and `windowed.rs:300` via `traced_solo`, `lanes/writes.rs:706,739`, `crud/run.rs:294-420` (7 sites), `lawful/run.rs:211-290` (6 sites) via `traced_twin`, and `scenarios/trace.rs:132` via `emit_pair` directly. `read_family.rs` is the only lane carrying a private copy of the full fold.
- `crates/bumbledb-bench/src/driver/trace.rs:61-72,75-82` — `cmd_trace` duplicates `split_harness` + `write_trace_pair` for both halves, but genuinely diverges downstream: it needs the returned artifact paths for `println!("traces: {} / {}", …)` (trace.rs:83) and prints the full `render()` (trace.rs:69), not `render_top(10)`. `emit_pair` discards the path and truncates the table, so today it cannot serve this CLI variant — a deliberate-looking but structurally duplicated half-copy.

### Failure scenario / impact

No behavioral bug today — the copies are currently identical. The risk is drift: any change to the fold's contract (a new artifact written beside the pair, a flame-row tie-break fix, a stem/naming convention change) lands in scenario, crud, lawful, writes, capacity, and windowed artifacts via `emit_pair`, while the read-family warm artifacts and report flame embeds silently keep the stale shape — two artifact dialects inside one baseline report, contradicting the doc comment's explicit singularity claim at trace_out.rs:47.

### Suggested fix

- `read_family.rs:168-180` collapses to `trace_out::emit_pair(&self.trace_dir, &format!("{}.warm", spec.name), events)?` feeding the `FlameEmbed` — pure deletion, no behavior change.
- `cmd_trace` keeps its full-render CLI shape but through the shared seam: either have `emit_pair` return the pair path alongside the table (callers that print paths use it, others drop it), or add a render-rows/full-render parameter so `split_harness → write_trace_pair → summary` has exactly one owner. Either way the doc comment's claim becomes true again.