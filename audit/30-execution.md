# Audit: 30-execution.md

Auditor scope: `docs/architecture/30-execution.md`, cross-checked against the Free Join
paper tex (02, 03, 04, 06), the other architecture docs, the Apple Silicon hardware
notes, and the v5 source at `465e3d4` (the cited `todo/` post-mortem is not in git —
see finding 15).

---

## 1. **Residual predicate execution is missing entirely** — [blocker]

The IR (`20-query-ir.md`) has `predicates: Vec<Comparison>` whose `lhs`/`rhs` are both
`Term`s, so comparisons between variables bound by *different atoms* (e.g.
`t1.time < t2.time` across two Posting atoms) are first-class. The paper only handles
single-atom selections: "We assume that the selections are pushed down to the base
tables" (§2, 02-background.tex). `40-storage.md` implements exactly that pushdown
("Filters push down into the image build"), which also only covers single-atom,
per-row-evaluable predicates. 30-execution never mentions predicates at all except NEON
"fixed-width predicate scans" — the image-build path. Cross-atom comparisons can only be
evaluated *during* the join, at the first node where both variables are bound, and
nothing specifies who places them (planner? `binary2fj`? a per-node residual list in
`ValidatedPlan`?), how they interact with vectorized batches (compact survivors after
probe?), or how a `Param`-valued filter behaves in a prepared query given that only
unfiltered images are cached (a per-execution filtered view: built where, from which
arena?). v5 had a whole `predicate.rs`/`PredicateMode`; the new doc dropped the topic
rather than deciding it.
**Question:** At which plan node, by what mechanism, and under whose responsibility is a
`Comparison` spanning multiple atoms evaluated, and how are pushed vs residual predicates
represented in the `ValidatedPlan`?

## 2. **Set semantics is asserted but never enforced in execution: projected images contain duplicate rows** — [blocker]

Relations are sets of *full* facts, but an image is "the set of requested columns of a
relation" (`40-storage.md`) built from a row-major F-scan — one row per fact. Two facts
agreeing on the requested fields yield duplicate rows in the image, and the doc itself
concedes this ("v5 compared **duplicate-inflated** vector lengths"). D2 says "Trie
leaves are membership, never counts," but no layer is assigned the job of collapsing
duplicates: not the image build, not `force()` (whose cost is stated as "proportional to
distinct keys" for the *map*, while the offset chains happily hold duplicate tuples),
not leaf iteration. If nobody dedups, the executor emits duplicate identical bindings;
the projection sink survives (it dedups anyway) but the aggregate sink folds twice —
Sum/Count are then wrong under the IR's own law ("Aggregation folds over **sets of
bindings**"). If some layer does dedup, its cost and allocation (a dedup structure per
trie level or per image) are unspecified and touch the allocation contract.
**Question:** At exactly what layer are duplicate rows of a column-subset image
collapsed, and what does that mechanism cost in `force()` time and arena space?

## 3. **D2's subtree skip is undefined and, as stated, unsound under aggregation sinks** — [blocker]

D2: "when the remaining plan suffix can only multiply witnesses of an already-emitted
projection (existential-only variables), the executor skips the subtree." Three holes.
(a) "Existential" in the IR means an *unbound field* — which never even reaches
execution, since images are built per `field_scope`; every plan node binds declared
variables, so "existential-only suffix" has no definition in the doc's own vocabulary.
It must mean something like "binds no variable that the active sink distinguishes," and
that is sink-relative. (b) Per `20-query-ir.md`, "Count counts the group's binding set,
exactly" — a suffix binding any new declared variable (even one not in `finds`)
multiplies the binding set, so skipping it changes Sum/Count inputs; the skip is sound
for the projection-dedup sink and unsound for the aggregate sink unless the fold input
is redefined as the projection onto (group key, `over` var) — which would itself break
ledger sums by collapsing equal amounts. The two candidate semantics give different
answers and neither doc picks one. (c) Mechanics: "skips the subtree" must mean
stop-after-first-witness (the *first* emission needs one complete traversal of the
suffix — D3 says sinks receive "complete bindings"), and it requires the executor to ask
the sink "already emitted?" mid-join — an interface D3's emit-only sink trait does not
have.
**Question:** Define "existential-only suffix" precisely (relative to which variable set
and which sink), and state whether the aggregate fold input is the full binding set or a
projection — and therefore for which sinks the skip is legal.

## 4. **The planner's cost model is not yet real — the same shape as the post-mortem's central failure** — [blocker]

The doc's stats are: exact row counts, "per-filter survivor estimates from cheap
heuristics," and constraint knowledge. Then: "Estimated cardinality is the entire cost
function." Estimated cardinality *of what*? The final output cardinality is
plan-invariant; a Selinger DP ranks plans by *intermediate* cardinalities, which require
join selectivity estimates. Unique/FK constraints do give exact bounds for key joins
(probably most ledger joins — worth saying!), but for a non-key equijoin the listed
statistics admit no estimate at all, and the doc's own rule — "a statistic that isn't
real doesn't exist in the struct" — forbids inventing NDV fields, while "cheap
heuristics" (which? magic selectivity constants are exactly what v5's fake model was
made of) is left as a blank check. Without the formula written down, this is a static
preference ranking waiting to happen again.
**Question:** Write the actual estimator: given only row counts, filter-survivor
estimates, and constraints, what is the estimated cardinality of `R ⋈ S` on (a) an FK,
(b) a unique key, (c) a plain field — and what exactly does the DP minimize (sum of
intermediates?)?

## 5. **Vectorized batching below the root node is unspecified; 64–256 may be unreachable there** — [design-gap]

The paper's vectorization (§4.3) batches only the cover iteration *within one node
entry*, then recurses per surviving tuple. Dynamic cover choice (§4.4, adopted) is made
per node entry, so at inner nodes the batch source is the current subtrie — whose key
count for ledger FK walks is typically 1–10, nowhere near "batches of 64–256" or 28
independent probes in flight. Only the root (iterating the base image) reliably yields
big batches. Filling MLP lanes at depth requires accumulating tuples *across* node
entries before probing — a real algorithmic extension the paper does not contain and the
doc does not mention. As written, D4's headline claim quietly holds only for the
outermost loop.
**Question:** Is batching root-only in practice (and is that acceptable for the
ledger suite), or is cross-entry batch accumulation intended — and if so, where is that
design?

## 6. **The chained-offsets COLT force is asserted, not designed** — [design-gap]

"children chain within the arena" contradicts the adopted definition two sections up ("a
node is a vector of offsets into the base columns") and deviates from the paper's own
single-pass force, which pushes offsets into growable per-key child *vectors*
(Fig. 8, 04-optimizations.tex) — yet it sits in the allocation section with no
`Deviation:` block. Iterating a chained child is dependent pointer-chasing — the exact
access pattern the hardware notes and D4 are built to avoid — and `iter_batch` over a
chain cannot issue independent loads. Chain granularity (per-offset next pointer vs
chunked/unrolled blocks) is unspecified, and "contiguity is not worth 2× work" is stated
without a measurement or reversal condition.
**Question:** Specify the chain representation (per-offset or chunked, and chunk size),
and name the benchmark that decides chained vs two-pass-contiguous plus the evidence
that would reverse the choice.

## 7. **The zero-allocation contract leaves its hard cases undefined** — [design-gap]

(a) "Steady state" is never defined: the first execution after any commit rebuilds
images (environment-scope allocation) and re-forces COLTs into arenas; is arena *growth*
on a larger-than-before result an allocation? (b) Dedup-set and aggregate group-map
state grows with output size — arena-backed but unbounded, and reusable capacity is only
"steady" if data stops growing. (c) "The result buffer is the single sanctioned
allocation site" — undefined for aggregate queries (is the group map itself the result
buffer or sink state?). (d) Cross-doc: `20-query-ir.md` says "statistics changes
invalidate plans," and exact row counts change on *every commit* — the naive reading is
that every write invalidates every prepared plan, so interleaved read/write workloads
re-plan (and allocate) constantly, gutting the contract on precisely the prepared-query
hot path; any invalidation threshold would be a statistic the "no fake fields" struct
doesn't have.
**Question:** Define steady state operationally (the exact scenario the CI counting
allocator runs), classify arena growth, and state the plan-invalidation policy under
continuous writes.

## 8. **The parallelism story is absent** — [design-gap]

The paper's system is single-threaded; nothing in 30-execution says so, and nothing
states bumbledb's own threading contract. Every mechanism described (backtracking trie
replacement, `&mut` arenas owned by the prepared query, mutable sink state) implies
single-threaded execution per query, and implies one prepared-query object cannot
execute concurrently from two threads — yet `40-storage.md` advertises many concurrent
readers sharing cached images, and `00-product.md` promises no async API but says
nothing about threads. An implementer must guess whether `PreparedQuery: Sync`, whether
arenas live in a per-thread transaction context, and whether intra-query parallelism is
a non-goal.
**Question:** State the decision: is execution single-threaded per query (matching the
paper), and what is the concurrency contract for a prepared query and its arenas?

## 9. **EXPLAIN's "actual cardinalities" contradict the no-instrumentation rule — an execution mode in disguise** — [design-gap]

EXPLAIN promises "per-node estimated vs **actual** cardinalities + cover choices";
two sentences later the release hot loop is guaranteed to have "no counters accumulating
on hot paths." Actual cardinalities *are* hot-path counters. The options are: a
debug-only EXPLAIN (contradicting "the debugging story for a database with no REPL" —
you would debug a different binary than the one misbehaving), or a counted executor
variant selected at runtime — which is exactly the Scalar/Vectorized-style mode enum the
doc condemns v5 for (v5's `ExecutionMode` in `query/cover.rs`). Also unspecified:
whether EXPLAIN executes the query (ANALYZE semantics), and what it reports for dynamic
cover choices, which are made per node *entry* — potentially millions of decisions.
**Question:** What representation yields actual per-node cardinalities without either a
runtime mode or counters in the release hot loop — and what exactly does EXPLAIN run and
print for per-entry cover choices?

## 10. **Plan space silently narrowed to one left-deep pipeline; the paper's bushy machinery dropped without a Deviation block** — [design-gap]

The paper accepts possibly-bushy optimizer plans and decomposes them into *several*
left-deep plans with materialized intermediates (§2.2; §4.1: "We decompose a bushy plan
into a set of left-deep plans"), and its discussion names intermediate materialization
as the major bottleneck. The doc describes "a cost-based **binary** left-deep plan →
`binary2fj`" as the paper's core, and its DP presumably searches left-deep only — but
never says so, never records the alternative (bushy + materialization; incompatible with
sinks and the allocation contract, which is a *good* reason), and thereby violates the
README's decision rule. An implementer writing the DP must guess whether bushy shapes
are in the search space.
**Question:** Is left-deep-only a decision? If so, add the alternative/why-it-lost
paragraph; if not, where do materialized intermediates live under the sink and
allocation designs?

## 11. **No resource limits anywhere: output-proportional state and the image cache are unbounded** — [design-gap]

Dedup sets, aggregate group maps, and result buffers grow with output size; on 10⁷ facts
a bad (or WCOJ-honest cyclic) query can produce outputs far beyond the "data fits in
RAM" envelope, and the doc gives no behavior: error, cap, or OOM. Cross-doc, D1's "the
whole working set caches" leans on `40-storage.md`'s cache where "stale-generation
images fall out by key" — a keyed map does not shed old entries by itself; that's an
eviction policy nobody wrote — and live-generation images are keyed by `field_scope`, so
one relation can be cached several times with no stated cap.
**Question:** Is there any memory ceiling (arena cap, result cap, cache eviction
policy), and what typed error does exceeding it produce?

## 12. **Point lookups by unique key — a headline workload item — have no decided execution path** — [design-gap]

`00-product.md` names "point lookups by unique key" as core workload; `50-validation.md`
benchmarks "membership point-lookups via unique keys." Under this doc, a point lookup is
a Free Join over images: first use per (relation, generation) pays a full O(n) image
build plus COLT force, then amortizes. Meanwhile `40-storage.md` maintains `U`
unique-guard entries in LMDB that answer the same lookup in one B-tree get — and
execution never consults them. The paper explicitly flags "our current optimizer does
not make use of existing indices" as a limitation (06-discussion.tex); the doc inherits
it silently for the one query family where it stings (interactive point read immediately
after a commit).
**Question:** Is "COLT is the only index" a recorded decision with the U-guard fast path
as the losing alternative, and what post-commit point-read latency does the ledger suite
actually demand?

## 13. **Sum's checked overflow makes "results identical by test" order-sensitive** — [clarification]

`20-query-ir.md`: "Sum uses checked overflow → error." Whether an intermediate i64
overflow occurs depends on fold order (e.g. `{i64::MAX, 1, -5}` errors in one order and
not another). Dynamic cover choice, batch size, and NEON survivor compaction all perturb
binding emission order, so scalar and vectorized runs — required by D4 and
`50-validation.md` to be "identical by test" — can legitimately disagree on *whether the
query errors*. A 128-bit accumulator (cannot overflow with < 2⁶⁴ i64 terms) makes Sum
order-free; alternatively define a canonical fold order.
**Question:** i128 accumulation or canonical order — which, and in which doc does the
decision live?

## 14. **The batch-size and alignment numbers lack a derivation, and the hardware reference contradicts itself** — [clarification]

D4: batches of 64–256 to "fill ~28 MLP lanes." Why 2–9× the lane count (probes
outstanding per lane? ROB occupancy across the probe sequence?) is never derived, so
"tuned by measurement" starts from an unexplained prior. Separately, the cited hardware
notes are internally inconsistent: Category 1 says "Cache lines are 128 bytes across
levels," while Category 5 cites three sources putting L1D at "64B lines, 256 sets" —
and D4's "columns 128-byte aligned" leans on the 128-byte figure.
**Question:** Which L1D line size is correct (resolve it in the reference doc), and what
is the one-sentence model connecting batch size to the ~28 lanes?

## 15. **The post-mortem the doc encodes is not where the docs say it is** — [clarification]

`README.md` and `40-storage.md` cite the review at "`todo/` at `1b65ae8^`", and this
doc's planner section rests on "the post-mortem's central engine finding." `git ls-tree
-r 1b65ae8^` contains no `todo/` — nor does any commit in the repo's history (the dump
was evidently untracked and destroyed by the wipe). I verified what I could against v5
source at `465e3d4` instead: the "fake vectorized mode" claim checks out (a runtime
`ExecutionMode::{Scalar, Vectorized}` enum in `query/cover.rs`; `runtime_vectorized.rs`
fills a batch then runs the scalar bind/probe path per tuple inside it), and cover
selection had a `KeyCountEstimate` type. But the 34-file review itself, including the
query-engine findings 30–40 these docs claim to encode, is unrecoverable from git.
**Question:** Does the `todo/` dump survive outside git (backup, another machine)? If
not, the docs should stop citing `1b65ae8^` for it and re-anchor the load-bearing claims
to the v5 source.

## 16. **What a sink actually receives is representation-unspecified** — [clarification]

D3 sinks receive "complete bindings," and aggregation groups "by the non-aggregated
finds" — but the doc never says a binding is a dense VarId-indexed slot array (the only
reading under which the plan's variable order vs group-key order is a non-issue), nor
where the aggregate sink's group map lives (sink arena?) and how its keys are formed
from the slots. One sentence pins it down; without it, an implementer could plausibly
stream bindings in plan-variable order and rediscover the mismatch as a bug.
**Question:** Confirm bindings are VarId-indexed slots read in place by the sink, and
that the group map is arena-backed sink state (interacting with finding 7's growth
question).

---

**Counts:** 4 blocker, 8 design-gap, 4 clarification.
