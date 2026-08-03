## One unregistered Phase-category event silently suppresses the entire phase table

inappropriate-branching | low | CONFIRMED | obs-estate
outcome: fixed 94ec57f4

### Summary

`render_phase_table` (crates/bumbledb-bench/src/trace_out/phase_table.rs) applies `parse_phase_name(event.name)?` inside its event-collection loop. Because the function returns `Option<String>`, one `Category::Phase` event whose name is not in the `obs::names::JOIN_PHASE` registry makes the entire render return `None` — the same value its doc reserves for "the capture holds no phase events" — silently dropping every valid row already collected.

### Evidence (verified)

- **The abort-on-one mechanism** — `crates/bumbledb-bench/src/trace_out/phase_table.rs:16-18`:
  ```rust
  for event in events.iter().filter(|e| e.cat == Category::Phase) {
      let (phase, node) = parse_phase_name(event.name)?;
      cells.push((node, phase, event.a0, event.a1));
  }
  ```
  `parse_phase_name` (lines 69-76) returns `None` for any name outside the `JOIN_PHASE` registry, and the `?` propagates that as the whole function's return.
- **The conflated contract** — `phase_table.rs:8-9` documents `None` as meaning "the capture holds no phase events (non-join plans, pre-upgrade traces)". The unregistered-name path returns the same `None`, indistinguishable from that benign case.
- **The caller treats None as benign** — `crates/bumbledb-bench/src/trace_out.rs:64-67` (`emit_pair`): `if let Some(phases) = render_phase_table(&engine) { ... }` — on `None` the phase table is simply omitted from the report embed, with no error or log.
- **Unreachable today, by construction** — the only production emitter is `PhaseTimers::flush` (`crates/bumbledb/src/exec/run/counters.rs:37-51`), which emits exactly `names::JOIN_PHASE[phase][node]`. Its accumulator is `[[(u64,u64); 6]; PHASE_NODE_CAP + 1]` with `PHASE_NODE_CAP = 8` (`exec/run.rs:247,257`), indices clamped via `node.min(PHASE_NODE_CAP)` (`counters.rs:89,93`), and the registry is `[[&str; 9]; 6]` (`obs.rs:330`) — so every emitted name round-trips through `parse_phase_name`. This is a latent trap, not a live bug.
- **The trigger is invited** — the `Category::Phase` doc (`obs.rs:26-31`) describes the category generically ("executor phase accumulators... synthetic point events carrying `(total_ns, calls)` per (node, phase)"), not as JOIN_PHASE-registry-exclusive, so a second accumulator flushing under this category with a new name is the natural extension point.

### Failure scenario / impact

A future Phase-category accumulator (e.g. a fixpoint-delta or commit-phase timer) flushes point events under a new name. From that commit on, every traced join query's report embed carries no executor phase table at all — reading as "non-join plan / no phase events" — with zero signal. The regression stays invisible until someone hand-opens a Chrome trace, the lying-by-omission failure mode the observability doctrine (docs/architecture/40-execution.md) designs against.

### Suggested fix

Skip-and-continue instead of abort, keeping `None` strictly for the zero-phase-events case the doc promises:

```rust
let Some((phase, node)) = parse_phase_name(event.name) else { continue };
```

optionally with a `debug_assert!` on registry membership so a non-registry Phase name still trips loudly in checked builds.