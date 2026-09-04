# 40 — A fast application database, measured where the application lives

Status: **documentation-only successor contract**. Existing source, the repository README/benchmark machinery, and the sibling `bumblebench` ledger were inspected. No new benchmark was run, no tuning constant was changed, and no successor speedup is claimed. Historical measurements below retain their machine, workload and provenance limits.

## 1. The product we are making fast

Bumbledb is a typed, set-semantic **application database per student, user or tenant**. A local app should get excellent Apple Silicon performance; hosted apps should get the same meanings on Graviton ARM and qualified x86-64 Node deployments. Many small databases, a skewed active set, short entity/graph/calendar reads, and ordinary writes matter at least as much as one large warm join. A Turso-style tenant model describes the product shape, not a claim that this LMDB/S3 design already has Turso's deployment or storage architecture.

Preserve the differentiators: direct typed ASTs rather than SQL parsing, final-state constraints, prepared reuse, direct key lookup, Free Join/COLT, efficient intervals, and measured vector/word kernels. Exact float **sum and mean stay**. `Interval<F64>` adds continuous ranges through the existing ordered-endpoint family. Generated migrations and app-owned 128-bit entity IDs simplify application integration; they do not justify weakening the read engine.

The work is not a new analytical warehouse, external-sort platform, global cache service or hardware autotuner. A bounded disk fallback lets a database outgrow RAM without a correctness cliff; it is not permission to turn fast application joins into nested-loop scans. Equally, Free Join is not permission to require an entire large relation to fit memory. Chapter [12](12-query-execution.md) specifies the exact common meaning and path selection.

## 2. What this repository already earned—and what it did not

The current [README](../README.md) describes the 2026-08-22 M2 Max run at revision `01084e3e`, crate 0.17.0, with 253,264 ledger and 192,369 calendar rows. It states shared-machine mode, boosted execution, three runs per durability class, best-median summaries, prepared/indexed/analyzed SQLite, matching durability and 2,879 randomized result checks. Preserve those qualifications. The literal `night-2026-08-22/` raw-report directory named there was not present at the inspected repository path; quoted numbers here are **README-recorded results**, not an independently reproduced campaign or a claim that every underlying artifact was verified.

| Recorded observation | Product implication, not a new performance claim |
| --- | --- |
| 28.0× geometric-mean speedup on 32 warm read queries | Free Join and the warm execution work are valuable. This is not a general database ranking or a hosted request-latency promise. |
| 22.2× on the 31 completed broader comparisons; two SQLite timeouts | Retain all families and timeouts. Do not fabricate ratios from a timeout or omit adverse cases. |
| SQLite faster on 19 of 22 CRUD comparisons; 1.85× geometric-mean advantage | Ordinary application mutations are a first-class optimization target, not an unimportant analytical side case. |
| SQLite faster on 10 of 12 constraint comparisons; failed durable key case 4.24 ms versus 7.7 µs | The old persisted never-reuse allocation policy was part of that path. Removing generated IDs deletes an obligation; its actual latency benefit still needs a new same-contract measurement. |
| Cold recursive fan-out 786 µs versus SQLite 22.7 µs; warm 791 ns versus 10.0 µs | Activation and first-read work can dominate the tiny hot query. A per-user database cannot be evaluated only after all users are warm. |
| `spread` and three largest displaced-data probes missed the old 10 ms p99 target | Keep explicit tail failures. A selected median does not close a tail-latency gate. |
| Compacted bytes/row 167 ledger and 228 calendar versus indexed SQLite 73 and 93 | Roughly 2.3–2.45× physical footprint needs a budget and explanation, not automatic acceptance as the price of Free Join. |

The source contains unusually useful measurement infrastructure: [verification stamps](../crates/bumbledb-bench/src/verify/stamp_value.rs) bind the binary, corpus, query families and verification configuration; the [harness](../crates/bumbledb-bench/src/harness/measure.rs) separates allocation windows from trace capture, accumulates actual work and keeps latency samples; [SQLite setup](../crates/bumbledb-bench/src/sqlite_run/open_for_bench.rs) requests and checks mapped-file coverage. Keep and extend these, rather than replacing them with a second benchmark framework.

The storage lane [measures](../crates/bumbledb-bench/src/lanes/storage.rs) engine raw and compacted files, indexed and table-only SQLite, truncates/checks WAL separately, and verifies row counts. File length is not resident RAM, allocated filesystem blocks, active pages, or a per-index byte attribution. The reported gap cannot be attributed to in-memory columnar images: [image.rs](../crates/bumbledb/src/image.rs) constructs vectors/`Arc`s from an LMDB scan; COLT is also transient memory. Persisted row/index/dictionary layout and page overhead need their own census. Chapter [41](41-storage-and-hashing.md) owns that decomposition and the local-fingerprint choice. Repeated text, wider application IDs, receipt rows, page utilization, churn and old readers must all be represented in the successor space report.

## 3. Keep the actual hot path, not a slogan about it

Read-only source inspection found these existing mechanisms:

| Mechanism and source | Successor obligation |
| --- | --- |
| [Direct probe classifier](../crates/bumbledb/src/exec/dispatch/classify.rs) recognizes suitable key/full-fact queries | Preserve the image-free entity lookup. A direct AST should reach this path without a generic join setup toll. |
| [COLT](../crates/bumbledb/src/exec/colt.rs) has lazy forcing, compact singleton children, indexed pools, dense iteration and bounded batches | Keep Free Join's ability to avoid a bad fixed binary-join intermediate; avoid eagerly building unused trie levels. |
| [COLT probes](../crates/bumbledb/src/exec/colt/probe.rs) specialize key widths 1–4, check tags before full keys, and retain a general path | Do not replace the measured scalar tag gate with unconditional NEON key loading merely because NEON wins in isolation. |
| [Probe pass](../crates/bumbledb/src/exec/run/probe_pass.rs) gathers/hashes in batches, filters residuals before sibling probes and uses disjoint buffer borrows | Preserve useful code shape while adding budgets. No per-row allocation, generic callback, or unmeasured dynamic dispatch should leak into this loop. |
| [Aggregate batch fold](../crates/bumbledb/src/exec/sink/aggregate/fold_batch.rs) separates constant group/argument paths and dense versus indexed gathers | Keep small-group application rollups cheap; add exact floats to the existing batch discipline. |
| [Dedup state](../crates/bumbledb/src/exec/sink.rs) and [constructors](../crates/bumbledb/src/exec/sink/aggregate/new.rs) already support a checked distinctness witness | Requalify that witness and elide work when justified. Set semantics do not require blindly materializing a second seen-set for bindings already proved distinct. |
| [Integer folds](../crates/bumbledb/src/exec/kernel/fold.rs) use vector carry counts for exact widened sums, plus dense/strided min/max | Preserve the exact integer kernels. A future F64 accumulator is a different numerical algorithm; measure it separately. |
| [Allen kernel](../crates/bumbledb/src/exec/kernel/allen.rs) consumes ordered endpoint words; [coverage/pack sweep](../crates/bumbledb/src/interval/sweep.rs) is generic over `Copy + Ord` | Float interval order keys can reuse these shapes after proof/bitwise qualification. Do not introduce an approximate temporal implementation. |
| [Image advance](../crates/bumbledb/src/image/cache/advance.rs) retains untouched state; [append](../crates/bumbledb/src/image/build.rs) copies the decoded prefix into newly allocated slabs | Preserve the good reuse, but measure first read after mutation. “Append-aware” does not mean O(delta) memory traffic. |

These are inspected implementation mechanisms, not a claim that all old behavior is correct. The audit counterexamples still stand. The replacement canonical values, exact collision handling and ownership boundaries must be carried through the fast paths, not bypassed to retain an old number.

## 4. What bumblebench changes about the engineering decisions

The sibling [ledger](../../bumblebench/FACTS.md), [charter](../../bumblebench/docs/00-charter.md), selected fact dossiers, [dedup investigation](../../bumblebench/docs/dedup_floor.md), corresponding antagonist source and [recorded report](../../bumblebench/REPORT.md) were inspected. The report is stamped **M2 Max / Mac14,5, macOS 15.7.7 / 24G720, rustc 1.96.0, 2026-07-08**. The database's inspected pin is a different compiler. Several dossiers retain provisional/contended-run notes; the index has two DRIFTED ambient-sensitive entries, and interleaved A/B is labeled a measured protocol record rather than a locally verified antagonist. None is silently promoted to a current universal hardware law.

| Ledger evidence | Consequence for Bumbledb |
| --- | --- |
| [`const-arity-tax`](../../bumblebench/facts/m2max.probe.const-arity-tax.md): dieted dynamic arity was 1.28–1.38× the const-K skeleton in its displaced 16 MB regime; the larger 1.7–2.1× claim concerns a different full runtime shape | Keep finite common-width specializations. The current source already has some; do not re-announce them as a new 2× discovery. Price the actual successor path and binary-size cost. |
| [`tag-gate-beats-sweep-in-situ`](../../bumblebench/facts/m2max.probe.tag-gate-beats-sweep-in-situ.md): an isolated 2.7× sweep win became recorded +4.4% triangle/+25% chain regressions in the engine | More loads can defeat fewer branches. The sibling's local antagonist validates the line-traffic mechanism; its historical engine verdict is cited provenance, not a new engine test here. |
| [`residency-is-interleaving`](../../bumblebench/facts/m2max.mem.residency-is-interleaving.md): the same map changes cost when foreign traffic lands between phases | A tenant's cache footprint alone is not its residency. Measure execution-phase and other-tenant interference, not just isolated map size. |
| [`sum-throughput`](../../bumblebench/facts/m2max.simd.sum-throughput.md): exact **u128 integer** NEON sums retained a roughly 1.9× cache-tier advantage in that experiment | Do not demote SIMD based on lane count. Also do not borrow this figure for exact F64 sum/mean: 34-limb accumulation and rational finalization were not the experiment. |
| [`16k-pitch-aliasing`](../../bumblebench/facts/m2max.cache.16k-pitch-aliasing.md) records the initial padding-rule polarity error and its correction | Geometry constants need a falsifier and structural test. Copying even a familiar “cache-line” rule can create the pathology it claims to prevent. |
| [`interleaved-ab`](../../bumblebench/facts/m2max.method.interleaved-ab.md), timer attribution and fsync/clock investigations | Use paired/interleaved comparisons, real work denominators, fresh data and clock/ambient evidence. A normalized number cannot generally correct shared-fabric interference. |

The inspected [dedup antagonist](../../bumblebench/src/bin/dedup_floor.rs) contains both the runtime-arity compare/hash loops and interleaved candidate runner. The [probe antagonist](../../bumblebench/src/bin/simd_probe.rs) explicitly contrasts tag-gated key loads with a four-vector eight-key sweep. These comparisons explain why their source shapes matter. They are not substitute performance tests for the new engine's inline text, 128-bit IDs, float domains or changed storage layout.

Keep bumblebench's adversarial habit: write the expected effect, the regime, the alternative explanation and the result that would kill the hypothesis **before** running the experiment. No “always prefetch,” “always vectorize,” “always branchless,” or “always use a cursor” rule survives this evidence.

## 5. Magic numbers: classify them before changing them

The following is a concrete high-impact source inventory, not a claim that every numeric literal in the repository was exhaustively reviewed. Each remaining tunable gets one private owner, unit, regime/evidence reference, bounded candidate sweep and fallback. Do not build a public configuration field or registry for every constant; do not turn this into an online autotuner.

| Current source/value | Class | Decision and acceptance obligation |
| --- | --- | --- |
| [storage/env.rs](../crates/bumbledb/src/storage/env.rs): 32 GiB map | Arbitrary storage policy | Remove as a ceiling. Elastic 64-bit maps with transaction-gated resize and actual address/disk refusals; no eager resident allocation. |
| Same file: `MAX_READERS=1024` | Resource policy | Size from the supported owner/concurrency envelope; account reader slots and return a typed limit. It is not a data-size invariant. |
| [storage/keys.rs](../crates/bumbledb/src/storage/keys.rs): 511-byte keys / 496-byte determinant boundary | Backend/encoding boundary | Read LMDB's real maximum at open. Long logical keys get an exact bounded representation/fallback rather than hidden truncation or a text-length ban. |
| Same file: 32-byte membership digest | Persisted fingerprint choice | Replace with chapter41's selected 16-byte local fingerprint plus full collision comparison; keep authoritative remote digests separate. Format freeze follows the targeted benchmark. |
| [exec/run.rs](../crates/bumbledb/src/exec/run.rs): `BATCH=128` | Throughput/latency tuning | Sweep realistic partial/full batches and cancellation quanta on each target. The M2's measured miss parallelism is not a Graviton/x86 constant. |
| Same file: `PREFETCH_WIDTH_FLOOR=4` | Workload/hardware tuning | Compare no-prefetch and exact current coverage under hit/miss/phase pressure; count redundant loads and instructions as well as elapsed time. |
| [wordmap.rs](../crates/bumbledb/src/exec/wordmap.rs): `WINDOW=8` | Representation geometry | Eight ctrl bytes form one 64-bit SWAR word; retain with its mirror-tail/mask proof. This is not an unexplained tuning number. |
| Same file: `LOAD_DEN=3` | Memory-versus-probe tuning | Current comments cite 50/33/25% sweeps. Requalify actual 128-bit/text/group key mixes and peak bytes. A low-load RAM table is not an explanation for LMDB disk bloat. |
| Same file: `HINT_CAP=1<<21` | Initial allocation policy | It clamps a presizing hint, not total table cardinality. Make initial growth budget-aware; do not turn the hint into a silent database limit. |
| [colt.rs](../crates/bumbledb/src/exec/colt.rs): 8-key buckets, 0.4 load, first child chunk 8, later chunk 64 | Mixed representation/tuning | Keep bucket arithmetic coherent; separately sweep occupancy and child-size distributions. Distinguish structural eight-slot operations from a workload-chosen occupancy threshold. |
| [sink.rs](../crates/bumbledb/src/exec/sink.rs): dense groups cap 4096 | Representation crossover | Reserve full dense-table/accumulator cost; compare sparse versus dense groups, especially exact float states. Falling back must preserve answers. |
| [image.rs](../crates/bumbledb/src/image.rs): stride 16,384; line 128; pad floor 64 KiB; tolerance 384 | M2-derived placement policy | Do not apply blindly to all ARM64. Page size, L1 line management and memory fetch granularity are distinct; benchmark supported targets, retain conservative unpadded/aligned fallback. |
| [api/prepared.rs](../crates/bumbledb/src/api/prepared.rs): four memo slots | Cache policy | Measure tenant/parameter alternation and retained bytes, not only the warmed hit path. A count of cache entries is not a memory budget. |
| [exec/run.rs](../crates/bumbledb/src/exec/run.rs): phase attribution cap 8 | Instrumentation | Current deeper nodes enter an overflow bucket; it is not a query-depth limit. Keep instrumentation limits out of language validation. |
| [clockproxy.rs](../crates/bumbledb-bench/src/clockproxy.rs): three-cycle multiply model / 3.2 GHz threshold | Measurement calibration | Requalify or replace per target. The current formula must not certify a Graviton or x86 machine's real GHz using an unverified M2-style instruction cost. |
| [log writer](../crates/bumbledb-log/src/writer/mod.rs): loss bound 16; drain at most 512 writes / 4 MiB | Contention/admission policy | Retire the old writer's transition semantics; select successor retry/queue/command budgets from latency, fairness and actual sealed bytes. Counts and bytes bound work; they are not S3 or relational laws and cannot weaken receipt/durability guarantees. |
| Same file: checkpoint after braid-sum 256 / 16 MiB; successor [21](21-storage-and-retention.md) initially discusses 8-MiB chunks | Maintenance/transfer policy | Delete the braid-sum trigger. Qualify whole-checkpoint throughput, decision/receipt growth, buffer concurrency and replay headroom. The earlier 4,096-decision/64-MiB tail example and chunk size must not become new unexplained universal defaults. |

Related fixed widths need the same honesty: `u32` image positions/trie references can remain compact **fast-representation** limits when checked before construction and backed by a wider/disk path; they must not reintroduce a global row cap. A generation bit field needs a no-alias lifetime/reset invariant, not a benchmark justification. Schema tags, widths, hash domain separators and interval endpoint semantics are versioned contracts, not candidates for an ad hoc runtime tuning sweep.

## 6. A small application scorecard that cannot hide the weak parts

Freeze representative schemas, populations, parameter schedules and pass criteria before the first replacement hot path lands. These are benchmark inputs, not tenant-size product caps. Use actual encoded bytes and identity widths, not an old all-u64 fixture that omits the new layout's cost.

| Application family | Work that must be measured |
| --- | --- |
| Student learning state | Keyed profile/attempt lookup; assignment → attempt → mastery joins; small per-course count/sum/mean; one attempt insert followed immediately by refreshed results |
| Personal/tenant scheduling | Point membership, overlap/exclusion, coverage and packed availability; integer time plus separate continuous F64 range fixtures; accept/reject a booking and read its consequences |
| Application graph | Neighborhood, two-hop paths, mutual edges, cyclic/triangle joins and bounded linear reachability; varied selectivity and frontier width/depth |
| Ordinary object CRUD | Small insert, keyed replacement, deletion, batch write, read/modify/write condition, no-change and rejected update; app-owned 16-byte IDs chosen before sealing |
| Text-rich personal data | Repeated short labels and unique text churn; inline payload size, exact key probes, long-key collisions/range fallback, delete/export/reopen |
| Tenant fleet | Many unopened/idle databases, skewed hot users, sequential activation, prepared reuse, pressure eviction and actual native release; concurrent small queries beside one large query |

Each family needs warm reuse, first execution after open, first read after insert/replace/delete, and memory-pressure/displacement variants. Supported >RAM queries need their own correctness/resource lane, not an analytical headline. Retain exact float single-group/high-cardinality and simultaneous sum+mean cases. Retain interval adjacency, rays, dense float gaps, unbounded measure and bounded length overflow cases. Recursion uses both long narrow chains and wide frontiers; “finite” is not a useful latency estimate.

For every cell report absolute p50/p95/p99, observed work and output count, preparation versus execution versus result delivery, bytes copied/decoded, allocations/retained capacity, peak RSS, actual disk consumption, and every failure/timeout. Report no-sync results separately and never use them as evidence for acknowledged durability. For hosted app calls, show queue, query/judgment, remote publication, cache catch-up and local commit distinctly; core microseconds do not erase network or scheduling costs.

Set product latency budgets from these operations, including x86 serverless cold-start and CPU-allocation behavior; do not invent universal nanosecond promises from the M2 run. A Vercel-class Node host without adequate local disk/lifetime is an explicitly unsupported materialization placement, not a benchmark success achieved by ignoring setup/recovery.

## 7. Method and release discipline

1. Verify complete result sets, errors and final states against the independent oracle before timing. Share schemas/data, not the production algorithm that computes expected outcomes. Floats compare canonical bits, not epsilon; float intervals use the dense endpoint oracle.
2. Bind the result to source, binary, dependency/toolchain, schema, dataset and configuration digests. Record CPU model/generation, OS, page size, memory/disk, Node version, enabled ISA and actual durability mode.
3. Serialize **performance measurements** per machine. Independent code/test work may run in parallel; a pile of benchmark agents sharing the fabric does not produce several independent latency measurements.
4. Interleave A/B arms and include repeated baseline/baseline controls. Rotate data/parameters to avoid measuring only a memorized branch pattern. Keep raw distributions and ambient/clock flags. Interleaving reduces some confounding; it does not mathematically cancel arbitrary asymmetric interference.
5. Use sufficient duration/repetitions for the timer resolution; verify per-operation denominators with actual counters. Inspect generated hot-loop code when the hypothesis is about vectorization, calls, compares, aliasing or unrolling. Run trace/allocation modes separately from uninstrumented timing.
6. Separate controlled warmed microbenchmarks from real cold/post-fsync application behavior. A spin-up calibration belongs to the microbenchmark protocol; **do not add spinning to production writes** to improve a chart or warm away the app's true first-request cost.
7. Preserve portable scalar/reference paths and test forced optimized/fallback dispatch. Optimize Apple Silicon first, qualify Graviton and x86 separately, and never ship a distributable artifact compiled blindly for the build machine's CPU.
8. Accept or reject a proposed optimization across the whole agreed application scorecard, including space and tails. Delete failed experiments; retain their evidence. If a simple shape is neutral, prefer it. If a fast shape earns a material win, preserve it even if a generic abstraction looks tidier.

## 8. New blocking performance gates

All gates below are **proposed and unrun**. Chapter70 owns their release status together with the safety/resource gates in10–13 and the hosted gates in20–33.

| Gate | Required evidence |
| --- | --- |
| `APP-FAST` | Same verified application queries on direct probe, preferred Free Join and forced cursor paths; successor warm scorecard retains the agreed fast-path/tail budgets instead of claiming victory over a deliberately slow fallback |
| `APP-MUTATE` | Insert/read, replace/read, delete/read, no-change and domain rejection; separate first-read rebuild/copy bytes and ID-allocation work, which must now be absent |
| `APP-NUMERIC` | Exact F64 sum and mean alone/together, single and many groups, distinct-witness and dedup-required inputs; dense float interval kernels/length errors; bits agree and full numerical state is charged |
| `APP-LARGE` | Supported >RAM/large-store execution with actual data, bounded RAM/scratch, checked fast-representation overflow and no hard 32 GiB limit; report fallback work and usability, not merely a successful open |
| `APP-TENANTS` | Per-user activation/churn and mixed-size noisy neighbors; queue/event-loop tail latency, native bytes released on eviction, filesystem space and storage-cost budget |
| `APP-TARGETS` | Fresh Rust/TypeScript artifacts on named Apple Silicon, Graviton ARM and x86-64 Node targets; identical answers, target-local calibration and explicit unsupported host-resource envelopes |
| `APP-METHOD` | Binary/data-bound verification, baseline controls, interleaved comparisons, real work counts, raw distributions, code-shape checks and truthful unrun/drift/timeout reporting |
| `APP-MAGIC` | Every high-impact constant above classified and owned; structural invariants tested, resource policy explicit, hardware crossovers measured, default/fallback correctness preserved; no public knob explosion |

The go/no-go question is not whether one join can beat SQLite spectacularly. It is whether the complete application database is fast, small enough, predictable and trustworthy on its promised hardware—and whether the implementation preserves that evidence as its representations change.
