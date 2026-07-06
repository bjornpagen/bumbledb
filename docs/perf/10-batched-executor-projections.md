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
