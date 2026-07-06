# PRD 02 — Aggregate batch fold: probe the group once, not per row

## Purpose

The aggregate sink hashes and probes its group map once **per folded row**
even though the trie delivers rows already grouped: stats probes 100,000
times for 512 distinct groups whose key (the account) is constant across
every leaf batch — it was bound at n0 and never changes within the n1
subtree. Same story for balance and skew. The baseline charges this, plus
the per-row per-find `match (op, acc)` dispatch, inside the descend rows
(stats `jp_descend_n1` 3,635 µs). Hoist the group probe to once per batch
run and turn the fold into a straight loop the next PRD can kernelize.

## Technical direction

All in `exec/sink.rs`, on `AggregateSink::emit_batch` (from PRD 01).

- **Classify the group key against the batch, once per batch.** Using
  `LeafBatch::source_of` on each `group_slots[i]`: two regimes.
  - **Constant-group batch** (every group slot reads `Outer`): the whole
    batch folds into ONE accumulator row. This is stats/balance/skew shape.
  - **Varying-group batch** (any group slot reads `Key`): per-row group
    resolution stays (rare in practice; correctness path).
- **Constant-group fast path.**
  1. Gather the key from `bindings` into `key_scratch`; ONE
     `groups.get_or_insert_with` probe per batch.
  2. **Registers, not memory**: copy the group's accumulator row
     (`n_aggs ≤` small) into locals, fold the entire survivor batch into
     the locals, write back once at batch end. The fold loop per
     `FindSpec::Agg` dispatches on the op **outside** the row loop:

     ```rust
     for agg in &self.finds { match op {
         Sum(signed) => { for &s in survivors { acc += decode(keys[..]) } }
         ...
     }}
     ```

     One pass per aggregate over the batch (aggregates are few; passes are
     cache-resident) — this is the loop PRD 03 replaces with kernels, so
     shape it as a call to a free function per op taking
     `(keys, arity, word_idx, survivors)` from the start.
  3. `Count` accumulates `survivors.len() as u64` — no loop at all.
  4. The aggregated-over slot may also be `Outer` (constant over the
     batch): then `Sum += value * count`, `Min/Max` fold the single value
     once. Handle explicitly — it is both a correctness case and free
     speedup. (Sum-of-constant × count must use i128 multiplication —
     `i128::from(v) * i128::from(n)` — semantics identical to n additions.)
- **The dedup regime is untouched but honest.** When `seen` is `Some`
  (the plan could not prove distinct bindings), the batch path must dedup
  full bindings first — per-row `seen.insert` before folding, exactly as
  today, then fold the survivors that inserted. Keep this inside
  `emit_batch` as the slow arm; do NOT try to batch the dedup here. Every
  ledger aggregate family plans with `distinct_bindings == true`
  (serials bound) — assert which regime each family takes in a test using
  EXPLAIN/plan flags so a planner regression can't silently put stats on
  the slow arm.
- **Group-run memoization across batches.** Consecutive batches within one
  node entry share the same outer bindings; remember the last group key
  words + accumulator index and skip even the once-per-batch probe when
  unchanged (compare `key_scratch` words — cheaper than the hash). Reset
  the memo on `reset()`.
- **Tests**: equality of every aggregate family against batch size 1
  (existing sweeps extended to cover: constant-group, varying-group,
  constant-over-slot, empty batches, Sum overflow at the boundary in the
  batch path — the `Overflow { find }` error must be byte-identical);
  the elision-vs-seen equivalence test (`distinct_flag_elision...`)
  re-pointed at `emit_batch`.

## Passing requirements

1. Functional gates green; the overflow determinism test passes in the
   batch path (i128 semantics preserved — no partial-sum i64 wraps).
2. Group probes counted (add a test-only counter or assert via WordMap len
   vs insert calls): stats executes with **≤ 1,100 group probes** per
   execution (one per n1 node entry + slack), not 100,000.
3. Measured (vs baseline; traced sample for phase rows, untraced p50s for
   families):
   - stats p50 ≤ 1,600 µs (baseline 4,130.9); `jp_descend_n1` ≤ 900 µs
     (baseline 3,634.9).
   - balance traced `jp_descend_n1` ≤ 300 µs (baseline 774.3); balance p95
     ≤ 700 µs (baseline 1,110.2).
   - skew traced `jp_descend_n2` ≤ 350 µs (baseline 886.5).
   - No family regresses >5%.

## Out of scope

Kernelized folds (03 — this PRD's loops are plain Rust), suffix gather
fusion (05), the seen-set map layout (06), finalize (08).

## Result (2026-07-06, run bench-out/2026-07-06T23-46-17Z)

**Premise correction, found by this PRD's own regime test.** The gates
assumed stats folds on the elided path. It cannot: stats binds no unique
coverage **by design** — collapsing duplicate (kind, amount, at,
instrument) bindings is the family's set semantics, so its dedup pass is
load-bearing. The pinned regime roster (bench test
`aggregate_family_fold_regimes_are_pinned`): balance elides, stats
dedups. skew, listed in an early task note as an aggregate, is a
projection family — no PRD 02 gate applies to it. Consequence: the
implementation grew a third arm beyond the PRD's two — dedup-then-gather
(seen-set pass per row, first-seen entries gather-folded through the same
hoisted constant-group core) — so the group probe hoist applies to BOTH
aggregate regimes.

Measured (ALL-WIN held, verify green, emits digests identical, no family
regressed — every p50 improved):

- **balance**: p50 12.3 → 2.6 µs (−79%); p95 1,110.2 → 304.8 µs (gate
  ≤ 700 ✓); traced `jp_descend_n1` 774.3 → **35.9 µs** (gate ≤ 300 ✓) —
  50,813 rows folding at ~0.7 ns/row in descend; the elided
  constant-group path is essentially free.
- **stats**: p50 4,130.9 → 2,265.4 µs (−45%); `jp_descend_n1` 3,634.9 →
  1,601.5 µs (−56%). The elision-premised gates (p50 ≤ 1,600,
  descend ≤ 900) miss: the remaining 1,601 µs is ~16 ns/row of
  semantically-required seen-set insert (4-word full-binding keys,
  100,000 rows) — exactly PRD 06's sink-map layout target, recorded
  there as the follow-on expectation.
- Group probes: the unit test pins one probe per group run (8 probes for
  8 groups across 24 batches, memoized); per-row probing is gone from
  both aggregate arms.
- Spillover wins from the cheaper leaf: fk_walk p50 −31%, chain −29%,
  range −19%, skew −33%, spread −6% / p95 −22%.
