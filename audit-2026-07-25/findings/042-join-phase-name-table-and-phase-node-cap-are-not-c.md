## JOIN_PHASE name table and PHASE_NODE_CAP are not compile-time pinned together

incoherence | low | CONFIRMED | obs-estate
outcome: fixed 915afe6e

### Summary

`obs.rs` pins its two other cap-indexed name tables at compile time — `RULE` to `crate::ir::MAX_RULES` and `STRATUM` to `crate::ir::MAX_PREDICATES` — but the third such table, `JOIN_PHASE` (inner width 9), has no pin to `exec::run::PHASE_NODE_CAP` (8), and the bench-side renderer carries its own bare `8`. The three widths are only coincidentally equal; widening `PHASE_NODE_CAP` compiles clean and panics at trace-flush time on the first sufficiently deep plan — precisely the failure mode the sibling asserts were written to make unrepresentable.

### Evidence (verified)

- `crates/bumbledb/src/obs.rs:155-161` — `RULE: [&str; 16]` followed by `const _: () = assert!(crate::ir::MAX_RULES == RULE.len());` with the comment "The cap and the table move together, or the rule loop's span lookup would panic on a legal program."
- `crates/bumbledb/src/obs.rs:167-187` — the identical pattern for `STRATUM` / `MAX_PREDICATES`.
- `crates/bumbledb/src/obs.rs:327-397` — `JOIN_PHASE: [[&str; 9]; 6]`, doc comment `JOIN_PHASE[phase][min(node, 8)]`. No `const _` assert anywhere references it (grep over the repo confirms).
- `crates/bumbledb/src/exec/run.rs:247` — `pub const PHASE_NODE_CAP: usize = 8;` under `#[cfg(feature = "trace")]`; `PhaseTimers.acc`/`open` are `[[_; 6]; PHASE_NODE_CAP + 1]` (run.rs:257,259; counters.rs:29-30).
- `crates/bumbledb/src/exec/run/counters.rs:89,93` — `phase_start`/`phase_end` clamp with `node.min(PHASE_NODE_CAP)`, so accumulator rows run `0..=PHASE_NODE_CAP`.
- `crates/bumbledb/src/exec/run/counters.rs:38-48` — `PhaseTimers::flush` enumerates every accumulator row and indexes `crate::obs::names::JOIN_PHASE[phase][node]` with no bound against the table's own width. The invariant `JOIN_PHASE[0].len() == PHASE_NODE_CAP + 1` (9 == 8+1) holds today by coincidence only.
- `crates/bumbledb-bench/src/trace_out/phase_table.rs:58` — `JOIN_PHASE[phase][node.min(8)]`: a bare `8` independent of `PHASE_NODE_CAP`. (This site cannot itself panic — `parse_phase_name` at phase_table.rs:69-76 derives `node` from a position inside the table — but it is a third un-tied copy of the width, and after a cap bump it would silently mis-render rather than fail.)

### Failure scenario / impact

`PHASE_NODE_CAP` is bumped to 9 to widen per-node attribution. `cargo check` is green; the test suite is green unless some test traces a plan whose join order reaches node index 9. The first traced bench run over a 10-node plan then panics inside `PhaseTimers::flush` (`index out of bounds: the len is 9 but the index is 9`), killing the capture mid-artifact. The two sibling tables demonstrate this exact drift was considered a compile-time-preventable defect class; `JOIN_PHASE` is the one table of the three left unguarded.

### Suggested fix

Next to the `JOIN_PHASE` table in `obs.rs`, add the pin (trace-gated, since `PHASE_NODE_CAP` only exists under `#[cfg(feature = "trace")]` while `obs::names` is compiled unconditionally):

```rust
// The node cap and the table move together, or the trace-flush
// lookup would panic on a legal (deep) plan.
#[cfg(feature = "trace")]
const _: () = assert!(crate::exec::run::PHASE_NODE_CAP + 1 == JOIN_PHASE[0].len());
```

(or place the assert in `run.rs` beside the cap, whichever direction the module graph prefers). Additionally replace the bare `8` at `phase_table.rs:58` — and ideally the `min(node, 8)` in the obs.rs doc comment — with the re-exported `PHASE_NODE_CAP` so all sites move together. A matching `assert!(JOIN_PHASE.len() == 6)` against the `JoinPhase` variant count (counters.rs:12-21 hardcodes indices 0..=5, and the accumulator arrays hardcode `6`) would close the same gap on the other axis.