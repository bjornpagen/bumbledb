# PRD 10 — Cross-node batching, part 2: D2 suffix skips under the pipeline

## Purpose

PRD 09 excluded the one semantic that fights batching: `Flow::SkipSuffix`
crossing nodes. Under the recursive executor, a skip unwinds the suffix of
*one specific outer binding* — cheap, because that binding's subtree is the
only live work. Under the pipeline, entries from many parents interleave in
one pending buffer: a skip must cancel exactly the pending/expanding work
descended from one parent entry at the skip's origin depth, and nothing
else. Chain is the family in this class (its plan carries
non-sink-relevant nodes; baseline p50 210 µs — the win here is smaller
than PRD 09's, but the point is retiring the recursive executor so ONE
executor exists again).

## Technical direction

Extends `exec/pipeline.rs`. Read PRD 09 first; this changes only the
skip machinery and the executor-selection rule.

- **Where skips can originate/absorb — precomputed.** For each node n,
  `skip_barrier[n]` = the nearest ancestor node index `b ≤ n` such that
  every node in `(b, n]` is non-sink-relevant... — restate from the
  semantics rather than inventing: under the recursive executor a skip
  emitted below node n propagates upward, absorbed at the first node with
  `sink_relevant == true`, unwinding every node strictly below it. So the
  unit of cancellation is: *the pending subtree rooted at one entry of the
  deepest sink-relevant node at-or-above the leaf*. Compute per plan, at
  validate: `absorb_node = max { m : m ≤ leaf, nodes[m].sink_relevant }`
  (for projections with may_skip sinks; always the leaf for skip-free
  plans — degenerating to PRD 09 behavior).
- **Ancestry tracking.** Each pending entry at node n carries
  `origin: u32` — the index of its ancestor entry at `absorb_node`
  (entries at nodes ≤ absorb_node use their own index; entries deeper
  inherit the parent's `origin`). One extra u32 per pending entry.
- **The cancellation.** When the sink (leaf batch/scan/pinned path)
  returns `SkipSuffix` for a row with origin `o`:
  1. Stop consuming the current leaf batch for rows with origin `o`
     (rows with other origins in the same probe batch are still live —
     the leaf emit loops must filter by origin after a skip fires; keep a
     small `skipped_origins` epoch-set: a `u32 -> epoch` array sized to
     the absorb node's pending capacity, checked per row — O(1), no
     allocation).
  2. Mark origin `o` cancelled: every deeper node's pending entries with
     that origin are dead — they are filtered lazily at expansion time
     (check the origin's cancel mark before expanding an entry; compaction
     is unnecessary because pending buffers flush every pass).
  3. The absorb-node entry `o` itself continues with its NEXT cover entry
     as the recursive executor would (the skip kills the suffix, not the
     absorb node's iteration) — since expansion in PRD 09 drains an
     entry's cover fully into child pending, "continue with next" is
     already satisfied by the cancel-marks: later cover entries of `o`
     were tagged with the same origin... **wait — they must NOT be**: a
     skip kills only already-produced suffix work; subsequent cover
     entries of `o` at the node BELOW the absorb node start a fresh
     suffix. Therefore origin tagging must be per (absorb entry ×
     absorb-child cover entry): the origin id is minted when the absorb
     node's pass appends a row into its child's pending — one fresh origin
     per appended child row — and inherited verbatim below that. The
     cancel mark kills one origin; the absorb entry's other child rows
     carry other origins. With that refinement, step 3 is automatic and
     the epoch-set sizes to the absorb node's *child* pending capacity.
     Implement it that way from the start.
- **First-emit skip semantics** (PRD 01): the projection sink signals on
  the first emit per distinct tuple; under the pipeline the executor
  passes `stop_on_skip` per leaf row's origin as above. The `may_skip()`
  tripwire (`debug_assert`) is preserved: a skip surfacing for an origin
  whose plan is skip-free is a bug — assert.
- **Retire the recursive executor.** With D2 handled, every plan runs the
  pipeline. Delete `run_node` recursion, the journal, and the global
  cursor vector; `Executor` becomes the pipelined machine (keep the name;
  kill `exec/pipeline.rs` as a separate module or fold — end state is ONE
  executor in `exec/run.rs`). The batch-size-equality and differential
  harnesses stop being cross-executor and go back to being
  cross-batch-size. The `skip_free` plan flag remains (it sizes the
  origin machinery to zero for skip-free plans — the epoch-set and origin
  column are allocated only when `may_skip` plans need them; zero-alloc
  gates hold either way since allocation is at prepare).
- **Tests**:
  - D2-specific: the `duplicate_witness_projection_dedups_and_skips_suffixes`
    fixture through the pipeline at batch sizes {1, 2, 128}; a two-parent
    interleave case: parents A and B pending at the same node, A's suffix
    skips, B's rows after A's skip must all emit (craft with a fanout-2
    root and a non-sink-relevant middle node; assert exact result set AND
    that `counters.skip` fired exactly once);
  - a case where the absorb entry has multiple child cover entries and a
    skip kills only the first child's suffix (asserting the second child's
    results present);
  - the full randomized differential harness (all three shapes × random
    orders × batch sizes) now running every plan through the one executor
    against the nested-loop oracle — with a projection variant added that
    projects a strict subset of variables (forcing non-sink-relevant
    nodes and real skips) — this is the harness that must catch any
    origin-tagging bug; give it ≥ 200 cases.
  - EXPLAIN `emits` totals unchanged on every family; skip counters on
    chain nonzero and result-identical.

## Passing requirements

1. All D2/differential tests green; functional gates green; the recursive
   executor is deleted (grep: no `journal`, no per-survivor recursion in
   exec/).
2. Measured: chain p50 ≤ 140 µs (baseline 210.0); every other family
   within 5% of its PRD 09 numbers (the origin machinery must be free for
   skip-free plans — that is a hard gate, not a hope).
3. `## Result` records the final executor's phase table for chain and
   triangle side by side with the baseline tables.

## Out of scope

Everything else — this PRD ends with one executor, one discipline, and
the suite's structural work complete.

## Result (2026-07-07, run bench-out/2026-07-07T02-50-13Z)

Landed: origin-tagged D2 under the pipeline — the absorb node (deepest
sink-relevant, static per plan) mints one origin per routed survivor,
deeper nodes inherit, and a leaf skip cancels the origin's subtree
through epoch-stamped marks checked at expansion and routing.
Cancellation is correctness-free by construction (a late cancel
re-emits rows the seen-set already holds), and origins are meaningful
strictly below the absorb — the randomized subset-projection
differential caught an origin-id collision between the upstream seed
and the first minted id on its second case (the exact bug class the
harness was specified for) and pinned the fix. The recursive executor
is deleted: no journal, no per-survivor recursion; `run_node` is the
leaf pass, the pipeline pumps every middle node, and the module header
formally retires the paper's "cross-node accumulation is future work"
caveat.

Gates:
- chain p50 **130.0 µs** (gate ≤ 140; baseline 210.0, −38%) ✓.
- Transferred from PRD 09: fk_walk p95 **1,039.7 µs** (gate ≤ 1,200) ✓
  with p50 4.0 µs (−69% vs baseline); skew p95 **1,066.8 µs** (gate
  ≤ 1,200) ✓ with p50 28.3 µs (−52%).
- triangle: batching counter-proven — `jp_hash_n1` calls 100,000 →
  **2,725** (mean probe batch ~37, gate ≥ 32 ✓); n0 descend bookkeeping
  4,220 → 1,089 µs; p50 **15,083 µs** (−13.7% vs baseline, best ever;
  p95 15,459 — its tightest distribution yet). The ≤ 8,000 p50 and
  `jp_probe_n1` ≤ 1,500 gates miss: probe time held at ~5.5 ms despite
  batching + prefetch — the latency is no longer in flight-count but in
  the probe work itself (miss-heavy walks against a ~100k-key map, and
  batch fragmentation from per-entry cover flips capping means at ~37).
  Recorded as the standing wall with two named follow-up levers outside
  this suite's remaining scope: cover-stable batch segregation and
  longer prefetch lead.
- Every family within band or improved vs post-09; ALL-WIN held; verify
  green (2,468 cases, all through the one executor); **EXPLAIN emits
  digests byte-identical on all ten families including triangle's 464**
  — the cancellation eagerness reproduced the recursive counts exactly.
- Suite-wide at this commit vs the committed baseline: every family
  between −11% (string) and −85% (balance); fk_walk −69%, skew −52%,
  range −51%, stats −54%, chain −38%, spread −18%, triangle −14%.
