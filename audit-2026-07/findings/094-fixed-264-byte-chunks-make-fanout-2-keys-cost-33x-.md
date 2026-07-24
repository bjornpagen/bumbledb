## Fixed 264-byte chunks make small-fanout keys cost up to ~33x their payload in the COLT chunk pool

category: perf | severity: low | verdict: CONFIRMED | finder: engine:colt

### Summary

COLT's chunked child lists use one fixed geometry: `CHUNK_LEN = 64` positions per chunk (`crates/bumbledb/src/exec/colt.rs:32`), giving a 264-byte `Chunk` (`[u32; 64]` positions + `len: u8` + `next: u32`, size verified by compilation: 264). The singleton optimization keeps a key's first position inline in the packed child word (`colt.rs:150-156`), but the second position of any key allocates a full zero-initialized chunk plus a fresh `NodeState` pool entry (`crates/bumbledb/src/exec/colt/append_child.rs:12-29`). At a forced level whose keys have small fanout — 2-8 duplicates per key, the common foreign-key-join shape — each key stores 8-32 bytes of payload in a 264-byte chunk: up to ~33x inflation of the chunk pool, and the force pass writes all 264 bytes (the `[0; CHUNK_LEN]` zero-init plus the push) per such key. Under the allocation contract, this footprint is a retained monotone high-water across executions, not a transient.

### Evidence (verified against the code)

- `crates/bumbledb/src/exec/colt.rs:32` — `const CHUNK_LEN: usize = 64;` with the rationale comment "bounded pointer traversal, independent loads within a chunk (the deviation from the paper's growable per-key vectors)". No fanout-sensitivity rationale.
- `crates/bumbledb/src/exec/colt.rs:140-146` — `struct Chunk { positions: [u32; CHUNK_LEN], len: u8, next: u32 }`. `size_of::<Chunk>() == 264` (compiled and checked).
- `crates/bumbledb/src/exec/colt/append_child.rs:12-29` — on `Slot::Single`, the second position constructs `Chunk { positions: [0; CHUNK_LEN], len: 2, next: u32::MAX }`, pushes it whole, and pushes a `NodeState::Unforced(Positions::Chunks { .. })` node entry.
- `crates/bumbledb/src/exec/colt/force.rs:46-62, 81-112` — `force` is a single pass over all of a node's positions; every duplicate-key position lands in `append_child` (force.rs:100), so forcing a level materializes one chunk chain per multi-position key for the entire node at once.
- `crates/bumbledb/src/exec/colt/select.rs:108-122` — set-level union survivors copy through the same 264-byte chunks even for 2-position unions. (Framing correction: these union chunks sit past the `PoolMark` watermark and are reclaimed at the next `select` — select.rs:27-29, 177-184 — so the durable cost is the force path, not the union path.)
- Spec check — the Free Join paper, `docs/free-join-paper/arXiv-2301.10841v2/tex/04-optimizations.tex` §"COLT: Column-Oriented Lazy Trie" (line 181): "A COLT is a tree where each leaf is a vector of offsets into" the columns — growable per-key vectors, which size to their content.
- Doctrine check — `docs/architecture/40-execution.md:796-802` records the chunked-child-list as a deliberate deviation ("64 offsets per arena chunk... rather than the paper's growable per-key vectors or a two-pass contiguous layout"), but its "Reverses if" clause is scoped to a force+iterate microbenchmark of two-pass-contiguous vs chunked. The chunk *size* against small fanouts is measured nowhere: no test (tests/sizing.rs covers singleton/chunked structure only), no bench pin, no ruling.
- Retention check — `docs/architecture/40-execution.md` "The allocation contract": scratch capacity, including COLT pools, is a monotone high-water retained by the prepared query, so the inflated chunk pool persists across executions.

### Bench impact

A 10^7-row relation forced at a level with average fanout 2 allocates ~5×10^6 chunks ≈ 1.32 GB of chunk pool for ~40 MB of position payload, plus ~5×10^6 `NodeState` entries. Beyond footprint, the force pass dirties a mostly-empty 264-byte span (four-plus cache lines) per key, and chunk-chain iteration loads whole lines for 8-byte payloads — force and iterate bandwidth dilute proportionally. Any benchmark whose inner trie levels have low per-key fanout (FK joins, sparse graph edges keyed by source) carries this tax; high-fanout levels (≥~32 duplicates per key) are unaffected.

### Suggested fix

Graded chunk sizes in one pool — e.g. a small first chunk (8 positions) with subsequent chunks at 64; the chain already carries `next`, so the geometry costs no new indirection, though a heterogeneous chunk pool needs a byte- or word-addressed slab rather than the current homogeneous `Vec<Chunk>`. Alternative: spill a key's first chunk-worth of positions into the dense slab as a `(start, len)` child word once the parent's force completes (positions are append-only during force, fixed afterward). Either way, per the repo's measured-choice doctrine (the same 40-execution.md entry demands a microbenchmark to reverse the chunked layout), this wants a force+iterate measurement at fanouts {2, 4, 8, 64} before shipping — the finding's mechanism is arithmetic-certain, the end-to-end win is not yet measured.
