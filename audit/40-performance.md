# Performance: what is earned, what is at risk, what to measure next

## Bottom line

There is meaningful performance engineering here: fixed execution kernels, columnar relation images, Free Join/COLT, prepared-query reuse, append-aware image reuse, zero-copy carry for untouched relations, direct key probes, trace support, allocation gates, and comparative benchmarks with explicit unfavorable results.

The existing 28× headline is not a prediction for a per-tenant S3 application. The next performance work should target cold starts, write/read alternation, contention recovery, checkpointing, native memory, and host tail latency before more arithmetic-kernel tuning.

No new latency, throughput, cloud-cost, or hardware performance claim was measured in this audit. Tests and reliability probes ran on a shared machine, so they are not benchmark evidence.

## What the published measurements actually establish

`README.md:169` identifies a 2026-08-22 run at revision `01084e3e`, version 0.17.0, on an M2 Max. It describes roughly 253k ledger rows and 192k calendar rows; three runs; best medians; equivalent SQLite durability/indexing; 2,879 randomized verification cases; warm, engine-favorable workloads; and known p99 misses (`README.md:175`, `:198`, `:204`).

Those qualifications are good. Preserve them. This audit did not reproduce the raw run or validate every chart transformation. The benchmark sources have binary-bound verification stamps (`crates/bumbledb-bench/src/verify/stamp_value.rs:10`, `verify/tests.rs:35`) and retain separate oracle logic. That is stronger than an unqualified screenshot of a speed ratio.

For public comparisons, retain all repetitions and dispersion, not just the selected best median. A geometric mean over selected query families is a useful summary of those families, not a general database ranking. Show absolute latency and failure/time-out cases beside ratios.

## PERF-001 — One-row changes can still produce relation-sized read work

**Priority:** P2 workload risk; can become P1 for strict host latency. **Confidence:** confirmed mechanism, impact unbenchmarked here.

**Evidence:** dirty relations invalidate cached images in `crates/bumbledb/src/image/cache/advance.rs:23`; cache miss chooses full build or append in `image/cache/get_or_build.rs:99`; full build allocates and decodes a relation at `image/build.rs:230`. Append is better than full re-decode but still allocates new full-size slabs and copies the prefix (`image/build.rs:340`).

The current code is more sophisticated than the historical “every generation rebuilds everything” description: untouched relations can carry an `Arc`, append-only tails can reuse decoded prefix values, and direct key reads do not necessarily build images. Do not erase those improvements in the diagnosis.

Still, a delete/replacement followed by a query touching a large relation can pay a full relation rebuild. Append followed by query pays full prefix copy even when only a few rows are new. With one query after every write, the original read-heavy amortization can disappear.

**Measure:** alternating insert/read, replace/read and delete/read at realistic tenant sizes; report first read after commit separately from steady-state reads; include retained prior snapshots and concurrent readers. Record bytes decoded/copied and peak RSS, not only query duration.

**Possible directions after measurement:** immutable chunked images, finer invalidation, delta overlays, or smaller modeling partitions. Each affects ordinals, COLT references, old snapshots, and the no-allocation contract; do not apply one as a cosmetic optimization.

## PERF-002 — A tenant disk budget is not a process memory budget

**Priority:** P1 fleet-readiness gap. **Confidence:** confirmed architecture.

**Evidence:** `TenantOptions` explicitly budgets on-disk bytes and handle count (`crates/bumbledb-log/src/tenants.rs:97`). Images and prepared objects own additional state; `Answers` retains capacity (`crates/bumbledb/src/api/prepared.rs:183`); `ResolveMemo` keeps resolved text for the prepared query's lifetime (`:200`). SDK/FFI deterministic-close defects amplify the problem.

A rough resource model is:

`process footprint ≈ active native stores + resident image versions + query/COLT state + retained text + answer high-water + in-flight batch/checkpoint buffers + runtime overhead`.

This is a sizing model, not an exact additive RSS identity: mapped/shared pages and allocator behavior complicate accounting. It is enough to show why counting only `.mdb` bytes cannot enforce the host limit.

**Measure:** tenant open/close churn, a skewed hot set, one large result followed by tiny results, many prepared queries, and queries resolving previously unseen strings. Verify actual release after eviction.

**Direction:** bound admission before expensive work, account for native resources, provide deterministic close/reset, and consider memory-pressure trimming distinct from steady-state zero-allocation behavior.

## PERF-003 — Hosted commit cost is larger than the successful slot PUT

**Priority:** P2 planning requirement. **Confidence:** confirmed request path.

The normal path includes recording/marshalling, encoding, pending-file durability, local engine apply, conditional publication, and sidecar updates. ID block refill adds counter operations. A loss reopens/reseeds the tenant and rejudges; see `writer/discipline.rs:68`, `writer/loss.rs:47` and the replication report.

A useful accounting identity is:

`commit latency = queue + record/encode + local durable preparation + judgment/apply + object round trips + settlement`,

plus a variable recovery/retry term. Terms may overlap in some modes; measure the critical path rather than blindly summing separate timers.

**Measure:** 1/2/4/8 writers on one braid versus genuinely independent braids, same-key versus disjoint-key commands, ID refill boundaries, object latency distributions, packet-response loss, and checkpoints active/inactive. Include request count and bytes per accepted application command.

Do not infer current per-key conflict avoidance from the historical footprint design. Current Rust losses always re-establish the materialization.

## PERF-004 — Checkpoint duty can reread work and lose progress under sustained writes

**Priority:** P2 liveness/cost risk. **Confidence:** static mechanism; see detailed REP findings.

`Checkpointer::log_volume` fetches the tail objects to total their sizes (`crates/bumbledb-log/src/checkpointer.rs:202`). The resident snapshot hold requires the settled chain to remain unchanged across compaction, so ongoing writes can force retries. GC's swept-prefix accounting and checkpoint scratch lifecycle need correction before tuning them.

**Measure:** checkpoint completion time and bytes reread under continuous writes, with large tenant images. A cadence threshold is not a completion-time guarantee. Track peak disk overlap of source, compact copy, download and replacement materializations.

**Direction:** capture one coherent snapshot plus its vector once, let publication compare that snapshot safely with the current head, and keep durable progress for repetitive maintenance. Fix snapshot/version correctness first.

## PERF-005 — Synchronous embedding and shared request hosts need an explicit execution model

**Priority:** P1 for shared low-latency Node hosts; P2 for general Rust integration. **Confidence:** confirmed API shape, load impact unmeasured.

TypeScript engine work is synchronous native work. An async log API does not make its judgment/query phases nonblocking. A slow query can delay lease renewal and unrelated requests in the same event loop. Rust `S3Store` explicitly refuses use inside an async runtime context and owns a multithreaded runtime (`store/s3.rs:63`, `:76`, `:128`); integrations need an intentional blocking-worker arrangement.

**Direction:** either enforce a small bounded synchronous workload envelope or isolate database work onto bounded workers. Do not merely increase lease timeouts to disguise event-loop stalls. Reuse/clones of the shared S3 client/runtime should be preferred to per-tenant runtime construction.

**Measure:** p99/p99.9 request latency, event-loop delay, lease renewal lateness, queue wait, CPU fairness, and resource use under mixed tenants. Include noisy-neighbor tests, not just isolated query medians.

## Benchmark program that matches the intended product

| Family | Required regimes | Primary outputs |
| --- | --- | --- |
| Embedded steady reads | Warm prepared, varied parameters, old/new snapshots | p50/p95/p99/p99.9, allocations, RSS |
| Mutation/read alternation | Insert-only, replace, delete-heavy; several read/write ratios | First-read penalty, copy/decode bytes |
| Tenant activation | Cold process, warm object cache, long replay tail | Open time, requests, bytes, peak disk/RSS |
| Fleet churn | Many tenants, skewed hot set, pinned borrowers | Admission failures, release latency, tail isolation |
| Hosted commit | Standard S3 and any separately supported backend; 1–8 writers | Published latency, loss rate, requests per command |
| Maintenance | Concurrent checkpoint/GC and live workload | Time to checkpoint, stalls, retained history |
| Recovery | Pending, stale reader, expired lease, historical restore | RTO, recovered receipts, operator intervention |
| Real application | Booking/billing, graph/knowledge, event/ledger | User-visible workflow latency and correctness |

Treat these as multi-objective results. A change that cuts median CPU by 10% while doubling memory or failure-recovery time is not automatically an improvement for per-tenant hosting.
