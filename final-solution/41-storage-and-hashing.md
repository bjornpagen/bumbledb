# 41 — Earn every stored byte; use hashes for their actual job

Execution routing: P14 probes/accounting; P02 physical layout; P01 canonical equality; P04/P09 authoritative digests; P00 selects C12 format decisions. Probes execute only in final F3 before physical format qualification, per 64. See [work packets](62-work-packets.md) for source ownership and complete deliverables.

Status: proposal and source-backed cost analysis, 2026-09-04. No new storage benchmark or hash race was executed here. The reported engine/SQLite numbers are the repository's historical results, not a measurement of the successor. This chapter answers the owner's storage-size, magic-number and TigerBeetle questions and adds binding pre-format qualification work.

## Conclusion first

Some storage amplification buys useful indexed admission and cheap access. **The whole measured gap is not an unavoidable price of Free Join or set semantics.** A full membership digest per fact, duplicate key material, an immortal two-way text dictionary, fixed-width numeric encodings and B-tree occupancy all deserve separate accounting. The warm columnar images and lazy COLT tries are in memory, not persisted copies charged to `data.mdb`.

Selected default for the successor:

- **16-byte local membership fingerprints plus exact canonical-byte comparison.** Use the first 16 bytes of domain-separated BLAKE3 initially; compare the AEGIS candidate before freezing the format. A collision may add lookup work, never merge two facts.
- **32-byte BLAKE3 for authoritative content identities:** schema, command/receipt binding, decision chains, snapshots, migration plans and remote objects. These occur per object/decision/receipt, not automatically per fact; count that population independently. A small hot tenant can retain far more decisions than live facts.
- **Existing 64-bit routing hashes inside transient fixed-word tables**, with full-key comparison and bounded work. Do not put a cryptographic digest into every hot join probe.
- **16-byte application IDs**, generated once before sealing. They are not content hashes or proof of database-issued uniqueness.

Do not add a pluggable hashing subsystem, persist a choice per row, or silently vary a persisted hash algorithm by CPU. One format fixes its algorithms and domains. Hardware implementations of that algorithm must return identical bytes. The AEGIS comparison can revise the selected local algorithm before format freeze if evidence warrants it; it is not a commitment to ship two persistent formats.

## What the in-repo evidence actually says

The [README](../README.md) attributes the following compacted values to the 2026-08-22 shared-machine M2 Max campaign at revision `01084e3e`:

| Workload | Bumbledb bytes/row | Indexed SQLite bytes/row | Ratio | Excess bytes/row |
| --- | ---: | ---: | ---: | ---: |
| Ledger | approximately 167 | approximately 73 | 2.29× | approximately 94 |
| Calendar | approximately 228 | approximately 93 | 2.45× | approximately 135 |

These rounded values do not support finer precision. The same README reports substantially worse churn file-size ratios; raw high-water files and compacted steady-state sizes are not interchangeable observations.

The [storage lane](../crates/bumbledb-bench/src/lanes/storage.rs) loads matching generated populations, measures the raw Bumbledb store, reconstructs/opens a compacted sibling, compares indexed and table-only SQLite, truncates the SQLite WAL, closes connections, and cross-checks per-relation row counts. Bumbledb's `disk_size()` delegates to heed's `real_disk_size()`; this is a file-size measure, not a per-namespace live-page accounting or a process-RSS measurement. The README's `night-2026-08-22/` raw-report directory was not found in this checkout during this review; preserve the recorded claims as historical until the raw artifacts are located or the campaign is rerun. Do not invent per-index percentages from charts.

The README also records weaknesses that matter more to applications than a peak join ratio: SQLite wins 19/22 CRUD comparisons and 10/12 constraint comparisons; a failed durable key check includes persistence of the old never-reuse ID guarantee. Removing the allocator removes that particular obligation, but hosted durable rejection receipts still require authoritative publication. Do not claim all rejection latency vanishes.

## The current physical bill, before page overhead

The audited [key codec](../crates/bumbledb/src/storage/keys.rs) and [commit applier](../crates/bumbledb/src/storage/commit/applier.rs) make this accounting explicit. Let `W` be one encoded fact's width and `d` the projected key width.

| Current entry | Key bytes | Value bytes | Raw bytes per entry |
| --- | ---: | ---: | ---: |
| Fact `F` | tag 1 + relation 4 + local row 8 | `W` | `13 + W` |
| Membership `M` | tag 1 + relation 4 + digest 32 | local row 8 | 45 |
| Key determinant `U` | tag 1 + relation 4 + statement 2 + `d` | local row 8 | `15 + d` |
| Reverse containment/capacity edge `R` | tag 1 + statement 2 + `d` + source relation 4 + source row 8 | empty or weight 8 | `15 + d`, or `23 + d` |
| Dictionary forward, per distinct historical string | tag 1 + digest 32 | intern ID 8 | 41 |
| Dictionary reverse, per distinct historical string | tag 1 + intern ID 8 | UTF-8 text | `9 + text length` |

Thus every ordinary fact already costs **`W + 58` raw bytes for F+M**, before any determinant, reverse edge, dictionary, counters, LMDB node/slot/page overhead or unused page space. Every additional law-derived index may carry another projected key and row reference. Counter/metadata entries are generally per relation/field, not per fact; don't multiply them by the row count.

For example, a **hypothetical**, non-text 24-byte row with one 8-byte determinant and one 8-byte unweighted reverse edge costs `24 + 58 + 23 + 23 = 128` raw bytes before pages. This is not a reconstruction of either benchmark's schema; it shows why narrow records amplify metadata.

The text dictionary adds `50 + text length` raw bytes per distinct historical string, plus an 8-byte intern reference at each stored occurrence. Repeated long strings can amortize that well. Unique short strings and deleted historical strings can do badly. Removing mandatory interning improves lifetime semantics and can save overhead, but **inline text can increase space for highly repeated values**. Closed vocabulary relations are already a good typed representation for actual vocabularies; don't silently turn arbitrary text back into a hidden global dictionary.

Current [scalar widths](../crates/bumbledb-theory/src/schema.rs) are also concrete: bool 1 byte; integer and interned text reference 8; interval 16; fixed-width integer interval 8; `bytes<N>` padded to an 8-byte multiple. The store does not simply persist all values as 64-bit words—bool is already compact. Canonical external bytes, persisted fact bytes and warm query words need not share padding. The successor should not preserve unused tail padding merely because the warm kernel likes word-aligned loads; decoding once is already part of that architecture.

### Why SQLite can be smaller even with indexes

SQLite's [record format](https://www.sqlite.org/fileformat2.html#record_format) stores integers using sizes selected by serial type: 0/1 have zero payload bytes; other values can use 1, 2, 3, 4, 6 or 8 bytes. B-tree rowids use varints. This is not the same as saying every SQL integer cell is a varint. Our fixed-width 8-byte IDs/counts/timestamps spend more on small values.

The benchmark [SQL mapping](../crates/bumbledb-bench/src/sqlmap.rs) uses an `INTEGER PRIMARY KEY` rowid alias for eligible fresh/closed IDs and avoids a redundant separate index for that key. It also builds source indexes for containment and workload indexes. Bumbledb currently keeps fact storage, full-fact membership and determinant/reverse namespaces. SQLite's compact rowid representation and our per-fact membership index are a real structural difference.

Index equivalence must be checked, not inferred from the label “indexed”: interval/capacity enforcement can need different structures, and the benchmark generator does not synthesize an identical physical index for every law. Compare the actual declared semantics and concrete index roster of each workload. Native LMDB page layout, key repetition, page fill and copy-on-write high-water retention add another layer. A bigger file does not by itself establish that LMDB is inefficient; nor does choosing LMDB justify ignoring duplicated keys.

### What to change and what to measure first

1. Remove database-issued entity IDs, C product machinery and the immortal dictionary as already selected. Keep required safety/semantic behavior.
2. Shorten local membership fingerprints from 32 to 16 bytes **while adding exact collision handling**. One candidate representation is `(relation, fingerprint, local-row-id) → empty`, fetching the fact for exact equality. The row ID moves from the value to the key; do not duplicate it in both. All colliding rows remain enumerable and individually deletable.
3. Account for every live F/M/U/R entry and each derived index. Share or omit an index only when one physical representation demonstrably supplies every required lookup/order/update operation. An 8-byte local row ID may still be worth its indirection; do not make secondary indexes repeat a long application key to save an unrelated counter.
4. Keep schema/statement discriminants compact and justified. Per-relation LMDB databases might remove repeated prefixes but add tree roots, small-tenant floor cost and database-handle limits. Do not introduce that cross-product as a speculative optimization.
5. Compare compact canonical facts with the warm representation. Integer varints/compression are candidates only if actual disk/cold-read benefit exceeds decode/complexity cost; no new compression engine is selected. Generic `I64`/`U64` retain their numeric domains, and 128-bit IDs are recommended identity values, not a ban on application-owned integer keys.

Truncating one per-fact digest saves **16 raw bytes per fact**: 16 MB per million facts, 1.6 GB per 100 million facts (decimal units), before page effects. Against the rounded historical totals that is an illustrative 9.6% of 167 or 7.0% of 228 bytes/row—not a measured file reduction. It cannot alone erase a 94–135-byte gap. Exact comparisons introduce row fetches and collision-safe indexes change node shape; qualify end-to-end CPU/I/O/fanout, not only the subtraction.

Conversely, replacing the previous proposal's 28-byte IDs with 16-byte IDs saves 12 bytes per occurrence. **Compared with current 8-byte fresh IDs, 16-byte IDs add 8 bytes per occurrence.** Do not combine those two baselines into a fictitious net saving. They buy portable application-owned identity while deleting the allocator; assess the repeated-reference cost explicitly.

## TigerBeetle: the exact answer

At inspected commit [`47aeb2212a255273dda508288412e537d11e4b7c`](https://github.com/tigerbeetle/tigerbeetle/commit/47aeb2212a255273dda508288412e537d11e4b7c), TigerBeetle's [`src/vsr/checksum.zig`](https://github.com/tigerbeetle/tigerbeetle/blob/47aeb2212a255273dda508288412e537d11e4b7c/src/vsr/checksum.zig) uses **`Aegis128LMac_128` with an all-zero 16-byte key**, returning a **128-bit/16-byte checksum**. The source names disk bitrot, network framing and prepare/client hash chains as its uses. The MAC specialization uses the message as associated data with an empty encrypted message; the fixed public key makes this a checksum, not secret-key authentication.

It requires hardware AES support at compile time, caches the initialized state, and hashes by copying that state and streaming the input. AES round instructions exist on Apple Silicon, Graviton and suitable x86 CPUs; actual runtime feature exposure must still be qualified on the selected host. Hardware AES does not automatically mean every Vercel x86 instance promises every newer VAES vector extension.

TigerBeetle [vendors the algorithm](https://github.com/tigerbeetle/tigerbeetle/blob/47aeb2212a255273dda508288412e537d11e4b7c/src/stdx/vendored/aegis.zig) from Zig 0.13.0 to maintain checksum stability. This detail matters: “use whichever current AEGIS implementation” is not an exact persisted format specification. Its [checksum benchmark](https://github.com/tigerbeetle/tigerbeetle/blob/47aeb2212a255273dda508288412e537d11e4b7c/src/vsr/checksum_benchmark.zig) varies blobs from 1 KiB to 1 MiB. That benchmark is not evidence for hashing a 16–64-byte Bumbledb fact or for end-to-end LMDB inserts.

The vendored code itself warns about collision probability for the 128-bit MAC output. Adopting the same checksum does not abolish the birthday bound. A cryptographic construction with a public fixed key also must not be described as authenticating a hostile sender. Secret-key MAC security theorems do not by themselves establish public-zero-key collision resistance. Our proposed AEGIS experiment concerns exact-checked local fingerprints, not a silent replacement of authoritative cryptographic commitments.

### Why BLAKE3 was a reasonable choice—and where it is oversized

The current [fact hash](../crates/bumbledb/src/encoding/fact_hash.rs) explicitly selects full 32-byte BLAKE3 and treats collisions as an accepted logical axiom. The successor rejects that **axiom**, not merely the algorithm. A smaller exact-checked index is safer semantically than a larger digest incorrectly treated as equality.

[BLAKE3's official implementation](https://github.com/BLAKE3-team/BLAKE3) has Rust support, streaming, fixed cross-platform results, test vectors and SIMD implementations including NEON and x86 feature detection. It is not an unaccelerated scalar straw man. Its default 256-bit output supplies approximately 128-bit generic collision resistance, useful for authoritative content addresses and commitments. AES acceleration may beat it for selected regimes; no local result establishes that here.

**Output width and hashing time are separate decisions.** Taking BLAKE3's first 16 bytes does not halve its compression work. It can still improve index density, comparisons and I/O. AEGIS might separately reduce hash CPU; measure setup/state-copy, short-input latency, batch throughput, streaming boundaries and full write/read behavior. More bytes processed per second on a bulk buffer is not necessarily faster per small fact.

For an 8-MiB snapshot chunk, removing 16 digest bytes saves about **0.00019%** of payload size per digest occurrence. A 40-GiB checkpoint has 5,120 such chunks: 81,920 binary digest bytes saved per occurrence in a manifest (163,840 if encoded as hex). This is not a meaningful remedy for full-checkpoint upload amplification. Small decision/receipt records can be more digest-heavy; inventory their exact field multiplicities, omit derived duplicate fields where safe, and keep the required authority strength.

## How many bytes do we need?

No finite hash can prevent every collision over arbitrary-length input. Exact tuple comparison is what prevents a hash collision from changing the database's meaning.

For `n` distinct inputs and an ideal uniform `b`-bit hash, accidental birthday collision probability is approximately:

```text
lambda = n(n - 1) / 2^(b + 1)
p ≈ 1 - exp(-lambda)
when lambda is small: p ≈ lambda

for desired small probability epsilon:
b >= ceil(log2(n(n - 1) / (2 epsilon)))
```

These are sizing assumptions, not a proof that any particular noncryptographic hash behaves ideally under hostile inputs. Choose `n` for the actual collision domain and retention period. Separate tenant/relation lookup namespaces do not mistake equal fingerprints across namespaces for the same fact. Fleet risk sums the relevant per-namespace probabilities; it is not always the square of the global row count. Long-lived content addresses may need to account for every distinct object generated over the relevant lifetime, not only the hot cache.

| Distinct inputs in one domain | 8 bytes / 64 bits | 12 bytes / 96 bits | 16 bytes / 128 bits | 32 bytes / 256 bits |
| --- | ---: | ---: | ---: | ---: |
| 1 million | 2.71 × 10⁻⁸ | 6.31 × 10⁻¹⁸ | 1.47 × 10⁻²⁷ | 4.32 × 10⁻⁶⁶ |
| 1 billion | 2.67% | 6.31 × 10⁻¹² | 1.47 × 10⁻²¹ | 4.32 × 10⁻⁶⁰ |
| 1 trillion | effectively 100% | 6.31 × 10⁻⁶ | 1.47 × 10⁻¹⁵ | 4.32 × 10⁻⁵⁴ |

For accidental collision probability below `10^-15`, the approximation requires 89 bits at one million inputs, 109 at one billion, and 129 at one trillion: respectively at least 12, 14 and 17 whole bytes. There is no single magic byte count without a population and risk budget. We choose 16 for local fingerprints because it is compact, naturally handled, has negligible accidental collisions at the intended per-tenant populations, and **correctness is exact even when one occurs**. A 12-byte option buys only four further bytes and another width choice; it is not selected without evidence justifying the format/CPU tradeoff.

Example fleet arithmetic: one million independent lookup domains with one million distinct rows each gives approximately `10^6 × 1.47e-27 = 1.47e-21` accidental 128-bit collisions, not the `1.47e-15` for one shared trillion-item collision domain. Exact comparison makes either a performance event, not data loss. For deliberate collision search, a generic `b`-bit cryptographic hash offers roughly `b/2` collision bits; a 16-byte content commitment is only about 64-bit generic collision resistance, unlike a full 32-byte commitment's approximately 128. Random accidental probabilities must not be substituted for that adversarial analysis.

Random corruption of one already chosen message against its expected checksum has a different ideal miss estimate (`2^-b` per independent corruption) from any-pair birthday collisions. Hash chains and content addressing use equality across objects and therefore cannot universally claim only the easier corruption model.

Application ID entropy is separate again: a full-random 16-byte helper can use 128 random bits; UUIDv4 stores 128 bits but has 122 random bits after version/variant fields. UUIDv7 has timestamp/sequence/randomness semantics of its own. Do not use the 128-bit table blindly for every UUID generator or claim probability-zero identity collisions. Keys reject conflicting facts under the declared law.

## A small physical accounting and hash qualification campaign

Before freezing the layout, run the existing correctness-stamped storage/CRUD/read workloads and a per-student fixture. Add narrow measurement output to existing tools, not a storage-analysis service:

1. Canonical payload bytes and row counts per relation; live key/value bytes and counts by F/M/U/R/dictionary/metadata namespace; overhead from local/application IDs, digest bytes and padding.
2. LMDB page size, branch/leaf/overflow pages, free pages, occupancy where observable, file length and OS allocated blocks. Mixed namespaces share pages; do not invent an exact per-namespace page attribution without an actual accounting method. Compare raw, compacted, after-churn and held-reader cases.
3. SQLite actual DDL/index roster, `dbstat`/page/freelist figures, checkpointed main file plus WAL and allocated blocks. Same rows and comparable durability/constraints; show unavoidable semantic/index differences instead of hiding them.
4. Separate resident images/tries/plans/results from disk. Sweep cold, warm, post-write and >RAM regimes; report total local peak including migration/checkpoint/result scratch.
5. Compare current full BLAKE3, truncated BLAKE3 and an exact-pinned AEGIS-128 checksum implementation on representative 0/8/16/24/32/64/128-byte, 1-KiB, 4-KiB and 8-MiB canonical inputs. Include one-shot/streaming equivalence, state initialization, alignment, realistic input mixtures and hash-once reuse. Never silently copy TigerBeetle's little-endian `u128` convention into an unspecified byte codec.
6. Inspect actual hot assembly and CPU feature dispatch. Preserve a same-output portable implementation where the selected target contract needs it. No local instruction specialization may alter canonical bytes; no global CPU-native build may accidentally exclude Vercel's declared floor.
7. Force constant local fingerprints across insert, contains, delete, key judgment, joins, grouping, reopen and spill. Large collision buckets must remain bounded/cancellable with exact results, including mismatched schemas and long values. Corrupted/malformed authority commitments must refuse; immutable object creation must not overwrite an existing conflicting payload. A deliberately colliding object-address test requires comparing existing object bytes when resolving that creation conflict. A downloaded object's digest alone cannot detect every genuine cryptographic collision: full authoritative content identity retains its explicit collision-resistance premise, unlike exact local tuple equality. A valid digest alone is never authorization.

Deleting one hash, one duplicate key or one copy may improve CPU, disk and cache simultaneously. That is the plausible triple win. It is earned by exactness plus measured byte/work reduction, not by borrowing another database's algorithm name.

## Required gates

| Gate | Required evidence |
| --- | --- |
| `SPACE-01` | Reproduced same-data indexed-SQLite/raw/compacted comparison with actual index roster, namespace key/value accounting, page/free/allocated bytes and no claim that RAM-only tries occupy the file |
| `SPACE-02` | Before/after local fingerprint and ID/text-layout variants: exact semantics, disk/RSS/peak scratch, CRUD and warm/post-write/cold/>RAM costs; distinguish current 8-byte IDs from superseded 28-byte proposal IDs |
| `HASH-01` | Role/width/domain inventory and independent known-answer tests across Apple Silicon/Graviton/x86; byte-identical streaming and CPU implementations; full authoritative digests and opaque ETags never truncated by a generic helper |
| `HASH-02` | Forced constant/short fingerprints through every equality/admission/query/delete/reopen/spill path; collision buckets preserve exact values and bounded work; malformed object commitments refuse |
| `HASH-03` | Reproducible sizing math with namespace/population/retention/adversarial assumptions, including UUID entropy, random corruption versus birthday collisions, and no finite-hash collision-free claim |
| `HASH-04` | Pre-format BLAKE3/AEGIS actual-input comparison with pinned implementation, vectors, feature floor, assembly and end-to-end storage workload; selected algorithm documented, unearned alternatives removed rather than shipped as a plugin matrix |

These refine G00/G02/G03/G04/G05/G13/G14/G15. They do not claim any successor benchmark, native algorithm switch or layout reduction has already passed.
