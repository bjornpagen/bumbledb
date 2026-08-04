## The overlap index is invisible to the observability estate: no obs event, no counter signal, and its build+query cost runs outside every JoinPhase span

observability | medium | CONFIRMED | overlap-join-live
outcome: fixed b05046cc

### Summary

The leaf overlap enumeration (`overlap_enumerate`, the finding-012 start-sorted max-end index) is the one shipped measured-mechanism with zero observability: it fires no obs event, no `Counters` method records that the index path replaced the generic iterator, and its entire cost — per-group triple collection, `sort_unstable_by_key`, max-end tree build, and the per-outer-row `query_into` walk — executes between `node_entry` and the first `phase_start(Iter)`, so the JOIN_PHASE phase timers attribute it to no phase at all.

### Evidence (verified)

- `crates/bumbledb/src/exec/run/run_node.rs:40` — `counters.node_entry(node_idx)` (a no-op for `PhaseTimers`); `run_node.rs:149-158` — the `overlap_enumerate` call; `run_node.rs:165` — the first `counters.phase_start(node_idx, JoinPhase::Iter)`. The build+query interval spans no phase.
- `crates/bumbledb/src/interval/overlap.rs:117` — `triples.sort_unstable_by_key(|&(start, _, _)| start)` inside `get_or_build` (overlap.rs:105), plus the max-end tree erection; `query_into` at overlap.rs:160. Grep for `obs::` across `src/interval/` and `src/exec/run/overlap_leaf.rs` returns nothing; no `OVERLAP` name exists in `obs.rs`.
- `crates/bumbledb/src/exec/run/counters.rs:88-97` — `PhaseTimers::phase_start/phase_end` accumulate only inside spans; `counters.rs:37-51` — `flush` emits one `JOIN_PHASE[phase][node]` obs event per touched (node, phase). Phase accounting is the estate's attribution unit.
- The `Counters` trait (`exec/run.rs:185+`) has `node_entry`, `batch`, `cover_choice`, `probe_hash`, `probe`, `residual`, `anti_probe`, `emit`, `skip` — nothing records index-vs-generic enumeration. The only observable is the batch-size tally the test `the_overlap_enumeration_prunes_the_leaf_batch_to_true_candidates` leans on (`exec/run/tests/intervals.rs:1079-1119`).
- Precedents, all trace-feature-gated ZST-off (`obs::event` is `#[cfg(feature = "trace")]`, obs.rs:611-612): `COLT_FORCE` fires per force with (positions, distinct keys) at `exec/colt/force.rs:97`; `PREFETCH_PASS` fires inside this same `run_node` function with (batch width, probe footprint) at `run_node.rs:453-459`; `KERNEL_ALLEN` fires with (n, survivors) at `exec/kernel/allen.rs:237-242`.
- Doctrine: `docs/architecture/40-execution.md:1019-1020` — "no per-tuple labels, no always-on counters, no diagnostics allocation anywhere in the join loops." A per-group build event (the COLT_FORCE granularity) and phase-span coverage satisfy the law.
- The crossover re-pin obligation: `exec/run/overlap_leaf.rs:27-30` — `OVERLAP_CROSSOVER = 16` is "provisional... re-pin this number from that sweep, never by inspection," rig `overlap_profile` (`intervals.rs:1311`). That sweep cannot decompose index build vs walk vs residual today.

### Failure scenario / impact

A perf hunt on a temporal lane runs the trace build and reads the JOIN_PHASE breakdown: the leaf node's phases sum well short of the node's wall time because the per-group sorts and per-row tree walks live between spans, and nothing in the trace says the index path even fired. The investigator misattributes the gap (clock overhead, sink) or A/Bs the wrong phase. The owed OVERLAP_CROSSOVER re-pin sweep likewise has no decomposition signal without ad-hoc instrumentation.

### Suggested fix

Two zero-cost-off additions in the existing vocabulary:

1. An `OVERLAP_BUILD` obs event fired in `get_or_build`'s build arm with (entries, tree words) — the COLT_FORCE per-group shape; optionally an `OVERLAP_QUERY` event at run_node-call granularity with (hits, group size), the PREFETCH_PASS batch-granular shape.
2. Attribute the `overlap_enumerate` interval to a phase — `Iter` is the honest owner (the mechanism replaces the iterator): wrap run_node.rs:149-158 in `phase_start`/`phase_end(Iter)` or hoist the first Iter span above the call.

Both are representation, not mode; trace-off compiles to nothing, and neither is per-tuple.