# P14 measurement plan — baselines, method and the F3 probe order

Status: P14 F1 deliverable (gates APP-FAST/MUTATE/NUMERIC/LARGE/TENANTS/
TARGETS/METHOD/MAGIC, SPACE-01..02, HASH-01..04; audit PERF-003). Everything
here is a plan plus authored harnesses; **no measurement has been executed**.
Verification: NotRun. Harness code: `crates/bumbledb-bench/src/{appperf,
space, hashprobe, largefix}` plus the preserved `harness/`, `lanes/`,
`verify/` machinery.

## 1. Baseline provenance (historical, never current qualification)

- **Engine/SQLite campaign**: README-recorded 2026-08-22 shared-machine night
  at revision `01084e3e`, crate 0.17.0, Apple M2 Max; 253,264 ledger and
  192,369 calendar rows; three runs per durability class, best-median
  summaries; 2,879 randomized verification cases; known p99 misses (`spread`
  + three displaced probes) retained. The referenced `night-2026-08-22/` raw
  directory is **not present in this checkout** (re-verified 2026-09-04); all
  quoted numbers are README-recorded results, not reproduced artifacts.
- **Space baseline**: compacted bytes/row ~167 (ledger) and ~228 (calendar)
  versus indexed SQLite ~73 and ~93 — the 2.29×/2.45× gap SPACE-01 must
  attribute namespace by namespace.
- **CRUD/constraint baseline**: SQLite faster on 19/22 CRUD and 10/12
  constraint comparisons; failed durable key check 4.24 ms vs 7.7 µs (the
  persisted never-reuse allocator is deleted by the successor — its latency
  benefit still needs a new same-contract measurement, and hosted durable
  rejection receipts still require authoritative publication).
- **bumblebench**: sibling checkout `../bumblebench`, HEAD `30eb5eb2`
  (2026-07-08), report stamped M2 Max / Mac14,5 / macOS 15.7.7 / rustc
  1.96.0. Read-only input. Two index entries are DRIFTED; interleaved A/B is
  a recorded protocol; every fact is machine- and compiler-scoped.
- **F3 baseline comparison** uses an isolated baseline checkout at
  `01084e3e` built and run separately from the successor artifacts (same
  host, serialized), never a mixed working tree.

## 2. Method (APP-METHOD, binding)

1. Verified complete result sets/errors/final states against the independent
   oracle before timing; floats compare canonical bits; float intervals use
   the dense endpoint oracle. Shared inputs, never shared production
   algorithms.
2. Every result binds source/binary/toolchain/schema/dataset/config digests
   plus CPU model, OS, page size, memory/disk, Node version, enabled ISA and
   actual durability mode (extend the existing `verify` stamps and
   `report::Provenance`).
3. Performance measurements serialize per machine. Code/test authoring may
   parallelize; benchmark agents may not share the fabric.
4. Interleaved A/B arms with repeated baseline/baseline controls; rotated
   data/parameters; raw distributions and ambient/clock flags retained.
   Interleaving reduces some confounding; it cancels nothing asymmetric.
5. Durations/repetitions sized for timer resolution (`QUANTUM_FLOOR_NS`);
   denominators from actual work counters; hot-loop disassembly inspected
   when the hypothesis is about code shape; trace/alloc modes separate from
   uninstrumented timing (already enforced by `Modes`).
6. Warmed microbenchmarks separate from cold/post-fsync application behavior.
   No production spinning to warm away first-request cost.
7. Portable scalar/reference paths kept and forced; Apple Silicon first;
   Graviton/x86 qualified separately; no distributable artifact compiled for
   the build machine's CPU.
8. Accept/reject optimizations across the whole scorecard including space and
   tails; delete failed experiments, keep their evidence.
9. No-sync results reported separately, never as durable evidence; timeouts
   reported, never turned into ratios.

## 3. Native / bridge / Effect / whole-app decomposition

Format and merger: `appperf::layers` (one JSON schema both languages emit).
Rust-side emitters: `appperf::runner`. Node-side emitters are bench/test-only
harnesses owned by the runtime/SDK packets per chapter 40's routing (P06
bridge, P07 Effect, P13 app example); P14 supplies the schema, the summary
tool and the coverage-hole check (`native`→`bridge`→`effect` chains are
mandatory per op; `app` optional per op). Required Node-side columns: queue
wait, conversion time, event-loop delay (max over the cell), bytes copied,
GC count, external memory. Required cases: JIT warmup versus plateau, stable
versus polymorphic row shapes, bounded page pull, cancellation and noisy
neighbors during large ingestion and output conversion, scheduler-yield
proof that timers/sockets actually run. Attribution reads between per-layer
distributions, never per-sample subtraction across layers.

## 4. PERF-003 hosted-commit accounting

Accounting identity and sample shape: `appperf::hosted`. Cost per terminal
outcome (accepted / rejected / no-change / unknown) counts every request,
byte and retry on the path to the outcome — losing attempts, resolution
reads, catch-up and settlement included. Contention schedule: 1/2/4/8
writers × same/disjoint keys × shared/independent histories × checkpoint
on/off × loss injection on/off (`contention_schedule()`); the driver seam is
implemented over the successor log in F3 (real S3 for the qualification
cells; any emulator run is labeled emulator). Segment attribution can
overlap; end-to-end is measured on its own and `check_samples` rejects
summed-timer artifacts.

## 5. SPACE census and the matched SQLite comparison

`space::census` walks live entries per namespace over one coherent snapshot
(walker requested from P02 with the C04 cursor handoff), takes LMDB page
stats, file length and OS-allocated blocks as four distinct numbers;
`space::sqlite_stat` records SQLite page/freelist accounting and the actual
index roster (dbstat when the bundled build enables it, typed refusal
otherwise). Matched conditions: same generated rows, WAL + `synchronous=
FULL` + `fullfsync`, prepared statements, truncating checkpoint before stat
— the existing `lanes/storage.rs` protocol, extended rather than replaced.
Semantic/index differences (interval/capacity law structures SQLite does not
build; our per-fact membership index; SQLite's rowid alias and serial-type
integer compression) are reported as differences, not hidden. SPACE-02
variant matrix: `space::variants` (fingerprint 32→16; IDs 8→16 against the
audited tree and 28→16 against the superseded proposal, never netted; inline
text against both repeated-label and unique-churn populations), each variant
across disk raw/compacted, RSS, peak scratch, CRUD and warm/post-write/
cold/>RAM regimes.

## 6. Hash qualification (HASH-01..04) and long-key physical choices

Candidates and probe: `hashprobe::probe` — BLAKE3 full-32, truncated-16,
derive-key-16 (fingerprint shape with keyed-state init cost) and AEGIS-128L
MAC-16 with the TigerBeetle zero-key convention (bytes compared as bytes;
the little-endian `u128` convention is explicitly not copied). Inputs:
0/8/16/24/32/64/128 B, 1 KiB, 4 KiB, 8 MiB at three alignment offsets plus a
mixed short-fact stream; one-shot/streaming equivalence over every split
schedule gates all timing. KATs load from a hand-copied vector file (BLAKE3
upstream `test_vectors.json` at the locked crate revision; AEGIS vectors
from the pinned `aegis` crate / CFRG draft); a missing file is NotRun,
never a pass. Role inventory and sizing math: `hashprobe::{role_inventory,
sizing}` (birthday versus corruption versus deliberate-search models kept
apart; UUIDv4 = 122 bits). Collision forcing: `hashprobe::collision`
schedules insert/delete/contains/conflicting-pair/reopen/spill with
payload classes at and above the LMDB key bound, an independent BTree oracle
and a linear work bound — wired to the engine's test-only fingerprint
override in F3 (hook requested from P01/P02). Long-key physical choice
(exact bounded representation/fallback above `MAX_KEY`) is qualified inside
this lane: collision buckets must stay exact and bounded where the fact
cannot live inline. F3 order: equivalence → KAT → timing on each named
target → HASH-04 decision → **then** C12 format freeze; if the selected
algorithm changes, C12 consumers are notified, implementations/goldens
updated, and affected qualification reruns. Unearned candidates are removed,
not shipped as a plugin matrix.

## 7. Large-data gates (APP-LARGE / G05)

Two distinct fixtures (`largefix`): the >40 GiB **populated** store
(allocated-block enforcement rejects sparse impostors; boundary chunks
mutated and verified on both sides of the former 32 GiB ceiling; checkpoint/
restore under a recorded 8 GiB allowance) and the separately **enforced**
beyond-RAM workload (Linux cgroup v2 `memory.max`, data ≥ 4× the bound;
`RLIMIT_AS` explicitly forbidden). Streaming generator and chunk-checksum
oracle keep verification O(chunk). Both run on fitting hardware, never a
serverless scratch directory.

## 8. F3 execution order (P14 lanes, serialized per host)

1. Existing `verify`/`verify-store` stamps on the frozen corpora.
2. `app-perf` regimes + existing `bench`/`scenarios`/`crud`/`lawful`/
   `writes`/`curves`/`churn`/`heap` lanes (durable first, then no-sync,
   reported separately).
3. `storage` lane + `space` census (raw/compacted/after-churn/held-reader).
4. `hash-probe` per target, then the HASH-04/C12 decision.
5. `largefix` populated + beyond-RAM lanes on the qualified Linux runner.
6. PERF-003 hosted cells over the real backend.
7. Baseline checkout comparison and the final measured cost decisions
   (P00 approves; slower-than-0.x results investigated and either fixed or
   documented as tradeoffs before release).
