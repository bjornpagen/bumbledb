# Performance and physical storage: earn the complexity

This is an application database, not an analytics engine. The performance target is a warm per-user database with narrow relations, selective lookups, short mutations and frequent composed joins. A cold or oversized tenant remains correct and usable with slower I/O. This replacement handoff ran no benchmark; historical numbers below are attribution, not successor results.

## The physical representation we select

Keep LMDB, exact full-tuple membership, local row-ID indirection and compiled access paths. Keep BLAKE3. Use 16-byte fingerprints only to narrow an exact comparison; use 32-byte commitments where bytes must authenticate an object/history identity without retrieving another full preimage. Use exact bounded determinant encodings when they make the index simpler and suitably compact. Select the encoding once per compiled access path, never per row or CPU.

| Role | Width | Reason |
| --- | --- | --- |
| Physical row ID | 8 bytes | Local row indirection, not a content hash or portable application identity |
| Application Id128 | 16 bytes | Ordinary application data, chosen once before a command is sealed |
| Membership / wide determinant fingerprint | 16 bytes | Candidate routing; canonical bytes still decide equality |
| Exact determinant | Checked encoded width | Avoid hashing small fixed domains; ordering is explicit |
| Schema, command, decision, object and migration commitments | 32 bytes | Authoritative content bindings, with distinct domains |
| S3 ETag / conditional version | Opaque provider value | A CAS witness, not our content digest; do not truncate or reinterpret |

These are essential roles, not six configurable hash strategies. Prefer small semantic newtypes backed by one codec implementation to general helpers accepting arbitrary digest lengths. Verification can compare full durable rows independently of the routing fingerprint. Input, lookup, deletion, replay and spill must all preserve collision safety.

### Raw index cost, before page overhead

The current key layout in `crates/bumbledb/src/storage/store/rows.rs` and `det_index.rs` has:

| Entry | Raw key bytes |
| --- | ---: |
| Row namespace + relation + row ID | 1 + 4 + 8 = 13 |
| Membership namespace + relation + fingerprint + row ID | 1 + 4 + 16 + 8 = 29 |
| Determinant namespace + statement + fingerprint + row ID | 1 + 2 + 16 + 8 = 27 |

One fact with one fingerprint determinant index therefore uses 69 raw key bytes plus its canonical payload, before node headers, branch separators, pages, alignment, metadata, free pages or copy-on-write amplification. The 2-byte determinant discriminator is a current physical statement number; the compiled design must give shared projections a stable, checked physical discriminator, not blindly persist declaration-order accidents. Recalculate this table if that representation changes.

An exact u64 determinant would use 19 bytes at the same discriminator width, saving 8 key bytes. An exact Id128 uses 27 bytes, saving no disk over a 16-byte fingerprint but avoiding hashing/bucket comparisons. A 400-byte bounded composite might fit LMDB's key limit and still be a terrible index choice. **Fits the backend is a correctness eligibility check, not a cost model.** Select exact scalar determinants up to 16 encoded bytes, subject to the complete physical key bound; use fingerprint buckets otherwise. Wider exact ordered access requires a specific useful consumer and measured justification. Qualify the selected rule on common u64/Id128/composite keys before format freeze; do not add a runtime autotuner.

Share identical access paths across laws/lookups only when projection, filter, domain, order and candidate-state semantics match. A key law's candidate index must represent multiple conflicting rows until judgment. Do not replace it with a physical unique index that rejects early and loses diagnostic evidence.

Removing the full-row membership index is not selected by default. A determinant index is not automatically an exact full-row membership test for unkeyed relations or a way to find rows under every schema. A layout proposal must list every operation that depends on the removed structure and show its exact replacement. No new cluster-by-primary-key storage family in this pass.

### Why the historical SQLite gap is not just “the price of sets”

The repository README reports the 2026-08-22 Apple M2 Max experiment at `01084e3e`: roughly 2.3–2.45× the indexed SQLite storage in its ledger/calendar comparison. It predates the current successor. It is not evidence that the new layout uses that ratio, or that all of the old overhead was necessary.

Contributors can include multiple LMDB B-trees/index entries, repeated key prefixes, tuple/value representation, fill factor, free pages after churn and copy-on-write history pinned by readers. RAM-only COLT structures do not occupy the durable file. A reserved virtual map size is neither allocated file size nor resident memory. Set semantics require exact duplicate detection; they do not require one particular redundant index roster.

Reproduce the same logical facts, constraints, durability and useful indexes in SQLite and Bumbledb. Report live payload, each namespace's key/value bytes, leaf/branch/overflow/free pages, allocated/file sizes, compacted size, and peak import/scratch disk. Separate a fresh database from churn and from an old pinned snapshot. Distinguish disk amplification from process RSS and query-cache retention. Keep measured extra bytes if they buy the application's real reads/writes; remove demonstrably redundant bytes. Do not promise SQLite-sized files by changing the comparison's guarantees.

## Collision math, with the population stated

For independent uniformly distributed b-bit digests of n distinct values in one compared namespace, the birthday approximation is `p ≈ n(n−1)/2^(b+1)` while p is small. This is not an adversarial security proof or a guarantee of no collisions.

| Distinct values in a namespace | 128-bit birthday probability | 256-bit birthday probability |
| --- | ---: | ---: |
| 10^6 | 1.47 × 10^-27 | 4.32 × 10^-66 |
| 10^9 | 1.47 × 10^-21 | 4.32 × 10^-60 |
| 10^12 | 1.47 × 10^-15 | 4.32 × 10^-54 |

For exact-checked local fingerprints, collisions affect work, not truth. Test with a constant hash to make that contract real. A malicious collision-heavy input still consumes budget and may be refused; do not quietly truncate a bucket. For authoritative commitments, generic collision work is about 2^128 for a 256-bit digest, versus 2^64 for a 128-bit digest. Application ID entropy depends on the selected generator; UUID version/reserved bits mean “stored in 16 bytes” is not necessarily 128 random bits. Neither generator nor hash replaces schema uniqueness.

Count tenants/namespaces separately when identifiers cannot meet; sum their probabilities for fleet estimates. Count retained decisions/objects separately from live facts. A small repeatedly edited tenant can have few facts and enormous history. Random bitrot acceptance is a different experiment from collisions among all pairs.

### TigerBeetle and AEGIS

The previous source-backed investigation pinned TigerBeetle commit [`47aeb2212a255273dda508288412e537d11e4b7c`](https://github.com/tigerbeetle/tigerbeetle/blob/47aeb2212a255273dda508288412e537d11e4b7c/src/vsr/checksum.zig): `Aegis128LMac_128`, fixed all-zero 16-byte key, 16-byte output. This is a checksum use, not secret-key sender authentication. That finding is preserved here; this review did not re-fetch its current upstream HEAD. Hardware AES instructions make AEGIS interesting, but its large-buffer results do not prove superior short-fact latency. BLAKE3 already has portable, NEON and x86 SIMD implementations.

Selected release default is BLAKE3. Truncating its output saves index bytes, not half the compression work. AEGIS comparison is **optional**, restricted to exact-checked local fingerprints, and cannot expand into a hash plugin matrix. This deliberately narrows the old mandatory AEGIS experiment in `HASH-04`; the obligation is now a documented algorithm decision and actual-input BLAKE3/layout qualification. If an experiment establishes a worthwhile alternative before format freeze, review that one format decision with fixed vectors and platform coverage. Otherwise retain the implemented algorithm and move on.

For perspective, removing 16 bytes from one digest per 8 MiB checkpoint chunk saves approximately 0.00019% of payload. It cannot fix full-checkpoint upload amplification. Inspect duplicated commitment fields and checkpoint frequency, not just digest width.

## The engine work that matters

Retain Free Join's factorization, COLT lazy tries, cover selection, SIMD/batched probes and warm reuse. It is already more than a binary join loop wearing the name. Set semantics offer **conditional** dividends: existence-only suffix short-circuiting, projection distinctness proved by keys, shared aggregate state for sum/mean at one grain, direct determinant probes, reuse of unaffected relation images. They do not give a universal asymptotic improvement over the Free Join paper or eliminate all intermediate deduplication.

The priority is end-to-end locality: delta-local admission; first read after insert/replace/delete; compiled projection reuse; charged cache retention; avoiding a complete image build when a selective probe suffices. Derived queries must use the same resident-or-scratch relation contract. Do not build another optimizer, generic vector VM, adaptive sampling service or parallel group engine. Keep hot resident u32 row positions when eligible; switch to the bounded cursor path before the representation limit, without panics or silent truncation.

## Qualification workloads

Use a compact scorecard, not hundreds of overlapping benchmark jobs:

1. **Resident application read:** exact key hit/miss, selective joins, fanout/existence, anti-join, equal-argument distinct entities, named-stage reuse, a small positive recursive relation. Include symbolic source-field arithmetic in the migration application path, not merely constant backfills. Measure preparation separately from execution and owned result/page conversion.
2. **Mutation/read pair:** insert, replace, delete, no-change, rejection; immediately repeat the prepared read. Report judge groups/rows, relation versions invalidated, image bytes rebuilt, allocations and latency. Metadata-only log changes must not rebuild unrelated application images.
3. **Numeric/interval:** exact sum, mean and both over one/many groups; dedup-required versus witnessed-distinct; dense interval overlap/pack/length. Compare exact bits and errors before timing.
4. **Nonresident:** working set above an enforced resident budget, long text, wide groups, spill transition, large results and actual data above 32 GiB. A huge sparse map with tiny contents is not this test. Require bounded memory and useful completed queries; report temporary disk and long-reader effects.
5. **Tenant lifecycle:** many mostly idle users, opening storms, LRU churn, retained snapshots/results, slow tenant plus small neighbor. Measure native retained bytes, FDs/mappings, event-loop delay, queue wait and cleanup. Shared runtime means no per-tenant executor fleet. Many idle snapshot entries must share fixed workers without parked reactors. Trace actual payload contention: no runtime-global mutex may cover scratch reads, conversion or destruction.
6. **Hosted lifecycle:** 1/2/4 contenders, accepted/no-op/rejected commands, lost responses, checkpoint under sustained writes, bounded replay, backup and generated migration. Count requests and bytes per terminal outcome, wasted candidates and p50/p99. S3 round trips remain real latency; do not imply a Vercel cold start is a warm embedded query.

The actual `../edullm/packages/data/native-ledger` consumer guides the fixture, without copying its private production data. The older bronze/explanation paths are historical references, not mandatory imaginary integration targets. L20 uses ../bumblebench/README.md read-only as measurement discipline: its M2 Max cache/branch/MLP figures are calibration evidence, not universal constants.

## Constants and measurement policy

Classify high-impact constants at their owner: representation bounds, scheduling/resource policy, measured crossover, or experimental historical knob. Examples to inspect are u32 image positions, determinant inline width, map-growth step, exact-sum limb bound, join batches/work-poll quanta, token/image generation capacity, scratch pages, host ingestion/page size, worker/queue count, checkpoint chunks/tail bounds and retry limits. Prove structural bounds; expose a coherent small policy for resources; benchmark crossovers. Do not expose every private threshold as a public option.

Pin `nightly-2026-08-15` unless deliberately requalified. Existing `try_blocks` and `portable_simd` are means, not targets. Retain architecture-specific kernels only where they earn their extra implementation against the portable path. Check assembly for the intended hot loops, not an enormous brittle instruction-text mirror. No fast-math or implicit reassociation of specified F64 arithmetic.

Target Apple Silicon macOS ARM64, real Graviton Linux ARM64, Linux x86-64 Node/Vercel-class deployment. Record CPU, OS/libc, toolchain, exact artifact/data, flags, memory limits, durability, ambient load, sample counts and raw distributions. Serialize performance measurement on a host. Interleave baseline/candidate controls; report cold/warm separately, not just best medians. A container on x86 does not qualify ARM; compilation does not qualify runtime behavior.

Correctness/counter/ownership assertions are deterministic fast tests. Timing comparisons belong in explicit qualification. Do not assert a universal numerical speed budget without measured baseline and owner-approved application target. Release claims must state the measured envelope and any unavailable platform evidence. The absence of measurements in this proposal is intentional, not a performance pass.
