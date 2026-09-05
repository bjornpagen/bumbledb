# 12 — Free Join first, one query meaning beyond RAM

Status: **proposed successor, not implemented or benchmarked**. Bumbledb is a high-performance application database, not an analytical warehouse. **Direct key probes and Free Join/COLT remain the preferred production paths**, with prepared reuse, measured batching and SIMD. Add complete bounded LMDB-cursor execution so memory pressure and a large tenant do not make valid data inaccessible. The fallback must not become an excuse to replace a measured fast path with universally slow execution.

## 1. Small architecture

Five focused pieces are sufficient:

1. An immutable, schema-bound validated query template.
2. A snapshot-bound execution session with a work context.
3. Existing direct-probe and Free Join/COLT execution, adapted to checked canonical values and bounded ownership.
4. A complete disk-native cursor fallback for the supported query language.
5. One transient relation/map abstraction that starts in RAM and can use temporary LMDB.

Reuse that transient abstraction for projection distinctness, aggregate binding distinctness/group state, derived relations, recursive seen/frontier sets, and completed answers. Its RAM implementation retains useful fixed-arity/small-group specialization; one logical abstraction does not mean a virtual-call or runtime-key loop per tuple. Do not independently invent an external sorter, partitioned hash-join framework, analytical file format, persistent chunk-image store, spill scheduler, and a cache service.

This retains the database's identity while removing a size accident. Free Join's ability to blend binary and worst-case-optimal behavior matters for application graphs and cyclic joins. Keep the useful existing kernels unless a same-workload, same-durability comparison justifies replacement. Their resident inputs must fit the admitted budget, but their presence must not make the database's valid size equal to one image's RAM or `u32` position capacity. A relation or result that does not fit a fast representation uses a complete bounded path, not truncation or a schema-size ban. [40](40-performance-contract.md) records the actual source/benchmark evidence and release scorecard.

Rust macros and TypeScript builders construct a **shared typed query AST directly**. Prepare validates and lowers that data; no SQL/Datalog string parser, query-language server, or hidden ORM interpreter is part of this work. Repeated executions change owned parameters, not source text that must be reparsed.

## 2. Preserve the actual set language

Projection and union deduplicate full answer tuples. Negation tests existence in the same snapshot. Aggregation first identifies distinct full group bindings, then folds the requested argument of each binding: update a group accumulator only after the binding's insert-if-absent succeeds, **or under a checked distinct-binding witness that proves the insert unnecessary**. Preserve and requalify the current `DistinctWitness`/elided-dedup path rather than mandating a seen-table for every aggregate. Partial accumulator merges require disjoint binding partitions; exact addition is not idempotent and cannot itself deduplicate retry/union overlap. A group exists only if a binding derives it; empty global input emits no row. Recursion remains the supported finite active-domain linear fragment, evaluated to its least fixed point. No hidden output limit becomes relational truth.

Preserve scalar and temporal type distinctions, closed relations, keys, containment, chapter 10's scalar-key grouped capacity laws, and the useful interval operators. `Interval<F64>` extends the ordered endpoint domain using [11](11-floats.md)'s dense semantics, not another execution family or pointwise occupancy constraint. Exact float sum and mean remain required. Make **nonrecursive relation composition uniform**, including aggregate and computed outputs, without adding a textual query language, arbitrary schema assertions or a general rule engine.

### Typed relations compose; names do not force storage

A nonrecursive relation expression has a checked output row type and an acyclic dependency graph. Stored/closed relations, parameters, and named derived relations use explicit typed references; derived names are query-local identities, never stored schema relations. The selected operators are the existing set projection, joins, total filters, union, negation/existence, interval operators, and grouped aggregates, plus the specified scalar computations at a stage's output. An aggregate-derived relation is a valid source for a later nonrecursive join/filter/projection/aggregate. This deliberately generalizes current projection-only `Interior` heads in [the IR](../crates/bumbledb/src/ir.rs) and [Lean syntax](../lean/Bumbledb/Query/Syntax.lean); it is not a claim those paths already support the extension.

Every aggregate stage has an explicit **input grain**: the distinct complete typed rows/bindings of its input relation expression, partitioned by its group-key values. A direct rule aggregate retains the complete rule-variable binding grain before grouping, including bound identity variables not returned in its answer. An explicit projection before grouping forms a new deduplicated relation and therefore changes that grain. For example, projecting attempts to `(student)` then counting counts distinct students; counting `(attempt_id, student)` bindings counts attempts. Giving either expression a name changes neither result. Union branches align to one typed row vocabulary and deduplicate there; syntactic DNF expansion cannot add bag multiplicity or silently discard binding variables.

Aggregation emits ordinary typed set rows with finalized scalars. A subsequent join can produce multiple distinct rows per aggregate group; another aggregate counts/folds those rows, not concealed bag weights or original source-row multiplicity. No hidden lineage column repairs an application projection that discarded its desired identity. Keep group formation unchanged: no input binding means no group and no aggregate answer, including global count/sum. Chapter 10's constraint evaluation over an existing parent with an empty child group is separately zero; do not synthesize a zero query row to imitate it.

A name is a compositional handle, **not** a mandatory full materialization or optimization fence. Inline, reuse, stream, or materialize a derived expression according to cost and ownership, using the same scratch abstraction when needed. Repeated references observe the same snapshot/parameters. Preserve stage meaning, binding distinctness, aggregate finalization and errors under each choice. Blocking work genuinely required by grouping or recursion still completes before its values are valid; not every named projection needs a temporary table. No intermediate `CompleteResult` owner must be exported merely because the application names a stage.

Integer sums should also have order-independent failure semantics: for at most `u64::MAX` contributing bindings, accumulate `I64` in exact `i128` and `U64` in exact `u128`, then check the declared output range once. Count overflow is explicit. This avoids a planner-dependent intermediate overflow for a final sum that fits. Float sum/mean use their exact accumulator, not native repeated addition. This changes the old sequential-overflow contract and requires its own fixtures/proof update.

Errors belong to the denotation too. A stage evaluates its input relation and total input predicates, then its declared output computations/aggregate finalization. Partial casts, arithmetic or interval length are not evaluated speculatively on bindings that stage's input predicates exclude. Once a referenced producer is required, a later consumer filter cannot hide an error from that producer: a stage producing an overflowing sum fails even if the consumer would discard that group. Unreferenced definitions are not evaluated. An optimizer may move a predicate or eliminate work only with both value and error-equivalence evidence, not only a successful-row algebraic identity.

Keep partial arithmetic out of relational predicate terms in the minimal fragment. Applications can name a computed stage and compare its canonical output values in a downstream stage; that does not erase the producer's error boundary. Ordinary comparisons and membership remain total predicates. Canonicalize each F64 expression node, and finalize each aggregate-derived scalar once at its own stage. Fusing a sum of rounded subgroup sums into an exact sum of all original bindings, or a mean of means into a global mean, is not generally legal. Optimizations can avoid storing intermediate rows without erasing these logical boundaries. Any execution error still prevents publication of the final answer.

### One positive linear fixpoint over a frozen finite domain

Retain one recursive relation, positive base/step inputs, a projection-only recursive head, and exactly one positive self occurrence per recursive step arm. No mutual recursion, nonlinear recursive arm, negation in recursive rules, aggregation inside the cycle, or arithmetic-produced recursive head is added. The nonrecursive relation graph remains acyclic except for this explicitly represented linear self dependency; do not infer arbitrary recursive components from a general program.

**Frozen finite nonrecursive predecessors may include aggregates and computed outputs.** Establish their set-or-error meaning against the pinned snapshot and parameters before starting the fixpoint; their meaning does not change between rounds. Frozen means logically fixed, not resident in RAM: use replayable bounded cursors or the same scratch storage where reuse requires it. Its finite active domain includes those predecessor values plus the admitted literals/parameters. Recursive heads only select existing values from that fixed domain, so the usual finite-domain induction and monotone least-fixpoint argument still apply. A computed/aggregate node depending on the recursive result cannot feed back into its base or step; validation checks that dependency, not whether an innocent expression was given a name. Nonrecursive stages may consume the finished recursive relation normally. This retains the small recursive operator while removing the old incidental projection-only-interior restriction.

## 3. Fast-path selection and the complete cursor fallback

First preserve the query classifier's exact keyed lookup where its proof applies: a primary-key read need not build any image or trie. For multi-relation application reads with admitted resident inputs, prefer the measured Free Join plan and lazy COLT construction. Retain survivor compaction, constant-group batch folds, small fixed-key-width specializations, and safe disjoint buffer reborrows. No rule says a scalar loop is always faster than NEON or that a NEON key sweep is always faster than the scalar tag-gated probe. The source and bumblebench contain counterexamples to both slogans.

For each positive atom, enumerate a bounded LMDB cursor or use an available key/range probe under the current bound variables. A depth-first index-nested-loop join maintains only the current binding stack and cursor state. Apply bound comparisons and negative existence probes as soon as their variables are available. Stream surviving bindings to the transient sink.

This fallback can be much slower than in-memory Free Join, especially for an unindexed Cartesian product. Its purpose is correctness and usable bounded operation outside the resident regime, plus a simple implementation to compare against. It is **not** the performance reference to which fast application queries may regress. Every supported query has a correct path when resident execution does not fit; actual work limits can still refuse an impractical join. Such refusal is explicit, never a partial answer. A production app schema/query should be tuned from observed access plans, not told that a full scan is acceptable merely because it terminates.

Do not hold a variable-width determinant in an LMDB key beyond its supported key size. For oversized text/tuple determinants, hash to candidates and compare the full canonical value, or scan if order is required. Never compare text intern numbers as though they were lexical text. Exact byte comparisons can borrow/chunk through mapped row values instead of allocating an entire candidate bucket.

The planner may choose a poor cursor order; this affects work, not meaning. Explain must show selected direct-probe/Free Join/cursor path, estimated versus observed visits, indexes used, missing-index scans, transient storage mode, effective budget, and why the preferred path was unavailable. Statistics guide choices; stale statistics cannot authorize an invalid plan. Register benchmark cells at these boundaries before implementation so changing the preferred path cannot hide a regression in one aggregate speedup.

## 4. One transient representation

Call the internal abstraction `ScratchRelation`; keep the public API free of this implementation name. It supports exact insert-if-absent, lookup/update by a checked key, bounded iteration, and disposal. Separate set membership from application-visible ordering: answers are sets, so physical hash-bucket traversal is not a promised result order.

Its two implementations are:

- A small fallibly allocated, budget-accounted in-memory table/vector representation.
- An execution-owned temporary LMDB environment using the same canonical values and exact collision-bucket logic as the core.

On crossing the RAM threshold, create the temporary environment, copy existing scratch entries in bounded batches, and switch ownership only after the copy completes. Reserve the transient overlap of the old table, transfer buffer, and LMDB metadata before starting. If scratch capacity cannot be obtained, fail with an explicit resource error and publish no answer. Once spilled, remain on disk for that execution; no oscillating adaptive tier manager is needed.

LMDB keys remain bounded physical keys; long logical keys live in values and are compared exactly within candidate buckets. A forced-constant-hash fixture must preserve correctness and bounded memory, albeit with poor runtime. Where temporal sweep needs scalar endpoint order, use bounded scalar endpoint keys directly. Where a generic canonical ordering is needed inside a collision bucket, use a bounded-memory repeated selection scan instead of demanding a new unbounded sort buffer.

One scratch environment per spilled execution is the simple default; allocate it lazily so warm small queries pay no file-open cost. Namespace scratch tables in a small fixed database roster. The environment is disposable and its writes need not claim authoritative durability; any no-sync setting is confined to this internal scratch capability, never reachable through the normal persistent-store constructor. Loss of scratch loses only the query attempt/result, not facts or receipts.

Scratch owns its directory and environment until completion/result disposal. Close native handles before unlinking its files. Failed execution drops the private builder and cleans only its own validated scratch path. Crash leftovers can be removed under the owning process/directory's exclusive lifetime lock. This is local temporary-resource hygiene, not another remote garbage-collection protocol.

## 5. Operator capability matrix

No supported operator may secretly require a full relation to fit RAM. Each row below is a **1.0 acceptance obligation**, not a demand to route every small application query through temporary files or to add an analytical storage system.

| Operator | Complete bounded-memory path | Optional warm acceleration | Real refusal condition |
| --- | --- | --- | --- |
| Scan / equality / range | LMDB cursor; exact collision candidate scan for hashed keys | Bounded decoded batch/SIMD predicate | I/O/corruption, deadline/work budget |
| Positive join | Index-nested-loop/cursor fallback | Preferred Free Join/COLT application path after input reservation | Actual work/deadline exhaustion, not database size |
| Negation / containment probe | Snapshot key/existence lookup or bounded scan | Cached bounded lookup set | Same resource/error policy |
| Union / projection distinct | `ScratchRelation` exact set | Small RAM set | Scratch disk/quota/address-space failure |
| Grouped count / integer sum / float sum/mean / min/max | Scratch group state plus exact distinct binding set when required | Constant-group batches, checked distinctness elision, budgeted group cache and exact accumulator merging | Group/count representation overflow, disk/work failure |
| Temporal coalescing / pack | Scratch endpoint-ordered runs; cursor sweep per group for integer and float intervals | Existing ordered-endpoint/SIMD kernels and small RAM endpoint sets | Disk/work failure; unbounded measure and bounded float-length overflow are distinct |
| Nonrecursive derived relation, including grouped/computed output | Bounded producer/consumer execution; scratch rows when actual reuse/blocking needs them; downstream typed cursor | Inlining, streamed fusion with stage laws, or budgeted RAM reuse | Actual resource failure; required producer errors |
| Positive linear recursion | Frozen finite predecessors; scratch `seen`, `frontier`, and next-frontier; projection-only semi-naive rounds | RAM sets for small graphs | Deadline/work/explicit request round budget, scratch failure; no aggregate/value-creation feedback |
| Complete result | Private RAM or LMDB-backed sealed result | Owned compact vectors | Actual output/scratch policy, not fixed row count |
| Explain / prepare | Compact typed IR, bounded planning | Cached schema template/statistics | Query grammar/planning budget, not relation size |

Queries returning a tiny set after a massive join still consume work; an output-row cap cannot protect them. Conversely a million-row result can be legitimate when the caller provides disk/output capacity. Host-selected budgets and real device failures are honest reasons to refuse; an implicit 32 GiB database cutoff is not.

No general user-visible `ORDER BY`, external full-text collation sort, or arbitrary recursion is added by this table. A future ordered-answer feature must have its own exact comparator and disk path before acceptance.

## 6. Resident performance is first-class and bounded

Do not build a persistent chunk-image subsystem to repair current full-copy behavior. Retain reusable columnar images and lazy COLT for the warm application regime, with complete retained capacity reserved before construction. Preserve zero-copy reuse for untouched relations and direct probes that bypass images. Large relations can use bounded cursor batches; bounded selected subsets may enter Free Join only with an explicit complete-subset plan witness. Images are evictable accelerators, not obligatory authoritative state.

Cache keys include schema identity, native environment identity, snapshot/generation, and relation identity as appropriate. A schema-level template is immutable and shareable across tenants with the same schema; its mutable indexes, resolved text, and relation images are not. Do not weaken the foreign-catalog guard just to share plans.

On a write, invalidate affected small images or reuse only a demonstrated safe immutable version. Old snapshots may retain their old versions, all charged to their owners. Benchmark insert/read, replace/read and delete/read alternation: the current append path still copies the decoded prefix into new full-size slabs, so calling it incremental does not prove a cheap first read. No append operation is required to copy a huge prefix merely to make the next read possible. A memory-pressure trim can make the next query allocate or use disk; zero allocation remains an important measured warmed regime, not a global semantic law that prohibits trimming.

At plan selection, the preferred resident path obtains its memory or uses the bounded fallback. If actual cardinality exceeds its reservation, restart from the same pinned snapshot using the cursor path or transfer its private sink into scratch using an explicitly tested transition. For 1.0, restarting a private query attempt is simpler than arbitrary mid-operator state migration; the externally observable result remains unpublished. Bound retries to one fallback, never an endless replan loop. Record the discarded work and fallback latency; a frequent restart is a planner/product defect to fix, not acceptable hidden overhead.

## 7. Work context and honest resource accounting

Illustrative API:

```rust
struct QueryTemplate { /* immutable validated schema-level plan */ }
struct ExecutionSession { /* owner-bound caches, revocable on close */ }
struct WorkContext {
    working_bytes: Budget,
    scratch_bytes: Budget,
    result_bytes: Budget,
    work_units: Budget,
    deadline: Option<Instant>,
    cancel: CancellationToken,
}

fn execute(
    snapshot: &OwnedSnapshot,
    template: &QueryTemplate,
    parameters: &CheckedParameters,
    session: &mut ExecutionSession,
    ctx: &mut WorkContext,
) -> Result<CompleteResult, QueryError>;
```

Charge owned allocation capacity **before growth**. Count decoded batches, parameter sets, normalization/planning state, hash/group buffers, exact float accumulators, output text, retained caches, and transfer overlap. Check scratch file growth before/after bounded transactions and handle map-full/disk-full without overwriting authoritative files. TypeScript captures caller bytes during bounded Effect ingestion; successful execution establishes acceptance before native query/admission consumes them. Effect construction is lazy and performs no bulk copy.

Do not call a `Vec::len` sum a hard RSS bound. Allocator overhead, LMDB mapped page residency, operating-system page cache, stacks, and foreign runtimes are not identical to engine-owned bytes. Expose both logical resource charges and observed process/disk metrics. A hosted deployment requiring a hard process-memory isolation boundary uses an OS-supported process/container limit with reserved headroom; core budgeting prevents avoidable growth but cannot override the operating system's allocation/fault behavior.

Cancellation is checked at bounded CPU-work quanta and every cursor/batch/network boundary. Publish the maximum unpolled work quantum and verify it with counters. A blocking filesystem page fault cannot honestly be given a universal millisecond completion guarantee by a user-space token. Hosted execution belongs on bounded workers so such waits do not freeze a Node event loop or all other tenants.

The same context reaches prepare/bind, image creation, nested scans, derived relations, recursive rounds, aggregate finalization, answer encoding, and write admission. A timeout wrapper around a synchronous native call does not cancel that call. Caller cancellation after a published log decision also does not roll back that decision; query cancellation and publication certainty remain separate contracts.

Arbitrary DNF expansion is not a prerequisite for execution. Keep the Boolean query structure compact, or lazily enumerate a bounded branch at a time with exact union deduplication. Charge planning, dependency validation, derived-stage reuse and branch enumeration. Existing flattening/grounding optimizations may still run when their estimated expansion fits. Normalize a finite set of harmless equivalent builder forms once; do not duplicate spelling bans across macros, TypeScript and wire readers. Unknown references, row-type mismatch, unsafe recursive dependencies and unsupported semantic operators still refuse at validation, not halfway through normalization. Canonical wire parsing remains strict.

## 8. Atomic answers and result lifetime

Execution builds a private result. Only after all relational work, aggregate finalization, value checks, and result storage succeed does it return a `CompleteResult` bound to the snapshot/template identity. No caller-owned `Answers` is gradually populated. Existing output reuse may be implemented as private capacity pooling, but failed work never becomes that caller's new logical result.

`CompleteResult` may own RAM or temporary LMDB. `collect(limit)` is an additional conversion that either returns a fully owned collection or an error without replacing the caller's previous collection. A caller does not have to collect the entire result into RAM to consume it: `into_cursor(page_bytes)` **consumes** the result owner and transfers its sealed backing storage to one explicitly chunked cursor with completion identity and terminal framing. The old result handle is spent. No clone/shared-cursor ownership subsystem is needed in 1.0; abandoning the cursor closes its own storage after active access drains.

This is paged **delivery after query completion**, not a claim of early-result execution streaming. Rust exposes the consuming cursor; TypeScript exposes only `CompleteResult.pages`, an Effect Stream that moves the backing into a private scoped cursor on its first execution. Early take/error/interruption closes it; a second run refuses. No public TS cursor/AsyncIterable twin. Chapter 35 specifies the exact lifetime and V8-aware batch/page boundary. Most application reads should produce a bounded task/dashboard/entity result via their typed predicates. Measure time to complete and time to first delivered page separately. A future lazy/ordered/top-k contract is a product decision, not something smuggled into an iterator name.

Distinguish query completion from later delivery failure. A disk failure or cancellation while transmitting an already sealed result can interrupt chunk delivery; the cursor reports the delivered prefix and lack of terminal completion. It must not label that prefix the complete set. A plain `execute_collect` retains the stronger all-or-error returned-value contract. No API promises to make arbitrary future storage/transport reads infallible.

Result disposal closes its scratch environment and releases its reserved resources. A user holding many results pays for many results; the pool cannot silently free an active result while exposing valid-looking handles. Owner closure/release checks follow the SDK lifecycle contract, not garbage-collector timing.

## 9. Audit closure and blocking tests

All gates below are **proposed, not run**. The old audit's passing test counts are not a pass for any of them.

| Gate | Finding/requirement | Acceptance property |
| --- | --- | --- |
| `Q-ATOMIC` | QRY-001 | Valid groups before overflow/decode/bind/foreign-plan errors produce no new published answer; success/error/success reuse behaves identically |
| `Q-BUDGET` | QRY-002/003 | Tiny byte/work limits stop before unreserved growth at every phase, including prepare and recursive base/round; effective limits settable from each public client |
| `Q-DISK` | >RAM mandate; PERF-001/002 | Every operator in the matrix succeeds on a fixture exceeding its allowed working RAM by a large factor, with enough scratch; compare complete answers to a smaller/reference equivalent |
| `Q-LARGE-STORE` | Old 32 GiB cap | A legitimately larger store opens/scans/joins with bounded working memory; map size is not charged as resident RAM; actual address/disk failures typed |
| `Q-COLLISION` | Exact set semantics | Force constant hashes for unequal long text/group/join/result keys; exact answers and bounded bucket memory |
| `Q-FALLBACK` | Optional kernel architecture | Force each optimized path, force no-cache cursor path, force RAM→LMDB scratch, and force optimization reservation failure; sets and errors agree |
| `Q-RECUR` | Positive linear recursion/resource distinction | Long narrow chain, shallow wide frontier, cycles, finite frozen aggregate/computed predecessors, empty base, cancellation every round, spill during seen/frontier transition; no duplicate/missing derivations; reject mutual/nonlinear/negative/aggregate or new-value feedback |
| `Q-GROUP` | Distinct input grain, composable aggregates and new sum law | Identity-bearing equal arguments, students-versus-attempts projection grain, DNF/typed-union duplicates, negative atoms, grouped-output joins and second aggregates, integer cancellation, float staged rounding and empty-input rule |
| `Q-TEMPORAL` | Generic exact interval semantics | Integer rays/ceiling/fixed bounds and dense float endpoints, overlapping/adjacent/disjoint pack, multiple scalar groups; disk/SIMD sweep equals the correct discrete or dense endpoint oracle |
| `Q-LIFETIME` | PERF-002; SDK-007/013 | Held old snapshots/results plus repeated writes, trim, disposal and reopen; native owners/locks/scratch truly release |
| `Q-FAIR` | PERF-005; SDK-010/011 | Slow tenant and large query do not indefinitely block another tenant; queue cancellation and worker limits observed; event-loop delay measured |
| `Q-IR` | ASS-002; uniform nonrecursive composition | Compact Boolean/dependency structure and bounded planning; inline/stream/reuse/materialize named projection and aggregate stages; compare values/errors with independent staged evaluator; normalize harmless forms while preserving typed restrictions and producer error boundaries |
| `Q-INJECT` | Resource/crash assurance | Disk-full, allocation refusal, cancelled spill copy, map resize blocked by reader, malformed scratch page, abrupt worker exit; authoritative facts unchanged |

Test small forced-spill fixtures on every ordinary CI run. Separately run controlled >RAM and >32 GiB qualification with actual disk allocation/resource measurements; sparse file length alone is not evidence that a larger working database was exercised. A resource test that merely catches an error after allocating all output fails the gate.

Performance reporting must separate warm fits-in-RAM, first read after write, cold pages, disk-native >RAM, and scratch-heavy regimes. Include p50/p95/p99, work/bytes, peak owned memory/RSS/disk, output size, and failures. Do not impose a fantasy constant-factor promise on disk paging. The promise is correct, bounded, usable execution with graceful slowdown and honest diagnostics—not that storage is as fast as RAM.
