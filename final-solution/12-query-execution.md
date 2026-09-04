# 12 — One query meaning, from warm RAM to disk

Status: **proposed successor, not implemented or benchmarked**. The answer to a large database is not “allocate an image of every referenced relation,” and it is not “build a second analytical storage engine.” Use LMDB's ordered cursors as the complete baseline and keep optimized in-memory kernels as optional accelerators.

## 1. Small architecture

Four pieces are sufficient:

1. An immutable, schema-bound validated query template.
2. A snapshot-bound execution session with a work context.
3. A complete disk-native cursor executor.
4. One transient relation/map abstraction that starts in RAM and can use temporary LMDB.

Reuse that transient abstraction for projection distinctness, aggregate binding distinctness/group state, derived relations, recursive seen/frontier sets, and completed answers. Do not independently invent an external sorter, partitioned hash-join framework, analytical file format, persistent chunk-image store, spill scheduler, and a cache service.

This is a deliberate selection of less machinery. The old SIMD/Free Join kernels remain useful only where their inputs fit the available budget and where they beat the baseline on the intended workload. Their presence must not make the database's valid size equal to its maximum in-memory relation image.

## 2. Preserve the actual set language

Projection and union deduplicate full answer tuples. Negation tests existence in the same snapshot. Aggregation first identifies distinct full group bindings, then folds the requested argument of each binding: update a group accumulator only after the binding's insert-if-absent succeeds. Partial accumulator merges require disjoint binding partitions; exact addition is not idempotent and cannot itself deduplicate retry/union overlap. A group exists only if a binding derives it; empty global input emits no row. Recursion remains the supported finite active-domain linear fragment, evaluated to its least fixed point. No hidden output limit becomes relational truth.

Preserve scalar and temporal type distinctions, closed relations, keys, containment, capacity laws, and the existing useful interval operators. Arithmetic-producing expressions from [11](11-floats.md) remain at the nonrecursive output boundary. Do not expand the query language merely to populate a capability matrix.

Integer sums should also have order-independent failure semantics: for at most `u64::MAX` contributing bindings, accumulate `I64` in exact `i128` and `U64` in exact `u128`, then check the declared output range once. Count overflow is explicit. This avoids a planner-dependent intermediate overflow for a final sum that fits. Float sum/mean use their exact accumulator, not native repeated addition. This changes the old sequential-overflow contract and requires its own fixtures/proof update.

Errors belong to the denotation too. A rewrite may not manufacture a cast/overflow error on a binding that the reference semantics filters out, nor suppress an error on a surviving binding. Keep partial arithmetic out of early relational filters in the minimal 1.0 fragment; typed comparisons and membership have total predicate semantics. Any later extension must specify evaluation/error behavior before optimizing it.

## 3. The complete baseline uses cursors

For each positive atom, enumerate a bounded LMDB cursor or use an available key/range probe under the current bound variables. A depth-first index-nested-loop join maintains only the current binding stack and cursor state. Apply bound comparisons and negative existence probes as soon as their variables are available. Stream surviving bindings to the transient sink.

This can be slower than a good in-memory Free Join, especially for an unindexed Cartesian product. It nevertheless has three important properties: its memory use does not require full relations; it is a straightforward independent implementation to compare with optimized plans; and every accepted query has a correct path when an in-memory accelerator does not fit.

Do not hold a variable-width determinant in an LMDB key beyond its supported key size. For oversized text/tuple determinants, hash to candidates and compare the full canonical value, or scan if order is required. Never compare text intern numbers as though they were lexical text. Exact byte comparisons can borrow/chunk through mapped row values instead of allocating an entire candidate bucket.

The planner may choose a poor cursor order; this affects work, not meaning. Explain must show estimated versus observed visits, indexes used, missing-index scans, transient storage mode, effective budget, and why a requested acceleration was not used. Statistics guide choices; stale statistics cannot authorize an invalid plan.

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

No supported operator may secretly require a full relation to fit RAM. Each row below is a **1.0 acceptance obligation**.

| Operator | Complete bounded-memory path | Optional warm acceleration | Real refusal condition |
| --- | --- | --- | --- |
| Scan / equality / range | LMDB cursor; exact collision candidate scan for hashed keys | Bounded decoded batch/SIMD predicate | I/O/corruption, deadline/work budget |
| Positive join | Index-nested-loop/cursor join | Existing Free Join/hash/columnar kernel only after input reservation | Actual work/deadline exhaustion, not database size |
| Negation / containment probe | Snapshot key/existence lookup or bounded scan | Cached bounded lookup set | Same resource/error policy |
| Union / projection distinct | `ScratchRelation` exact set | Small RAM set | Scratch disk/quota/address-space failure |
| Grouped count / integer sum / float sum/mean / min/max | Scratch group state plus exact distinct binding set when required | Budgeted group cache and exact accumulator merging | Group/count representation overflow, disk/work failure |
| Temporal coalescing / pack | Scratch endpoint-ordered runs; cursor sweep per group | Small in-RAM sorted endpoints | Disk/work failure; invalid/ray measure remains semantic error |
| Nonrecursive derived relation | Scratch output reused as an ordinary relation cursor | RAM relation when it fits | Actual resource failure |
| Linear recursion | Scratch `seen`, `frontier`, and next-frontier; semi-naive rounds | RAM sets for small graphs | Deadline/work/explicit request round budget, scratch failure |
| Complete result | Private RAM or LMDB-backed sealed result | Owned compact vectors | Actual output/scratch policy, not fixed row count |
| Explain / prepare | Compact typed IR, bounded planning | Cached schema template/statistics | Query grammar/planning budget, not relation size |

Queries returning a tiny set after a massive join still consume work; an output-row cap cannot protect them. Conversely a million-row result can be legitimate when the caller provides disk/output capacity. Host-selected budgets and real device failures are honest reasons to refuse; an implicit 32 GiB database cutoff is not.

No general user-visible `ORDER BY`, external full-text collation sort, or arbitrary recursion is added by this table. A future ordered-answer feature must have its own exact comparator and disk path before acceptance.

## 6. RAM acceleration is optional and bounded

Do not build a persistent chunk-image subsystem to repair the current full-copy image behavior. Start by decoding bounded batches from LMDB cursors. A warm small relation may have an ephemeral columnar image if its entire retained capacity is reserved; a large relation uses cursor batches directly. Images are evictable accelerators, not obligatory state.

Cache keys include schema identity, native environment identity, snapshot/generation, and relation identity as appropriate. A schema-level template is immutable and shareable across tenants with the same schema; its mutable indexes, resolved text, and relation images are not. Do not weaken the foreign-catalog guard just to share plans.

On a write, invalidate affected small images or reuse only a demonstrated safe immutable version. Old snapshots may retain their old versions, all charged to their owners. No append operation is required to copy a huge prefix merely to make the next read possible. A memory-pressure trim can make the next query allocate or use disk; zero allocation is a measured warmed regime, not a global semantic law.

At plan selection, an optimization obtains the memory it needs or uses the baseline. If actual cardinality exceeds its reservation, restart from the same pinned snapshot using the cursor path or transfer its private sink into scratch using an explicitly tested transition. For 1.0, restarting a private query attempt is simpler than arbitrary mid-operator state migration; the externally observable result remains unpublished. Bound retries to one fallback, never an endless replan loop.

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

Charge owned allocation capacity **before growth**. Count decoded batches, parameter sets, normalization/planning state, hash/group buffers, exact float accumulators, output text, retained caches, and transfer overlap. Check scratch file growth before/after bounded transactions and handle map-full/disk-full without overwriting authoritative files. Captured caller byte arrays become owned immutable values before asynchronous work.

Do not call a `Vec::len` sum a hard RSS bound. Allocator overhead, LMDB mapped page residency, operating-system page cache, stacks, and foreign runtimes are not identical to engine-owned bytes. Expose both logical resource charges and observed process/disk metrics. A hosted deployment requiring a hard process-memory isolation boundary uses an OS-supported process/container limit with reserved headroom; core budgeting prevents avoidable growth but cannot override the operating system's allocation/fault behavior.

Cancellation is checked at bounded CPU-work quanta and every cursor/batch/network boundary. Publish the maximum unpolled work quantum and verify it with counters. A blocking filesystem page fault cannot honestly be given a universal millisecond completion guarantee by a user-space token. Hosted execution belongs on bounded workers so such waits do not freeze a Node event loop or all other tenants.

The same context reaches prepare/bind, image creation, nested scans, derived relations, recursive rounds, aggregate finalization, answer encoding, and write admission. A timeout wrapper around a synchronous native call does not cancel that call. Caller cancellation after a published log decision also does not roll back that decision; query cancellation and publication certainty remain separate contracts.

Arbitrary DNF expansion is not a prerequisite for execution. Keep the Boolean query structure compact, or lazily enumerate a bounded branch at a time with exact union deduplication. Charge planning and branch enumeration. Existing flattening/grounding optimizations may still run when their estimated expansion fits; unsupported syntax is rejected at validation, not discovered by running out of memory halfway through normalization.

## 8. Atomic answers and result lifetime

Execution builds a private result. Only after all relational work, aggregate finalization, value checks, and result storage succeed does it return a `CompleteResult` bound to the snapshot/template identity. No caller-owned `Answers` is gradually populated. Existing output reuse may be implemented as private capacity pooling, but failed work never becomes that caller's new logical result.

`CompleteResult` may own RAM or temporary LMDB. `collect(limit)` is an additional conversion that either returns a fully owned collection or an error without replacing the caller's previous collection. A caller does not have to collect the entire result into RAM to consume it: `into_cursor(page_bytes)` **consumes** the result owner and transfers its sealed backing storage to one explicitly chunked cursor with completion identity and terminal framing. The old result handle is spent. No clone/shared-cursor ownership subsystem is needed in 1.0; abandoning the cursor closes its own storage after active access drains.

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
| `Q-RECUR` | Finite recursion/resource distinction | Long narrow chain, shallow wide frontier, cycles, empty base, cancellation every round, spill during seen/frontier transition, no duplicate/missing derivations |
| `Q-GROUP` | Set aggregates and new sum law | Identity-bearing equal arguments, DNF/union duplicates, negative atoms, integer cancellation with intermediate overflow, float exact groups and empty-input rule |
| `Q-TEMPORAL` | Retained interval semantics | Rays/ceiling/fixed bounds, overlapping/adjacent/disjoint pack, multiple scalar groups; disk endpoint sweep equals naive point/interval model |
| `Q-LIFETIME` | PERF-002; SDK-007/013 | Held old snapshots/results plus repeated writes, trim, disposal and reopen; native owners/locks/scratch truly release |
| `Q-FAIR` | PERF-005; SDK-010/011 | Slow tenant and large query do not indefinitely block another tenant; queue cancellation and worker limits observed; event-loop delay measured |
| `Q-IR` | ASS-002 | Compact Boolean structure and bounded planning; optimized versus independent typed-tree evaluator compares both values and error outcomes |
| `Q-INJECT` | Resource/crash assurance | Disk-full, allocation refusal, cancelled spill copy, map resize blocked by reader, malformed scratch page, abrupt worker exit; authoritative facts unchanged |

Test small forced-spill fixtures on every ordinary CI run. Separately run controlled >RAM and >32 GiB qualification with actual disk allocation/resource measurements; sparse file length alone is not evidence that a larger working database was exercised. A resource test that merely catches an error after allocating all output fails the gate.

Performance reporting must separate warm fits-in-RAM, first read after write, cold pages, disk-native >RAM, and scratch-heavy regimes. Include p50/p95/p99, work/bytes, peak owned memory/RSS/disk, output size, and failures. Do not impose a fantasy constant-factor promise on disk paging. The promise is correct, bounded, usable execution with graceful slowdown and honest diagnostics—not that storage is as fast as RAM.
