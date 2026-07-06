# PRD 09 — Cross-node batching, part 1: the pipelined executor for skip-free plans

## Purpose

The deepest structural bottleneck the baseline exposes: batches exist only
at the root. Every deeper node re-enters per parent survivor, so its batch
is the parent's fanout — usually 1–10 — and its probes serialize at memory
latency. Triangle: `jp_probe_n1` = 100,000 probe passes of ONE probe each,
60 ns of dependent-chain latency every time (6.0 ms); `jp_iter_n1` 3.1 ms
across 199,536 batch-of-~1 calls; n0's descend bookkeeping 4.2 ms. The
two-phase probe was designed for ~28 misses in flight; deep nodes give it
one. The fix is the Free Join paper's named future work: accumulate
bindings *across* node entries and run each node when it has a full batch.

This PRD builds the batched executor for **skip-free plans** — plans where
`Flow::SkipSuffix` can never cross a node: aggregate plans (hardening
PRD 05 marks every node sink-relevant) and projections whose every node is
sink-relevant. Triangle, stats, balance, skew, spread, range, fk_walk are expected to
qualify; chain is expected not to. Do not trust the expectation: write the
classification test FIRST (assert each family's `skip_free` flag, from its
real plan) and let its output decide which families gate this PRD — any
family that classifies skip-free gates here, the rest gate in PRD 10.
Plans outside the class keep the recursive executor **unchanged** — both
executors coexist permanently unless PRD 10 retires the recursive one.

## Technical direction

New module `exec/pipeline.rs`; plan-time selection in `api/prepared.rs`
(`skip_free = aggregate || all nodes sink_relevant` — computed in
`plan/fj.rs::validate` and stored on `ValidatedPlan`).

- **State per node n** (allocated once at executor construction, zero-alloc
  warm — capacities from `2 × BATCH`):
  - `pending_bindings: Vec<u64>` — entry-major rows of the plan's slot
    words bound so far (full `slot_count` stride: simpler and cache-fine;
    slots not yet bound are garbage and never read — the epoch discipline
    already proves reads are plan-scoped).
  - `pending_cursors: Vec<(u32, Cursor)>` — flattened per-entry cursor
    sets: for each occurrence in `live_occs[n]` (computed at validate:
    occurrences with a subatom at any node ≥ n whose cursor was advanced
    by a node < n), the entry's cursor. Fixed small arity per node ⇒ flat
    `Vec<Cursor>` with stride `live_occs[n].len()`, occurrence ids in a
    per-node table.
  - `pending_len`, plus a scratch identical in role to today's
    `NodeScratch` (entry_keys, survivors, hashes, mask...).
  Global mutable `self.cursors` and the journal **die** for the pipelined
  path: cursors live with the pending entry that owns them. This is the
  end state; no shim keeps both disciplines inside one executor.
- **The loop.** A work-list driven flush machine, NOT recursion:

  ```text
  push the root pseudo-entry into node 0's pending
  loop {
      n = deepest node with pending_len >= BATCH, else
          (on drain) deepest node with pending_len > 0, else break
      run_node_pass(n)   // consumes ALL of node n's pending
  }
  ```

  `run_node_pass(n)`, per pending entry e (this is the expansion): iterate
  e's cover cursor exactly as today's batch loop (`iter_batch` with e's
  cursor, resume tokens per e), producing cover entries tagged with their
  parent index e. Fill the node's probe batch to `BATCH` across *multiple*
  pending entries — the batch buffer rows are `(parent_idx: u32, key
  words)`. When the probe batch fills (or e's are exhausted): phase 1
  hash (sources: `Batch(word)` reads the batch row; `Slot(s)` reads
  `pending_bindings[parent_idx][s]` — the indirection is the price of
  batching; parent rows are hot L1 lines), phase 1.5 prefetch (PRD 07),
  phase 2 probe every sibling with per-entry cursors from
  `pending_cursors[parent_idx]`, residual filter, then **survivors append
  to node n+1's pending**: copy parent's binding row + write the cover's
  new slot words + assemble n+1's cursor set (advanced cover child,
  probed sibling children, inherited others). At the last node, survivors
  go to the sink through the PRD 01/05 leaf paths (batch emit / scan
  pushdown / pinned fusion) instead of a pending buffer.
  - Overflow discipline: `pending` buffers are bounded (`2 × BATCH`); a
    node pass that would overflow the child's pending flushes the child
    first (recursively deeper-first — the natural work-list order above
    already encodes this: deepest-first drains). Bound memory =
    `Σ_n 2·BATCH·(slot_count + live_occs[n]) × 8 B` — a few tens of KB;
    assert it in a test.
  - Determinism: results are sets and fold order is order-free (i128 sums,
    min/max, count); EXPLAIN digest *counters* (batch sizes, node entries)
    legitimately change — update the digest tests' expected shapes, but
    `emits` totals must be identical.
  - `ensure_forced` per sibling happens once per pass (not per parent):
    all parents at node n share the sibling's *node-level* map only when
    their sibling cursors coincide — they do NOT in general (each parent
    advanced the sibling differently). `get_prehashed` takes the
    per-parent cursor exactly as today; forcing remains lazy per cursor.
    (This is why the probe batch is the unit: 128 independent
    `get_prehashed` calls with precomputed hashes and prefetched buckets,
    across parents.)
- **The COLT contract is already shaped for this** — `iter_batch` is
  cursor-explicit and resumable via tokens; `get_prehashed` is
  cursor-explicit. No colt changes should be needed beyond PRD 07's;
  if one appears necessary, stop and re-read — the design intent is that
  the executor owns all cross-entry state.
- **Selection & memo interplay**: none — selection levels resolve before
  the join as today; the pipelined executor starts from `colt.start()`.
- **Tests** (this PRD is where the randomized differential harness earns
  its keep):
  - Extend `randomized_differential_against_the_nested_loop_oracle` to run
    the pipelined executor whenever the generated plan is skip-free, at
    batch sizes {1, 2, 7, 128, 1024}, against the same oracle AND against
    the recursive executor's results.
  - Aggregate equivalence: every aggregate family shape through both
    executors, values bit-identical (including the Sum-overflow error
    class).
  - Fill-boundary unit tests: child pending exactly at, one-under, and
    one-over `BATCH`; a parent whose expansion spans 3 batches (resume
    tokens across pass boundaries); an empty-cover parent (yields
    nothing); a plan where a middle node's cover is `Cursor::Row`
    (fanout-1 pinning must still batch across parents — this is the
    triangle n1 case and the whole point: assert via counters that probe
    passes at that node run with batch length > 1 when multiple parents
    are pending).
  - The memory bound assert above.

## Passing requirements

1. All equivalence and unit tests green; functional gates green (the
   verify oracle now exercises both executors across its 2,468 cases —
   confirm scenario/family plan classification covers both).
2. Counter-proven batching: an EXPLAIN/counter check on triangle showing
   node 1 probe passes execute with **mean batch length ≥ 32** (baseline:
   1).
3. Measured (untraced p50s vs post-08 recorded numbers; phase rows vs
   baseline):
   - triangle p50 ≤ 8,000 µs (baseline 17,480.9); `jp_probe_n1` (or its
     pipelined equivalent row) ≤ 1,500 µs (baseline 6,005.1);
     `jp_iter_n1` ≤ 800 µs (baseline 3,078.3).
   - spread p50 ≤ 5,000 µs (from post-05/06's ≤ ~7,500): n0's descend
     bookkeeping (baseline excl 3,626.8 µs) becomes batch appends.
   - fk_walk p95 ≤ 1,200 µs (baseline 1,797.7 — the deep-walk tail is
     exactly fanout-starved probing).
   - skew p95 ≤ 1,200 µs (baseline 1,880.3).
   - stats/balance/range: within 5% of post-08 (already leaf-dominated;
     the pipeline must not tax them).
4. The recursive executor still runs non-skip-free plans; no
   recursive-path family regresses >5%.

## Out of scope

D2 skip semantics under batching (PRD 10 — chain stays recursive here),
plan selection changes, COLT restructuring beyond PRD 07's.
