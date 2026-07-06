# PRD 01 — Leaf batch emit: stop recursing into a function that only emits

## Purpose

At the last plan node, `run_node` still recurses once per surviving element:
per row it writes cover bindings, saves/restores the cursor journal, makes a
recursive call whose entire body is `sink.emit(bindings)`, and unwinds. The
baseline charges this per-row framework at ~15–40 ns/row everywhere outputs
are wide: range `jp_descend_n0` 38.7 µs for 2,000 rows, chain
`jp_descend_n2` 38.5 µs for 1,914, balance `jp_descend_n1` 774 µs for
50,813, stats `jp_descend_n1` 3,635 µs for 100,000. No deeper node needs
those cursors; the recursion is pure ceremony. Replace it with a batch
handoff: the sink receives the whole survivor batch at once.

## Technical direction

- **The batch view.** `exec/run.rs`: a borrowed, non-owning view over one
  leaf batch —

  ```rust
  pub struct LeafBatch<'a> {
      /// Cover-entry key words, entry-major (`entry * arity + word`).
      pub keys: &'a [u64],
      pub arity: usize,
      /// Surviving entry indices into `keys` (post probe/residual compaction).
      pub survivors: &'a [u32],
      /// Binding slot of each cover key word, in word order.
      pub key_slots: &'a [usize],
      /// Outer bindings (slots not covered by `key_slots` are already bound).
      pub bindings: &'a Bindings,
  }
  ```

  A sink reads a find/group slot either from `keys` (if the slot is in
  `key_slots`) or from `bindings` (otherwise). Sinks precompute that split
  once per batch, not per row — expose
  `LeafBatch::source_of(slot) -> LeafSource` (`Key(word_idx)` | `Outer`),
  a linear scan over `key_slots` (arity ≤ a handful; no map).
- **The trait.** `Sink` gains

  ```rust
  /// Emits every surviving element of a leaf batch. `stop_on_skip`: the
  /// executor's translation of the leaf node's sink-relevance — when
  /// true, the sink must stop at the first row whose emit would have
  /// returned SkipSuffix and return SkipSuffix; when false it must
  /// consume the entire batch and return Continue.
  fn emit_batch(&mut self, batch: &LeafBatch<'_>, stop_on_skip: bool) -> Flow;
  ```

  This **replaces** per-row `emit` as the leaf path; `emit` stays only for
  the guard-probe plan (single row by construction) and tests. No default
  implementation — both sinks implement it natively (no shims).
- **ProjectionSink::emit_batch**: per survivor, gather the projected slots
  via the precomputed sources into `scratch`, `seen.insert`. Semantics of
  D2 at the leaf, preserved exactly: today every row's emit returns
  `SkipSuffix`, and the executor absorbs it when the leaf node is
  sink-relevant, else unwinds after the *first* row. Therefore:
  `stop_on_skip == true` → insert the first survivor only, return
  `SkipSuffix` (the executor unwinds — later survivors of this batch bind
  nothing sink-relevant, exactly the rows the old path never visited);
  `stop_on_skip == false` → insert all survivors, return `Continue`.
- **AggregateSink::emit_batch** (naive in this PRD; PRD 02 rewrites it):
  loop survivors, reuse the existing per-row body (dedup via `seen` when
  present, group probe, accumulate). `stop_on_skip` is statically false for
  aggregate plans (hardening PRD 05 marks every node sink-relevant) —
  `debug_assert!(!stop_on_skip)`.
- **The executor change.** In `run_node`, when `node_idx + 1 ==
  plan.nodes().len()`: after residual compaction, do **not** enter the
  per-survivor recursion loop. Build the `LeafBatch` (keys =
  `scratch.entry_keys`, key_slots = `self.slot_map[node_idx][cover_sub]`),
  call `sink.emit_batch(&batch, stop_on_skip)` where `stop_on_skip =
  !plan.nodes()[node_idx].sink_relevant && sink.may_skip()`. On
  `SkipSuffix`, behave exactly as the old absorption arm: propagate the
  unwind (`counters.skip(node_idx)`, `break 'outer`) — sink-relevance was
  already folded into `stop_on_skip`, so a returned skip is always a real
  unwind. Cursor writes and journal restores for the leaf vanish entirely
  (nothing below reads them); `bindings` is **not** written with leaf values
  (the batch carries them) — this is why the sinks read through `LeafBatch`.
  `counters.emit()` fires once per emitted row still (EXPLAIN digests must
  not change meaning): call it `survivors.len()` times or add a
  `counters.emits(n)` bulk method and update the explain counter — either
  way EXPLAIN's `emits` numbers must be identical to before for every
  family (assert in the digest tests).
  The single-node-plan case (`nodes.len() == 1`) takes the same path.
- **The middle nodes are untouched** in this PRD — per-survivor recursion
  remains for `node_idx + 1 < len` (PRDs 09/10 own that).
- **Phase attribution**: the leaf batch emit is timed as the leaf node's
  `Descend` phase (same name, so baselines compare); the phase now wraps
  the `emit_batch` call instead of a loop of recursions.
- **Tests to add/extend** (executor tests, run.rs + sink.rs): batch-size
  equality sweeps already cover result identity; add a D2 leaf case —
  a projection whose leaf node is *not* sink-relevant with >1 leaf
  survivors per batch, asserting result identity with batch sizes {1, 128}
  and that `counters.skip` fired at the leaf; and an aggregate case
  asserting fold values are identical to the scalar path at every batch
  size (Sum near i64::MAX included — the overflow class must not change).

## Passing requirements

1. `emit` no longer reachable from `run_node` (grep-provable: the only
   `sink.emit(` call sites are the guard path and tests).
2. EXPLAIN `emits` digests unchanged for all ten read families.
3. Functional gates (README invariants) green, including the new D2 leaf
   test and aggregate batch-fold equality test.
4. Measured (untraced p50s vs baseline, trace phase rows vs baseline):
   - range p50 ≤ 50 µs (baseline 59.1) and `jp_descend_n0` ≤ 25 µs
     (baseline 38.7).
   - chain `jp_descend_n2` ≤ 20 µs (baseline 38.5).
   - balance traced-sample `jp_descend_n1` ≤ 550 µs (baseline 774.3).
   - stats traced-sample `jp_descend_n1` ≤ 2,900 µs (baseline 3,634.9 —
     the group probe still dominates; PRD 02 owns it).
   - No family regresses >5%.

## Out of scope

Fold vectorization (02/03), the group-key probe (02), pinned-row leaf
elision and gather fusion (05), middle-node recursion (09/10).
