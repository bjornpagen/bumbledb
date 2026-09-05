# Performance and storage contract (permanent)

This is an application database, not an analytics engine. The performance
target is a warm per-user database with narrow relations, selective
lookups, short mutations and frequent composed joins. A cold or oversized
tenant remains correct and usable with slower I/O.

This page is the **meaning home**. L20 owns the executable input plan in
`crates/bumbledb-bench/src/appperf/plan.rs`
(`appperf::plan::{render,l21_semantic_checks,script_steps,hardware_prerequisites}`)
and the frozen 13-cell matrix in `appperf::workloads::scorecard`. Those
symbols are the bound measurement inputs; they do not replace the
meanings below.

Historical README numbers are attribution, not successor results. Timing
comparisons belong in explicit G15 qualification on a quiet host after
writer freeze. **Do not start G15 until writers are frozen and the
post-retirement candidate exists.**
Do not assert a universal numerical speed budget without a measured
baseline. Verification of this tree: **NotRun**.

## Physical representation

Keep LMDB, exact full-tuple membership, local row-ID indirection and
compiled access paths. Keep BLAKE3. Use 16-byte fingerprints only to
narrow an exact comparison; use 32-byte commitments where bytes must
authenticate an object/history identity without retrieving another full
preimage.

| Role | Width | Reason |
| --- | ---: | --- |
| Physical row ID | 8 bytes | Local row indirection, not a content hash or portable application identity |
| Application Id128 | 16 bytes | Ordinary application data, chosen once before a command is sealed |
| Membership / wide determinant fingerprint | 16 bytes | Candidate routing; canonical bytes still decide equality |
| Exact determinant | Checked encoded width | Avoid hashing small fixed domains; ordering is explicit |
| Schema, command, decision, object and migration commitments | 32 bytes | Authoritative content bindings, with distinct domains |
| S3 ETag / conditional version | Opaque provider value | A CAS witness, not our content digest |

Live raw key sizes from the current layout (L20 `appperf::constants`,
rechecked 2026-09-05): row **13**, membership **29**, exact u64
determinant **19**, Id128 determinant **27**. Determinant overhead is
tag + `ProjectionId` + row (11), not a declaration-order statement
number. Recalculate if
`crates/bumbledb/src/storage/store/keys.rs` or
`crates/bumbledb/src/schema/compiled.rs` changes. Fits-the-backend is
eligibility, not a cost model.

Select exact scalar determinants up to 16 encoded bytes, subject to the
complete physical key bound; use fingerprint buckets otherwise. Share
identical access paths only when projection, filter, domain, order and
candidate-state semantics match. A key law's candidate index must
represent multiple conflicting rows until judgment.

Removing the full-row membership index is not selected by default. No new
cluster-by-primary-key storage family in this pass. A reserved virtual
map size is neither allocated file size nor resident memory. Distinguish
map size, file size, resident cache, plans/results and work.

## Collision and hashing

For exact-checked local fingerprints, collisions affect work, not truth.
Test with a constant hash to make that contract real. A malicious
collision-heavy input still consumes budget and may be refused; do not
quietly truncate a bucket.

Selected release default is BLAKE3. `HASH-04` is a documented algorithm
decision plus actual-input BLAKE3/layout qualification. AEGIS comparison
is optional (`hash-aegis-optional` in `script_steps`); missing KAT is
**NotRun**, never a fail. UUID stored in 16 bytes is not necessarily 128
random bits. Neither generator nor hash replaces schema uniqueness.

## Engine work that matters

Retain Free Join factorization, COLT lazy tries, cover selection,
SIMD/batched probes and warm reuse. Set semantics offer conditional
dividends: existence-only suffix short-circuiting, projection
distinctness proved by keys, shared aggregate state for sum/mean at one
grain, direct determinant probes, reuse of unaffected relation images.

Priority is end-to-end locality: delta-local admission; first read after
insert/replace/delete; compiled projection reuse; charged cache
retention; avoiding a complete image build when a selective probe
suffices. Derived queries use the same resident-or-scratch relation
contract. Keep hot resident u32 row positions when eligible; switch to
the bounded cursor path before the representation limit.

Count complete named-decision path, retries and checkpoint costs, not one
winning PUT. Count admitted benchmark work only (`REVIEW-001`: visitor/used
counters, never `elapsed>0` or file-exists).

## Bound L20 scorecard (13 cells)

Print the live plan with `bumbledb-bench app-perf --plan`
(`appperf::plan::render`). The frozen matrix is six families, one cell
per needed regime — not a cartesian ritual. Overlapping
curves/heap/primerlane/adversarial timing jobs are not in this table.

| Cell id | Gate | Oracle / criterion |
| --- | --- | --- |
| `resident-read/warm` | `APP-FAST` | Verified result sets before timing; visit counts stay local |
| `resident-read/selective` | `APP-FAST` | Keyed oracle + `consume_visits`; existence suffix stops at first witness |
| `resident-read/cold-open` | `APP-FAST` | Same answers as warm; activation plus first read is counted |
| `mutation-read/post-write` | `APP-MUTATE` | Independent write model; first-read rebuild/copy bytes; no ID-allocation work |
| `numeric-interval/warm` | `APP-NUMERIC` | Independent bit/rational fixtures; bits agree before timing |
| `numeric-interval/selective` | `APP-NUMERIC` | Endpoint/Pack oracle; D11 order is logical, not insertion-token |
| `nonresident/large-result` | `APP-LARGE` | Streaming chunk-checksum; >RAM and >40 GiB allocated blocks, not a sparse map |
| `tenant-lifecycle/tenant-churn` | `APP-TENANTS` | Eviction releases native bytes; no runtime-global mutex; fixed workers |
| `tenant-lifecycle/maintenance` | `APP-TENANTS` | Checkpoint/GC beside writes without relation-sized stalls |
| `hosted-lifecycle/hosted-contention` | `APP-TENANTS` | Requests/bytes per terminal outcome at 1/2/4 writers; missing S3 = NotRun |
| `resident-read/warm` (`APP-TARGETS`) | `APP-TARGETS` | Identical answers on Apple Silicon, real Graviton ARM64, x86 Node |
| `resident-read/warm` (`APP-METHOD`) | `APP-METHOD` | Raw distributions, admitted-work denominators, cold/warm split, interleaved A/B |
| `resident-read/warm` (`APP-MAGIC`) | `APP-MAGIC` | Every high-impact constant is representation bound, host policy, or measured crossover |

Evidence rows for G15 must name these cell ids. A nonempty timing array
without them is not a scorecard.

## Bound script steps (`appperf::plan::script_steps`)

Semantic steps may run as daily counters after the source barrier.
**Timing steps are G15-only, serialized per host, and must not start
until writers are frozen and the post-retirement candidate exists.**

| id | kind | warmth | command |
| --- | --- | --- | --- |
| `verify-oracle` | Semantic | — | `bumbledb-bench verify --scale S --seed 1` |
| `scorecard-semantic` | Semantic | — | `bumbledb-bench app-perf --plan` |
| `storage-census` | Semantic | warm | `bumbledb-bench storage --scales S,M --seed 1 --out $OUT/storage` |
| `app-perf-warm` | Timing | warm | `bumbledb-bench app-perf --regimes warm --out $OUT/app-perf-warm` |
| `app-perf-cold` | Timing | cold | `bumbledb-bench app-perf --regimes cold-open,post-write --out $OUT/app-perf-cold` |
| `app-perf-tenants` | Timing | warm | `bumbledb-bench app-perf --regimes tenant-churn --tenants 8 --out $OUT/app-perf-tenants` |
| `large-populated` | Timing | warm | `bumbledb-bench app-perf --regimes large-result --scale L --out $OUT/large` (Graviton only) |
| `hosted-decision` | Timing | warm | L08/L10/L11 hosted driver over real S3 — not this crate |
| `hash-blake3` | Semantic | warm | `bumbledb-bench hash-probe --out $OUT/hash-probe` |
| `hash-aegis-optional` | Optional | warm | `bumbledb-bench hash-probe --kat $KAT --out $OUT/hash-aegis` |

Durability is matched: LMDB default commit vs SQLite WAL +
`synchronous=FULL` + `fullfsync`. Interleave baseline/candidate
controls; report cold/warm separately. A container on x86 does not
qualify ARM. Compilation does not qualify runtime behavior.

## Hardware prerequisites (`appperf::plan::hardware_prerequisites`)

Missing is **NotRun**, not a fabricated pass or `NotApplicable` waiver.

| id | Requirement |
| --- | --- |
| `apple-silicon-macos-arm64` | Local M-series Mac; pin `nightly-2026-08-15`; serialize via `scripts/measure.sh` |
| `graviton-linux-arm64` | Real Graviton instance (not an x86 container); AL2023 aarch64 |
| `linux-x86-64-node` | Node host with the packed native addon; event-loop delay column required |
| `real-s3-iam` | Hosted cells; emulator runs must be labeled emulator and cannot close G15 |
| `large-populated-disk` | >40 GiB allocated blocks + `cgroup memory.max`; `set_len` sparse maps are not this cell |

Graviton, x86 Node, real S3, and >40 GiB remain **NotRun** on this tree.

## Expected semantic checks (`appperf::plan::l21_semantic_checks`)

These are the L21-owned expected assertions against L20 outputs. They
are authored now; they are not executed here. Everyday D04/D08/D09/D11/D29
discriminators stay deterministic fast tests. Timing belongs in G15 only
after writers are frozen and the post-retirement candidate exists.

Seven correspondence ids are `bumbledb-bench`
`correspondence::OWNED_CASES`. They run as bench-crate cargo tests
(workspace nextest), not from `scripts/lean.sh`. The independent
judgment reference is `judge_final_state` (and the rational/float
oracles for D19) — not the production planner.

| Gate | Check id | Expected meaning |
| --- | --- | --- |
| `D04` | `compiled-index-locality` | Count source/group visits while unrelated groups scale; roster uses `ProjectionId`; compact u64 keys are 19 raw bytes |
| `C-D04-collision-bytes` | `exact-bytes-not-fingerprints` | Unequal canonical encodings stay distinct facts; `judge_final_state` rejects same-key unequal payloads; neither merge nor wrong delete |
| `C-D19-cancel` | `rational-sum-cancel` | `{1e16, 1, -1e16}` exact sum bits from the rational oracle, not host f64 add |
| `C-D19-mean-once` | `mean-not-rounded-sum` | Mean of two `MAX_FINITE` is `MAX_FINITE`; sum overflows |
| `C-D19-merge-not-idemp` | `merge_not_idempotent` | Merge one finite partial with itself doubles total and count |
| `C-G03-mutable-support` | `untouched-statement-stable` | Delta outside a statement's mutable consulted rels leaves that statement's `judge_final_state` citations unchanged |
| `C-G03-add-wins` | `changeset-add-wins` | Same exact fact on both sides of one ChangeSet stays Add; finish/parse refuse a second action |
| `C-G03-raw-commute` | `admission-does-not-commute` | Disjoint child adds sharing a capacity parent: set application commutes; admission of the union rejects |
| `D08` | `work-without-output-stops` | `WorkContext` `work_units` below exploration fails the query; visit count < relation cardinality. Retained COLT capacity is a separate assertion — not `used(WorkUnits) > 0` |
| `D09` | `derived-scratch` | Derived pipeline (aggregate/negation/recursion) accepts Scratch; peak `OwnerSnapshot.scratch_bytes` bounded; no whole-image resurrection. Not a tiny fallback query plus a direct resolver test |
| `D11` | `pack-logical-order` | Wide spill `[10,20)+[0,15)` → `[0,20)`; compare bits with resident pack; no `all_claims` gather |
| `D29` | `tenant-ownership` | Two owners: paused payload must not hold a runtime-global mutex; retained charge returns to baseline after close cycles |
| `G05` | `beyond-ram-and-32gib` | Enforced resident budget + allocated-block >40 GiB; sparse maps refuse the large cell |
| `G12` | `work-queue-scratch` | `OwnerSnapshot` + queue wait + event-loop delay columns; cancellation joins |
| `G15` | `measured-envelope` | Raw distributions and request counts; cold/warm separate; no best-median-only claim |
| `REVIEW-001` | `admitted-work` | Harness work denominator is visitor/used counters, never `elapsed>0` or file-exists |

See [apple-silicon-performance.md](apple-silicon-performance.md) for
historical M2 Max measurement discipline. Those notes are calibration
provenance, not 1.0 qualification evidence. M2 constants are not
inherited by Graviton or x86 Node.
