# Magic-number review — classification of every high-impact policy constant

Status: P14 F1 deliverable (gate `APP-MAGIC`; chapter 40 §5). Source locations
verified against the working tree on 2026-09-04 (branch `codex/bumbledb-1-0`,
preserved source `4b127782` + wave-A edits in flight). No constant was changed
by this review; changes to production constants are measured-change requests
to the owning packet, never P14 edits. Every sweep/probe named below executes
only in F3.

Classes:

- **derived bound** — arithmetic/structural consequence of a representation;
  needs a structural test/falsifier, not a benchmark.
- **backend limit** — a real bound of LMDB/OS/S3/hardware; read or qualify it
  from the actual backend, never restate it as a product cap.
- **measured tuning** — workload/hardware choice; needs an owner, unit,
  regime, bounded candidate sweep on each named target and a fallback.
- **resource policy** — deliberate product envelope; sized from the supported
  concurrency/ownership contract with a typed limit error.
- **instrumentation/calibration** — measurement machinery only; must never
  leak into language/semantics or certify a foreign machine.

| # | Source (verified) | Value | Class | Owner | Disposition and F3 obligation |
|---|---|---|---|---|---|
| 1 | `crates/bumbledb/src/storage/env.rs:103` `MAP_SIZE` | 32 GiB | arbitrary storage policy (rejected) | P02 | Delete as a ceiling: elastic 64-bit map, transaction-gated geometric resize, typed address/disk refusals. F3: G05/G06 resize schedules plus the `largefix` >40 GiB populated fixture prove the ceiling is gone. |
| 2 | `crates/bumbledb/src/storage/env.rs:105` `MAX_READERS` | 1024 | resource policy | P02/P06 | Size from the supported owner/concurrency envelope; account reader slots; typed limit error. Not a data-size invariant. F3: reader-exhaustion test returns the typed error. |
| 3 | `crates/bumbledb/src/storage/keys.rs:24` `MAX_KEY` | 511 B | backend limit | P02 | Read LMDB's real maximum at open (`mdb_env_get_maxkeysize`-equivalent through heed). Long logical keys get an exact bounded representation/fallback, no hidden truncation and no text-length ban. F3: long-key collision/range fallback tests (HASH-02 lane includes payload class 1/2 at/above this boundary). |
| 4 | `crates/bumbledb/src/storage/keys.rs:172` `MAX_DETERMINANT_WIDTH` | 496 = 511 − 15 | derived bound | P02 | Arithmetic from #3 minus the R-overhead; the existing test pins the subtraction. Follows #3 automatically; never independently tuned. |
| 5 | `crates/bumbledb/src/storage/keys.rs` 32-byte membership digest | 32 B | persisted fingerprint choice (rejected) | P01/P02, decision C12 (P00) | Replace with the selected 16-byte exact-checked fingerprint (`space::successor_layout`). Format freeze only after the F3 `hash-probe` lane (HASH-04) and SPACE-02 variant runs. |
| 6 | `crates/bumbledb/src/exec/run.rs:294` `BATCH` | 128 | measured tuning | P03 | Sweep realistic partial/full batches and cancellation quanta per target (M2 miss-parallelism evidence does not transfer to Graviton/x86 — bumblebench `m2max.mem.miss-lanes` is M2-scoped). F3: per-target sweep in the APP-TARGETS lane. |
| 7 | `crates/bumbledb/src/exec/run.rs:362` `PREFETCH_WIDTH_FLOOR` | 4 | measured tuning | P03 | Compare no-prefetch and exact current coverage under hit/miss/phase pressure; count redundant loads/instructions, not only elapsed time (bumblebench `m2max.mem.prefetch-regime`, `m2max.core.issue-queue-binds-first` are the regime evidence, M2-scoped). |
| 8 | `crates/bumbledb/src/exec/run.rs:207` `PHASE_NODE_CAP` | 8 | instrumentation | P03 | Deeper nodes enter the overflow bucket; never a query-depth limit. Structural test: a 9-deep plan still validates and attributes to overflow. |
| 9 | `crates/bumbledb/src/exec/wordmap.rs:17` `WINDOW` | 8 | derived bound | P03 | Eight ctrl bytes = one 64-bit SWAR word; retain with the mirror-tail/mask proof test. Not tunable. |
| 10 | `crates/bumbledb/src/exec/wordmap.rs:48` `LOAD_DEN` | 3 (33% load) | measured tuning | P03 | Comments cite 50/33/25% sweeps on the old fixture. Requalify on actual 128-bit ID/text/group key mixes with peak bytes counted. A low-load RAM table never explains LMDB disk bloat (that is SPACE-01's job). |
| 11 | `crates/bumbledb/src/exec/wordmap.rs:41` `HINT_CAP` | 1<<21 | resource policy | P03 | Clamps a presizing hint, not table cardinality. Make initial growth budget-aware (C02 WorkContext); structural test: exceeding the hint grows, never refuses. |
| 12 | `crates/bumbledb/src/exec/colt.rs` 8-key buckets / 0.4 load / child chunks 8→64 | mixed | derived bound (8-slot ops) + measured tuning (0.4 occupancy, chunk ladder) | P03 | Separate the structural eight-slot arithmetic (test) from occupancy/child-size sweeps (F3, per target, real key mixes). |
| 13 | `crates/bumbledb/src/exec/sink.rs:218` `DENSE_GROUPS_CAP` | 4096 | measured tuning (representation crossover) | P03 | Reserve full dense-table/accumulator cost including the exact-float state; sweep sparse-vs-dense crossover; falling back preserves answers (APP-NUMERIC cell charges accumulator bytes). |
| 14 | `crates/bumbledb/src/image.rs:23,25,196,198` `SET_STRIDE` 16384 / `LINE` 128 / `PAD_MIN_STRIDE` 64 KiB / `PAD_TOLERANCE` 384 | M2-derived placement policy | measured tuning | P03 | Do not apply blindly to all ARM64: page size, L1 line management and fetch granularity are distinct (bumblebench `m2max.cache.16k-pitch-aliasing` records the polarity error and correction — geometry rules need a falsifier and a structural test). Retain the conservative unpadded/aligned fallback; qualify per named target. |
| 15 | `crates/bumbledb/src/api/prepared.rs:712` `MEMO_SLOTS` | 4 | measured tuning (cache policy) | P03 | Measure tenant/parameter alternation and retained bytes, not only warmed hits. An entry count is not a memory budget: the APP-TENANTS churn cell reports retained capacity. |
| 16 | `crates/bumbledb-bench/src/clockproxy.rs:4` `CONTAMINATION_GHZ` | 3.2 (three-cycle multiply model) | calibration | P14 | Requalify or replace per target in F3. The M2-style instruction-cost formula must not certify Graviton/x86 GHz; APP-TARGETS carries a target-local calibration cell. Until then the proxy is Apple-Silicon-scoped. |
| 17 | `crates/bumbledb-log/src/writer/mod.rs:57,60,63` `LOSS_BOUND` 16 / `DRAIN_MAX_WRITES` 512 / `DRAIN_MAX_BYTES` 4 MiB | contention/admission policy of the retired writer | measured tuning (successor re-selection) | P04 | The old writer's transition semantics retire with it. Successor retry/queue/command budgets are selected from measured latency, fairness and actual sealed bytes (PERF-003 lane). Counts/bytes bound work; they are never S3/relational laws and cannot weaken receipt/durability guarantees. |
| 18 | `crates/bumbledb-log/src/writer/mod.rs:69,74` `CHECKPOINT_EVERY_SUM` 256 / `CHECKPOINT_EVERY_BYTES` 16 MiB | maintenance/transfer policy of the retired writer | measured tuning (successor re-selection) | P05 | Delete the braid-sum trigger with the braids. Qualify whole-checkpoint throughput, decision/receipt growth, buffer concurrency and replay headroom in the Maintenance regime. The historical 4096-decision/64-MiB tail example and the chapter 21 8-MiB chunk discussion are not new unexplained defaults. |
| 19 | `u32` image positions / trie references (image/COLT internals) | 2^32 rows per image | derived bound (fast representation) | P03 | Legal only when checked before construction and backed by the wider/disk path — never a global row cap. Structural test: overflow admission routes to the fallback, no wraparound. |
| 20 | Generation bit fields (owner/handle registries) | width-dependent | derived bound (no-alias invariant) | P06 | Needs a lifetime/reset no-alias invariant test (generation exhaustion case in RUN family), not a benchmark. |
| 21 | Schema tags, scalar widths, hash domain separators, interval endpoint semantics | versioned contracts | neither — format contract | P01 (C01), P00 (C12) | Excluded from all tuning sweeps by definition. Any change is a format-family change with goldens, never a knob. |
| 22 | `crates/bumbledb-bench` protocol constants (`Protocol::WARM` 32/256, `Protocol::COLD` 2/16, `QUANTUM_FLOOR_NS` 500, `P99_BUDGET_NS` 10 ms) | measurement protocol | instrumentation | P14 | Protocol constants shape evidence, not the product. The 10 ms p99 budget is the recorded historical target; the successor budget set is frozen per scorecard cell (`appperf::workloads`) before F3, and misses are reported as misses. |

Rules carried from chapter 40 §5, restated as obligations:

1. Every remaining tunable has one private owner, unit, regime/evidence
   reference, bounded candidate sweep and fallback. No public configuration
   field or registry per constant; no online autotuner.
2. Hardware evidence is machine-scoped: bumblebench facts are stamped
   M2 Max / rustc 1.96.0 / 2026-07-08 (`../bumblebench`, HEAD `30eb5eb2`) and
   two index entries are DRIFTED — nothing there is a universal law, and the
   database pins a different compiler.
3. A constant that survives F3 unswept is either reclassified as a derived
   bound/backend limit with its structural test, or its sweep is a named
   unresolved gate in the release ledger. Silence is not a classification.
