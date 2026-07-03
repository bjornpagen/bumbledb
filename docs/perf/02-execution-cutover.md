# PRD 02 — Execution cutover: probe, don't scan

Authority: `30-execution.md` (the execute pipeline, the allocation contract),
suite README finding 1. Depends on PRD 00 (selections in the plan) and PRD 01
(`Colt::select`).

## Purpose

The cutover itself: equality constants stop being view filters and become
selection-level probes. After this PRD, a param change on an Eq field costs a
probe, not a scan; the view memo hits across param rotation; and the shim from
PRD 00 is deleted. string's 51.7 µs `view_build`-per-execution becomes one
`view_build` per generation.

## Technical direction

- **Resolution.** Extend the per-execution resolve pass (`api/prepared.rs`,
  `resolve_filters` and its `FilterPredicate`-level helper `resolve_filter`):
  selections resolve through the same machinery into per-occurrence
  `Vec<u64>` key words (`resolved_selections: Vec<Vec<u64>>` scratch on
  `PreparedQuery`, cleared and refilled per execution — capacity retained).
  Semantics preserved exactly:
  - `Const::Param(p)` with `missed_params[p]` true ⇒ the whole query is empty
    (the Eq dictionary-miss short-circuit — today `resolve_filter` returns
    `Ok(None)`; selections return the same signal).
  - `Const::PendingIntern` looks up the dictionary per execution; miss ⇒ empty.
  - `Const::Word`/`Byte` pass through (bytes widen to words).
- **Views.** `run_join` builds views from `occurrence.filters` only — which,
  post-PRD 00, contain no Eq-constant predicates. Delete
  `selections_as_filters` (the PRD 00 shim) and add a
  `debug_assert!` in `run_join` that no resolved filter is
  `Compare { op: Eq, .. }` (unrepresentable by lowering; asserted because the
  plan is constructible by hand).
  The view memo (`built_generation` + `built_filters`) is textually unchanged
  but now keys on residuals only — for every occurrence whose only predicates
  were selections, the memo hits on generation alone, across all params.
- **COLT wiring.** `Colt::reset` receives the occurrence's selection fields
  (PRD 01 signature). After the view/memo loop and before the join, probe:

  ```rust
  for each occurrence with selections:
      match colts[occ].select(&resolved_selections[occ]) {
          Some(cursor) => root_cursors[occ] = (cursor, first_join_level),
          None => return Ok(()),   // empty result, sink untouched
      }
  ```

  Find where the executor initializes `self.cursors[occ]` (in `exec/run.rs`,
  the per-execution cursor reset) and thread the post-selection cursor + level
  offset through it. The `None` early-return must leave the sink in the same
  state as a zero-emit join (the caller finalizes an empty sink — mirror the
  existing resolve-miss short-circuit path, which already does exactly this).
- **Tracing.** `view_build`/`view_memo_hit` semantics are now honest for the
  memo change with no edits. Add one point event
  `names::SELECT_PROBE` (`Category::Execute`, a0 = occurrence index,
  a1 = 1 hit / 0 miss) at each probe so PRD 10's tripwires and human traces can
  see selections. Registry-documented args, as always.
- **Allocation contract.** Probing is allocation-free; lazy subtrie forcing of
  never-probed keys grows COLT slabs to a high-water, exactly like join-level
  laziness. Extend the alloc gate (`tests/alloc_gate.rs`): add a
  string-family-shaped query (Eq param on a non-unique field) rotating **4
  distinct param values**; warm up with two full rotation cycles, then assert
  **zero allocations and zero deallocations** across four further cycles. This
  pins the new steady state: rotating params must not touch the allocator.
- **EXPLAIN/stats.** `ExecutionStats` needs no schema change; the selection's
  effect shows up as node `entries` shrinking to the selected cardinality.
  (PRD 07 fixes the `estimate` side.)

## Non-goals

The memo LRU for residual filters (PRD 03). Cover-choice fixes (PRD 06 — note
selections make the mislabeled-estimate bug *more* visible; that is fine and
expected until 06 lands). Guard-probe unification (unique-key point lookups
keep their dedicated fast path; selections are the non-unique analogue and the
two must not be merged here).

## Passing criteria

- **Behavioral equivalence:** the engine's randomized differential family
  gains Eq-param rotation cases (same query executed across 8 rotating param
  values, results compared against the nested-loop reference each time) and
  passes. The bench crate's full-S `verify` test stays green (results
  bit-identical).
- **The scan is dead**, asserted by trace (feature-gated test, runs in
  check.sh's obs lane): a string-shaped query over a populated store, executed
  9 times with 3 rotating param values, emits **exactly one** `view_build` for
  the Posting occurrence and ≥ 8 `view_memo_hit`s; every execution after the
  first emits `select_probe` with a1=1 for present keys. A never-interned param
  value emits neither `view_build` nor `join` (the resolve short-circuit) and
  returns empty.
- **Work is O(selected)**, asserted by counters: `profile()` on the
  string shape reports node `entries` equal to the selected row count (not the
  relation count) for each of 3 params with known selectivities over a
  hand-built corpus.
- The extended alloc gate passes in release
  (`cargo test --features alloc-counter --test alloc_gate --release`).
- `selections_as_filters` no longer exists (grep-clean).
- `scripts/check.sh` green.
