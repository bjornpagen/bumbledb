# 30 — Execution

The execution engine is Free Join as specified in the paper (Wang, Willsey, Suciu,
SIGMOD 2023 — `docs/free-join-paper/`), run over snapshot-local columnar data, with
documented deviations. When this doc and the paper disagree and no `Deviation:` block
explains why, this doc is wrong.

## Access paths (before any join machinery)

**Guard-probe point lookups.** A single-atom query whose bindings cover a unique
constraint of the relation (including the auto-unique on serial fields) or the full
fact executes as: one `U`-guard (or `M`-membership) LMDB get → one `F` fetch → decode.
No images, no COLT, no plan search. This serves the headline "point lookup by unique
key" workload at O(log n), including immediately after a commit (no rebuild cost).
**Decision.** **Alternative:** COLT-only ("the join engine is the only read path") —
lost because a fully-bound lookup through images pays an O(n) scan for a one-row answer
and loses the benchmark family outright; the paper itself lists index-blindness as an
open limitation (§6). **Reverses if:** never — the guards exist anyway (rule: every
mechanism names its reader; this is `U`/`M`'s read-side reader).

**Time-range scans are O(n)** (image scan + filter) in v0 — decided; acceptability is
policed by the latency budget (`00-product.md`), and the range-accelerator OPEN item
triggers on violation.

Everything else executes as Free Join.

## Inputs from normalization

Execution consumes `20-query-ir.md`'s normalized form: distinct-variable atom
occurrences, per-atom filter lists, and a residual comparison list.

- **Per-atom filters** evaluate at the source: a query-local **filtered view** — a
  survivor-position vector over the cached full-width image, arena-backed. On a cold
  relation, one scan produces both the cached unfiltered image and the survivor view
  (`40-storage.md`). COLT roots iterate the view; view positions index the image.
- **Residual comparisons** attach to the earliest plan node at which both sides are
  bound (computed at plan time, stored in the plan). The executor's node loop gains one
  step: iterate cover → probe siblings → **evaluate the node's residuals** → recurse.
  In vectorized execution, residuals run as batch survivor compaction after the probes.

**Deviation (paper §2):** the paper assumes selections pre-pushed to base tables and has
no residual concept; we own filter placement because there is no external optimizer.
**Reverses if:** never — WLOG assumption, not a design.

## The paper's core, adopted

- **GHT** (§3.1): trie; internal nodes are hash maps keyed by tuples; leaves are vectors.
- **Plan** (§3.2): a list of nodes, each a list of subatoms; the plan partitions every
  atom occurrence's variables; per node, no two subatoms share an atom occurrence
  (validity quantifies over **occurrences** — self-joins are ordinary), and the cover
  set is every subatom whose variables are **exactly** the node's new variables.
  **Deviation from the paper's Definition** ("containing all new variables"): a
  subatom that also carries an already-bound variable is iterable per the paper, but
  under dynamic cover choice the executor would *rebind* the bound variable without
  re-checking the occurrence that bound it — wrong results on skewed data (found by
  audit, demonstrated with a triangle query, pinned by a regression test). Restricting
  covers to exactly-the-new-variables loses nothing: every `binary2fj` node's opening
  subatom qualifies (its variables are exactly the remainder), and GJ-style
  single-variable covers all qualify. The alternative — equality-checking a mixed
  cover's old variables per iterated entry — buys generality no plan shape here needs.
- **Execution** (§3.3): recurse node by node — choose a cover, iterate it, extend the
  binding, probe siblings, replace each occurrence's current trie, recurse; backtrack
  restores.
- **COLT** (§4.2): lazy tries — a node is offsets into the base columns or a forced
  map; roots iterate the base image (or filtered view) directly; forcing happens only
  on `get` or non-suffix `iter`. Under laziness the paper's build-phase "drop the
  trailing []" question dissolves: nothing is ever built eagerly; a last-level subatom
  that is only ever suffix-iterated is never forced by construction, and one that gets
  probed forces like any level.
- **Dynamic cover choice** (§4.4): at node entry, iterate the cover with the fewest
  keys; forced maps expose `Exact(n)`, unforced vectors `Estimate(len)` — an
  `Estimate` is duplicate-inflated by construction, but both labels are admissible
  bounds on iteration cost. v0 rule, **magnitude-first**: the smaller magnitude wins
  regardless of label; on a tie `Exact` wins (it cannot shrink); a full tie keeps the
  lowest subatom index (deterministic). The earlier label-first rule ("an Exact
  always displaces an Estimate") iterated a 500-key forced map while a 7-row
  param-filtered view sat unforced beside it — the measured balance wrong-cover.
- **`binary2fj` + conservative `factor()`** (§4.1): exactly per paper, over the DP
  planner's left-deep output.

**`ValidatedPlan` contents** (the witness type execution trusts): atom occurrences with
field→column maps; the node list with subatom partitions; per-node cover sets; per-
occurrence trie schemas derived per §3.3; per-node residual lists; per-atom filter
lists; the binding-slot layout (below); and the provably-distinct-bindings flag (below).
Validated once at construction; nothing downstream re-checks (post-mortem §38).

## Set semantics in the executor

Bindings are **VarId-indexed slot arrays**, written in place by the recursion and read
in place by sinks; plan variable order is therefore irrelevant to sinks.

Two facts identical on all *bound* variables produce the same binding; the solution is a
**set** of bindings, so duplicates must collapse before folding:

- The **projection sink** dedups projected facts (its job anyway).
- The **aggregate sink** folds a binding only on first occurrence, using a seen-set of
  full binding tuples — the same arena-backed mechanism as projection dedup.
- **Elision optimization:** if every atom occurrence's bound fields cover a unique
  constraint of its relation (typical for ledger queries that bind serial ids),
  distinct facts ⇒ distinct bindings, and the plan carries a proof flag that lets the
  aggregate sink skip the seen-set entirely. Provable at plan time from schema
  constraints — a representation-level fix, not a runtime branch per binding.

**Deviation D2 (set semantics — replaces the old D2):** the paper is bag-semantic
(leaves may carry multiplicity, output is a tuple stream). We: sets everywhere; leaves
are membership; binding dedup as above; and the executor may **skip a plan suffix after
the first witness** when (a) the active sink is the projection sink and (b) the suffix
binds only variables outside the projection set — the emitted fact cannot change, so
the recursion unwinds on the sink's first-emit signal. The skip is **never legal under
an aggregate sink** (any new bound variable multiplies the binding set the fold is
defined over). **Reverses if:** never — product semantics.

**Deviation D3 (sinks, not `output()`):** the executor emits complete bindings to a
private sink trait; projection-dedup and aggregate folds (Sum/Min/Max/Count grouped by
the non-aggregated finds, semantics normative in `20-query-ir.md`) are the two sinks.
Aggregation never materializes the join. Group maps live in sink arena state; aggregate
result types: Sum(I64)→I64, Sum(U64)→U64 (i128/u128 accumulators, one final range
check), Count→U64, Min/Max→input type. **Reverses if:** never structurally.

## Planner

**Statistics** (all real, nothing else exists): exact per-relation row counts
(maintained on write, stored in `S`); schema constraint knowledge (unique/FK); filter
survivor counts — *measured, not estimated*: filtered views are built before planning
completes for the atoms whose filter constants are all concrete, so the planner uses
the view's actual length. **Carve-out:** an atom whose filters involve params or
not-yet-interned literals cannot be measured at prepare time — it plans on the
selectivity ladder (`plan/selectivity.rs`): unique-exact counts, resident-image
distinct counts (peeked, never built), schema bounds (FK domains, enum variant
counts, bool), then the documented keep-fraction floors per predicate class. No NDV
fields, no histograms; the floors are the only constants and each is documented at
its definition.

**Join cardinality estimator, written down:** for `L ⋈ R` on join variables J —
- J covers a unique constraint of R (incl. serial auto-unique): estimate = |L| (FK walk;
  exact upper bound).
- J covers a unique constraint of L: estimate = min(est(P), |R|) — each R row matches
  at most one prefix row, and each prefix row matches at most |R|; the min is the
  correct bound (the earlier "|R|" alone was looser).
- Neither: estimate = |L| × |R| — **no estimate exists, so pessimism**, which pushes
  non-key joins last; that is the correct behavior, not a modeling failure.
|X| is the row count or the filtered-view survivor count.

**Search:** exhaustive DP over atom occurrences, **left-deep only**, minimizing the sum
of prefix estimates *including the base relation's rows* (counting the root iteration
breaks ties toward iterating the small side). The cap is 20 occurrences (a 2²⁰-state
table, ~32 MB transient plus a 16 MB per-mask prefix-variables memo — amended down
from an earlier 32 occurrences, whose table would not fit memory; the cap is enforced
at the validation boundary as a roster item, alongside the 128-distinct-variable
bitset cap). Then `binary2fj`, then `factor()`, then plan
validation into the witness.
**Decision: left-deep-only.** **Alternative:** bushy plans + materialized intermediates
(the paper decomposes bushy input into several left-deep plans and names
materialization its main bottleneck, §5/§6). **Why it lost:** materialized intermediates
have no home under the sink model and the allocation contract; left-deep + factoring
covers the design space the workload needs. **Reverses if:** a real query family shows
a bushy-only win that survives the benchmark protocol.

Plans **pin their statistics at prepare time** and are never invalidated by writes
(decision recorded in `20-query-ir.md`).

**Deviation D5 (no DuckDB):** the paper takes DuckDB's optimizer output; we grow the DP
above. **Reverses if:** never — no external SQL engine as infrastructure.

## Vectorized execution

**Deviation D4 (batching tuned to Apple Silicon):** the paper batches cover iteration
and probes siblings per batch (§4.3), hardware-generic. We: same algorithm, batch sized
to fill the M-series' ~28 MLP lanes — model: each probe is ~1–2 dependent loads, so
~28 lanes want ≥28 independent probes in flight and the batch amortizes bookkeeping
across several waves: starting range 64–256, measured (OPEN, README). **Probing is
two-phase**: phase one computes keys and hashes for the whole batch (pure ALU, no
memory dependence); phase two issues all bucket loads — independent chains the OoO
engine overlaps across the full MLP width. COLT's forced maps use **open addressing
with inline keys** (one probe ≈ one or two cache lines, no node chasing) and are kept
compact enough that a query's hot maps live in the 12–16 MB shared L2. **Batches are
processed branchlessly**: probe misses and residual failures become survivor
compaction (the scalar branchless cursor-write — NEON has no compress instruction;
that is SVE, which Apple Silicon lacks), never per-tuple conditional control flow —
on a >99%-accurate TAGE predictor, the data-dependent per-tuple branch is the only
misprediction source left, so we remove it representationally. **No indirect
dispatch exists in the hot path**: sinks, counters, and kernels are monomorphized
generics, never `dyn`. NEON (`cfg(aarch64)`, 128-bit = 2×u64) is confined to the
sanctioned kernel shapes (amended by the performance suite, docs/perf/):
fixed-width predicate scans, survivor compaction, fold/accumulate kernels
(Sum/Min/Max/Count over batch columns, strided or gathered — Sum semantics
unchanged: i128 accumulation, one range check at finalization), gather kernels
(position-indexed column reads), and software-prefetch passes (`prfm`) between
probe phase 1 and phase 2. Fold kernels are **scalar-ILP-first**: unrolled
multi-accumulator scalar loops are the default shape, and NEON earns its slot
per kernel only by measuring faster on the reference host — the deep-OoO scalar
path is the primary engine and simple dependency-free loops are preferred over
clever vectorization (`00-product.md` machine model; unsafe policy there too). Columns are 128-byte-aligned SoA
with staggered bases (`40-storage.md`). Scalar fallback everywhere, equal results by
test across batch sizes. **Vectorized execution is the default and only path** — a
scalar "mode" exists solely as the degenerate batch size where useful for testing; v5's
fake vectorized mode (post-mortem §31) is the cautionary tale. Honest caveat, stated:
deep in the plan the batch source is the current subtrie, whose fanout on FK walks is
often 1–10 — large batches are reliably available only at the root; cross-node-entry
batch accumulation is future work, not assumed. **Reverses if:** measured
equal-or-worse than scalar on the ledger suite after honest tuning.

**COLT force is single-pass with chunked child lists:** forcing pushes each offset into
its key's child list, chunked (64 offsets per arena chunk, chained by chunk — bounded
pointer-chase, independent loads within a chunk), rather than the paper's growable
per-key vectors or v5's two-pass contiguous layout (which decoded and hashed every row
twice — post-mortem §33). **Deviation:** the paper's leaves are plain vectors; ours are
chunked. **Reverses if:** a force+iterate microbenchmark shows two-pass-contiguous
winning end-to-end.

## The allocation contract

**A warm prepared-query execution performs zero heap allocations**, excluding a
caller-provided result buffer. All scratch — binding slots, probe keys, batch buffers,
COLT pools, filtered views, sink state (dedup sets, group maps) — is retained-capacity
pools owned by the `PreparedQuery` (index-addressed `Vec`s that reset without
freeing; the `Arena` type proper serves only the write delta). Retained scratch is
O(touched data + output) per prepared query and is documented as such (an app holding
N prepared queries retains N scratch sets); pools reach a fixpoint for a given data
generation.

**CI gate protocol (the definition of "steady state"):** single-threaded harness; the
prepared query executes N warmup runs with parameters drawn from a fixed set and no
intervening writes; then M measured runs over the same parameter set assert **zero**
allocator hits, arena growth included (post-warmup growth is a failure), result buffer
caller-provided. First-execution and post-commit rebuild allocations are sanctioned and
outside the measured window.

**Concurrency contract:** the engine owns zero threads (`00-product.md` doctrine).
Execution is single-threaded per query; `PreparedQuery` is `!Sync` and executes from
one thread at a time; arenas imply exclusive access. **Inter-query parallelism is free
and is the intended scaling axis**: reader threads each own their prepared queries and
pools and share immutable `Arc`'d images; nothing in the executor synchronizes (the
prepared query memoizes its views per (generation, resolved filters), so a warm
execution does not even touch the shared image-cache mutex).
Intra-query parallelism is a non-goal with a recorded reversal trigger
(`00-product.md`).

**Resource limits: none in v0, stated.** Dedup sets, group maps, and result buffers
grow with output; a pathological query can exceed the envelope and the OS is the
backstop. The scale axiom makes engine-imposed caps ceremony; revisit only on real pain.

## Deviation D1 — data source

*Paper:* relations are columnar in main memory. *We:* durable data lives in LMDB;
execution reads **full-width cached columnar images** built once per
`(relation, storage_tx_id)` and shared across read transactions (`40-storage.md`).
After warmup, execution runs in exactly the paper's environment. *Why:* LMDB is the
durable truth; at ≤1 GB the whole working set caches and the write design point
(≥100 reads/generation) amortizes builds to noise. **Reverses if:** traced rebuild cost
violates the latency budget despite the cache — then persist columns instead.

## Observability

**EXPLAIN exists from day one** and is the debugging story. Mechanism — a
representation, not a mode: the executor is generic over a `Counters` trait;
the normal path instantiates `NoopCounters` (zero-sized, compiled to nothing — no
runtime branch, no hot-loop cost), and the EXPLAIN entry point instantiates the
counting variant and **executes the query** (ANALYZE semantics), reporting the plan,
per-node estimated vs actual cardinalities, residual selectivity, and cover-choice
histograms (choices aggregated per node, not per entry). Output shape: OPEN. Release
builds contain no other instrumentation: no per-tuple labels, no always-on counters,
no diagnostics allocation anywhere in the join loops (post-mortem §32 is the reason
this paragraph exists).

## Perf-suite mechanisms (docs/architecture/30-execution.md, landed 2026-07-03)

Five decisions from the first benchmark report's evidence, enforced forever by
`crates/bumbledb-bench/src/tripwires.rs`:

- **Selection levels.** Every Eq-against-a-constant (literal or param — the
  same machine) lowers into `PlanOccurrence::selections` and becomes a
  prepended single-column COLT trie level, probed per execution with the
  resolved word (`Colt::select`). Force is O(view) once per generation, probes
  O(1) per param; views carry only residuals (ranges, Ne, `FieldsCompare`).
  **Alternative:** per-image secondary hash indexes — lost because the trie
  already *is* the index and selections compose with join levels for free.
  **Reverses if:** never; the per-param full scan it replaced was the
  measured 6.35× string-family loss.
- **The view-memo LRU.** Each occurrence memoizes `MEMO_SLOTS = 4`
  (generation, resolved residual filters) bindings — one active whose COLT
  the executor consumes, three parked slots (empty at prepare) and swapped
  in on hit. Prepare pins nothing: every COLT starts over `View::Unbound`
  (no image Arc — a prepared-but-never-executed query holds zero image
  memory), and the first execution binds via the ordinary miss path. Each
  bind first reaps parked entries below the requested generation (provably
  unhittable — their pools and image Arcs die at the first post-commit
  execution); parking prefers an empty slot, then evicts by LRU, and a
  stale or unbound active rebuilds in place so selection-only occurrences
  never park. Sound because generational immutability makes a view valid
  for its whole generation. Memory bound: four COLT high-waters per
  occurrence per prepared query, current-generation images only.
- **Magnitude-first cover choice.** `KeyCount` labels mean keys-exact vs
  positions-upper-bound; both are admissible iteration-cost bounds, so
  `better_cover` compares magnitudes and uses the label only on ties. The
  old "Exact displaces Estimate" rule iterated a 500-key forced map over a
  7-row view — the measured balance wrong-cover.
- **Dense map iteration and occupancy sizing.** Forced maps carry a dense
  occupied-slot list (iteration is O(keys), never O(capacity); the map
  `BatchToken` is a dense index) and size from
  `next_pow2(clamp(count/8, 16, 2·count))` with rehash-doubling at 75 % load
  (fresh slab ranges at the tail; old ranges reclaimed at reset — a ≤2×
  transient).
- **The finalize intern memo.** `ResolveMemo` maps `(intern word, tag)` to a
  byte range per finalize: each distinct string resolves through LMDB once
  and lands in the result buffer once (`dict_resolve` fires per miss, so the
  trace count is the distinct count). Cross-execution caching stays out — an
  unbounded-memory policy the measured problem never needed.

Prepare-time statistics live in `plan/selectivity.rs` (the distinct ladder:
unique-exact, resident-image exact via `ImageCache::peek` — prepare never
builds — schema bounds, documented floors) and the DP's join steps multiply
per-binding fanout `rows / distinct(join field)` with unique coverage pinning
fanout to 1; measured worst est/actual across the eight families fell from
114,679× to ≤ 3.3×.
