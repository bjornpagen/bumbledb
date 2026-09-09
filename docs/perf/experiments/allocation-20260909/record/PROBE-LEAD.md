# Candidate lead: prepare a probe for a run, not every key

Status: **P1 selected; local candidate awaiting acceptance**. Priority was
confirmed against the saved broad traces, source-change audit, current timings
and completed allocation diagnostics (`CENSUS-REVIEW.md`).
Latest user instruction: exhaust their supported improvements before another
full trace; do not wait for or launch the previously planned broad captures.
This is an investigation note, not a new public API or memory framework.

## Direct evidence

- The published 1.1.0 `r4_bomb_t2` median is 1.291 s for a three-way aggregate
  returning one count row. Output materialization cannot explain that by itself.
- The newer historical r4 capture at `df737a6f` attributes substantial exclusive
  self CPU to `probe_sibling_keys` and its callbacks, `probe_walk`, and the gather
  kernel. Full paths place them in the middle-node pipeline, not finalization.
- `git diff df737a6f HEAD -- exec/run/probe_pass.rs exec/colt/probe.rs
  exec/colt/prefetch.rs` (under `crates/bumbledb/src/`) is empty. These hot-path
  source files are byte-identical to the traced revision. This strengthens
  the lead, but does not make the old whole-program profile current evidence.
- The carried-cursor branch resolves `parents[element]`, looks up its cursor,
  and calls `prepare_probe(cursor, level)` for **each** survivor. The uncarried
  branch already holds one prepared probe across its batch.
- `pump` emits a parent's cover keys together before moving to the next parent;
  residual/probe compaction ordinarily retains that order. However callers
  and existing direct tests deliberately permit unordered survivors, so an
  optimization may exploit adjacent equal cursors, not assume sorted elements.

## Question and falsifiable prediction

Can both branches use one borrowed probe over each maximal adjacent equal-cursor
run, with any Row/Map dispatch moved outside its key loop, without introducing
new allocations, cursor storage, persistent metadata, or a fallback policy?

For N keys in G adjacent cursor runs, setup should fall from N resolutions to G.
All logical probes, masks, child results and their order must remain identical.
Cases with long runs should improve the setup/caller portions of the trace;
root-only probes are a control, and alternating-cursor batches are a hostile
control for grouping overhead. No latency percentage is predicted from old
sample shares. A one-row answer does not imply cheap intermediate work.

This does not predict fewer warm heap allocations: the current already-forced
probe path is borrowed. Test that it adds **none**, and consult the separate
allocation census before treating it as the most important memory change.

## Ownership and correctness constraints read in source

1. Resolve/force the cursor before borrowing its Probe. No borrowed Map or pool
   slice may survive a later `prepare_probe` that can relocate the pools.
2. Group by the actual Cursor value in survivor order, never just the parent
   index or a numeric node payload stripped of its Row/Node tag. The occurrence
   and trie level are fixed by the call.
3. Preserve the first force refusal and exactly the successful processed prefix.
   A cancellation error must poison the executor, not become a missing key.
4. Presence-only probes must not load or write child cursors. Child writes use
   original element indices; masks use compacted survivor indices.
5. Keep fixed widths 1/2/3/4 and the dynamic path (including zero and wide keys).
6. Preserve cooperative explored-work polling in the surrounding executor;
   repeated probes on an already-forced map currently do not supply separate
   cancellation polls to remove.
7. Prefetch is a performance variable, not semantics. Moving it from the whole
   cross-parent batch to individual groups can hurt displaced/alternating
   workloads. Compare both the benefit and this risk before adopting a design.
8. Do not force additional nodes merely to classify a future group. Cold forcing
   and refusal order must stay in the actual probe order.

## Required discriminators if selected

- Existing first-refusal test:
  `exec::run::tests::work_ledger::sibling_width_batch_preserves_first_refusal_and_successful_prefix`.
  It covers unordered survivors, repeated Row cursors, cancellation at an
  unforced Map, fixed/dynamic widths, and presence/child modes.
- Add a reference comparison for repeated and alternating Row/Map cursor runs,
  empty/singleton batches, sparse survivors, hits/misses, and a group boundary
  that forces pool relocation. Retain untouched child sentinels on misses.
- Full COLT reference/model, width, reuse and cancellation tests; pipeline,
  interval-membership, residual and answer-equivalence tests.
- Exact allocation checks for warm probes and cold forced-node ownership;
  construction failure must preserve the existing pool rollback invariant.
- Saved profile/source analysis and alternating ordinary controls for r4, r3, triangle,
  displaced probes, graph/OLAP joins, temporal joins and cheap root/point reads.
  The entire supported suite and broad saved-profile survey remain required;
  another full trace is deferred until supported existing leads are exhausted.

## Other representation observation, not a selected fix

COLT uses flat control, bucket and dense-index pools. `grow_map` appends a new
table and dense index list, leaving the old table's space in those pools until
reset; `retained_bytes` measures capacities, not live map bytes. `force_nbuckets`
starts from a duplicate-biased estimate, so high-distinctness maps can rehash.
Cold census call sites and retained-pool measurements must establish the scale
of this waste before any map-layout rewrite. Do not conflate it with warm
per-query allocation or assume a dense-entry indirection will help probes:
successful random lookups could regress even if iteration and memory improve.

### Further source evidence: distinguish live maps from retained construction

`force::ingest_one` checks `(len + 1) * 5 > nbuckets * 16` **before**
checking whether the incoming key already exists. At a map's distinct-key
threshold, even another row for an existing key triggers doubling and rehash.
For example, eight groups admit 25 distinct keys under the current load rule;
a following duplicate triggers 16 groups although the distinct count stays 25.
This is a source-derived discriminator, not a measured prevalence estimate.

`grow_map` appends fresh control/bucket tables and another dense index list.
Earlier construction tables remain in the pool lengths after the map publishes,
not only in spare vector capacity. `clone_bound_from` reserves and copies those
entire lengths, and `api/prepared/run_join.rs` calls it for same-shape occurrence
reuse. Thus dead construction slots can also become subsequent copy traffic.
The actual frequency and byte cost must come from current cold/rebuild evidence;
do not label this a warm allocation or infer a workload speedup from source.

The ordinary force path builds only one new map at a time: `append_child`
creates unforced nodes/chunks, not nested maps. This may permit simpler
construction ownership, but a redesign must preserve old readable maps,
successful-prefix behavior, pool-mark rollback and stable row/node cursors.
Existing model tests cover widths 0/1/2/3/4/5/8, growth, clone equivalence and
batch resume boundaries. Existing growth-refusal tests cancel before resize;
mid-rehash cancellation and later-map preservation need separate coverage if
this mechanism is selected. No implementation has been chosen.

## P1 implementation and validation, September 9 UTC

Only carried sibling probe routing changed. The existing prefetch pass remains
byte-for-byte intact. A borrowed Probe now lasts until the next tagged cursor
differs; next-cursor resolution happens once per survivor, including boundaries.
No sorting assumption, auxiliary vector, persistent field or planner heuristic.

The new store-free differential test spans widths 0/1/2/3/4/5/8 and fixed widths
1/2/3/4, batch sizes 0/1/3/4/7/64 (both sides of prefetch floor 4), repeated and
alternating cursors, different parents sharing a cursor, Row/Node tags, sparse
scrambled survivor indices, presence/child modes, cold forcing and warm reuse.
It uses View::clone_in, the existing explicit ownership API. The original
uncompiled draft's Clone call failed and was fixed before baseline execution.

Baseline selected sibling tests: four passed (session 91107). First candidate
whole-library run: 1,322 passed, 18 ignored (session 67198). Allocation-enabled
execution tests: 287 passed, 11 ignored (`p1-evaluation-1/allocation-tests.log`).
That evaluation stopped at lint before any benchmark: the outer loop needed
while-let form, and an unnecessary test lint expectation was removed. The failed
attempt, source patch, test and exact driver are preserved in p1-evaluation-1.

Retry **session 1549**, `p1-evaluate.py 2`, owns `p1-evaluation-2`. It repeats
library/allocation/lint gates, freezes the ordinary release candidate, verifies
candidate and baseline against the independent oracle, then runs A0/A1/B0/A2/B1/A3
controls serially. A is the original frozen release binary. Each round contains
all 34 scenarios (24 samples) and nine selected affected/control read families
(96 samples, batch 1), with fresh scoped scenario/displaced fixtures. The
driver records hashes and refuses tracked source churn during evaluation. No
profiler/full CPU trace is invoked. End status requires manual review; it does
not accept the candidate automatically. No timing claim, commit or release yet.

### Verification handoff repair (source still frozen)

The candidate and baseline both passed all 2,879 oracle cases. The driver has
a protocol defect: both `verify` commands used `p1-evaluation-2/data`. A stamp
includes the executable fingerprint (`verify/stamp_value.rs`), and the second
verification rebuilt that directory and replaced the candidate's stamp. The
current A rounds are valid; B0's scenario gate is independent. B0's read command
will refuse before timing. Do not fabricate a stamp or call the bypass flag.

The original run remains in session **1549**. A0-scenarios completed at
03:21:28 UTC with all 34 report rows and a recorded artifact hash; A0-reads is
running (last inspected child 74351). Nothing was interrupted or restarted.

`p1-resume.py` is a separate recovery driver; its `check` mode validated the
frozen source, executable hashes, passed gates and completed scenario receipt.
Its monitor is live in session **35796**. It does no benchmark work while the
original is running. After the original finishes, it acquires the measurement
lock and requires the exact expected B0-reads stamp refusal. Other failures
are not silently retried. It then re-verifies the same candidate executable in
a fresh `p1-evaluation-2-resume-1/candidate-data` directory, adopts successful
original reports with their hashes, and completes the remaining alternating
rounds without rebuilding or repeating successful measurements. Baseline reads
continue to use the genuinely baseline-verified original directory. Original
artifacts remain untouched. Resume results still require manual review.

The new candidate was re-audited against `prepare_probe`, the already-forced
fast return, and the scalar differential fixture: there are no removed work
polls in already-forced setup, no live pool borrow across a new force, and no
new per-key heap owner. This is a source/correctness observation, not acceptance
of a CPU speedup. Latest user instruction still prohibits a new full trace.
