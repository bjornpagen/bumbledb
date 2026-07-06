# 40 — Storage

LMDB is the only durable backend (decision recorded in `00-product.md`), accessed
through `heed`. Durability (fsync per commit; no `NOSYNC`/`WRITEMAP`/`MAPASYNC`), write
atomicity, and reader snapshot isolation come from real LMDB transactions. Single
writer, many reader threads, one process (`00-product.md`).
**Decision: `heed`.** **Alternative:** `lmdb-rkv`/raw FFI. **Why it lost:** heed is the
maintained thin binding; raw FFI buys nothing at this layer. **Reverses if:** heed
becomes a correctness or maintenance liability.

## Design inputs (why this layout)

The v5 layout stored every field value 4–6 times across ten namespaces and did ~15
B-tree puts per inserted fact; its per-(field,row) column cells turned megabytes of
column data into hundreds of thousands of LMDB point-gets — 80% of traced execution
(post-mortem §20–§24, `docs/history/post-mortem.md`). The governing observation: an
encoded fact is already a fixed-width row sliceable into columns for free — **one
sequential scan of a row-major table yields every column.**

## Key layout (one `_data` database, first-byte namespaces)

```
F | relation_id | row_id            -> fact_bytes     row-major facts   (reader: image builds, point-lookup fetch, export scan)
M | relation_id | fact_hash         -> row_id         membership        (reader: insert/delete idempotence, point lookups)
U | relation_id | constraint | key  -> row_id         unique guards     (reader: constraint checks, guard-probe lookups)
R | target_rel  | constraint | key | source_rel | source_row -> ()      (reader: Restrict checks on delete)
Q | relation_id | field_id          -> next_u64       serial sequences  (reader: alloc)
S | relation_id | stat             -> u64            counters: stat 0 = row count (readers: the planner,
                                                     and image build's cross-check against the F scan);
                                                     stat 1 = row_id high-water (reader: commit's row-id assignment)
```

Plus `_meta` (format version, schema fingerprint, storage tx id, and the dictionary
next-id counter — the delta's pending-intern design mints provisional ids against it
from read snapshots) and `_dict` (forward `blake3(tag‖bytes) → id`, reverse
`id → bytes`; collision axiom in `10-data-model.md`). Key components are big-endian
(order-sensitive); stored values are not order-sensitive and are little-endian
(dictionary ids big-endian) — pinned here for the offline checker this doc defers.

- Every namespace names its reader above (README rule 3). The v5 namespaces with no
  reader (`H`, `P`, `C`, always-on `A`) stay deleted; declared opt-in accelerators may
  return only with a benchmark that demands them (OPEN, README).
- `fact_bytes` = the canonical encoding owned by `10-data-model.md`; identity = bytes.
- `fact_hash` = full 32-byte blake3 of `fact_bytes`; an `M` hit is trusted without
  verification (collision axiom, recorded in `10-data-model.md`; v5 truncated to 16
  bytes — we do not).
- Key-component widths: `relation_id` u32, `field_id` u16, constraint id u16, `row_id`
  u64 — all big-endian; ids assigned by schema declaration order and pinned by the
  fingerprint.
- Open-time checks, in order: storage format version, then schema fingerprint — each
  mismatch is a hard failure.

**Decision: one `_data` database with first-byte namespaces.** **Alternative:** one
LMDB database per namespace (enables per-namespace append mode and integer-key layouts).
**Why it lost:** a fixed tiny DBI set is simpler and the access patterns are prefix
scans and point gets either way. Stated consequence: LMDB append mode is only usable
for a **fresh-database bulk load written in global key order**; incremental writes never
append (an `M`/`Q`/`S` key always exceeds every `F` key). **Reverses if:** bulk-load
profiling shows append mode mattering for incremental use.

**Decision: `M` indirection (hash → row_id) rather than keying facts by their bytes.**
**Alternative:** `F | relation | fact_bytes -> ()` directly. **Why it lost:** fact-bytes
keys make `F` keys wide and unbounded-ish, while dense-ordinal row storage wants a
compact monotonic key; and images need scan ordinals anyway. **Reverses if:** never
likely; revisit only with the layout.

## Write path: the transaction is a delta

A write transaction is an **in-memory delta** — a net insert-set and delete-set of
canonical fact bytes (last disposition per fact wins), arena-backed. During the
closure, `insert`/`delete` are pure set arithmetic: encode the fact, probe `M` (a
read-only get) plus the delta to compute the `changed: bool` return value, record the
disposition. `alloc` reads `Q` once at first use per (relation, field) and increments
in memory (a transaction sees its own allocations); explicit-value inserts advance the
in-memory mark past the supplied value; mixed explicit/generated allocation tracks the
running maximum. **Nothing touches an LMDB data page until commit** — an abort (error
or panic) drops the arena and LMDB was never written, making "failed writes leave
nothing" true by construction, not by rollback.

**Commit applies the delta in one canonical order** — this is what makes constraint
enforcement commit-time state-checking (`10-data-model.md`) with plain eager mechanics:

1. **Deletes**: per deleted fact — `M` get → row_id → delete `F`, `M`, its `U` entries
   (guard keys re-derived by slicing constrained fields out of `fact_bytes` — never a
   scan), and its outgoing `R` entries.
2. **Inserts**: per inserted fact — `F` put (row_id from the in-memory high-water),
   `M` put, `U` puts, `R` puts. Because every delete has already landed and the
   insert-set is deduplicated, **a `U` conflict here is a genuine unique violation** →
   typed error, whole transaction aborts. Deletes and inserts both check what they
   touch: a live `M` entry whose `F` row or `U` guard is missing is the
   membership-desync corruption, a hard error — never silently scrubbed.
3. **FK validation** (final-state probes; LMDB write txns read their own writes):
   forward — every inserted fact's FK targets resolve via `U`; Restrict — every unique
   key deleted in step 1 and not re-inserted has no remaining `R` entries. Any failure
   → typed error, abort.
4. **Counters flush**: row_id high-waters, row counts, `Q` sequences, the pending
   dictionary entries and next-id (moved here from step 2 — equivalent inside one
   atomic txn, since nothing between the phases reads `_dict`), storage tx id.
5. **LMDB commit** (fsync).

User operation order inside the closure is therefore semantically irrelevant; the
delete-before-insert trap and FK insertion-ordering are unrepresentable. Crash
consistency is LMDB atomicity — *tested* (crash/reopen family, `50-validation.md`).
Dictionary entries are never removed (accepted leak).

**Storage tx id:** advances **once per commit that changed logical state**; a commit
whose delta is empty (all no-ops) does not advance it and does not invalidate any
image. It lives in `_meta` and commits atomically with the data. A successful no-op
commit still flushes any *dirty* serial marks (`Q` values that advanced past their
committed base — allocations the closure may have returned to the host) in a
counters-only LMDB transaction: the tx id identifies query-visible state (`F/M/U/R`),
and `Q` marks are write-path bookkeeping no query reads, so every image and memo key
stays valid. Pending interns of a no-op commit are dropped — intern ids never escape.

Bulk load (`60-api.md` surface) is the same delta mechanism at scale — chunked into
multiple transactions (4096 facts each; a failing chunk aborts whole, prior chunks
stay committed, and the error carries the committed count). The fresh-database
append-order fast path was deliberately not built (decision: it saves only the
membership probes on an empty database, and the normal insert path is the one with
the invariants); it may return with a benchmark that demands it.

**Corrupt data is a hard error, never a skip:** an `F` value whose length differs from
the schema's fact width, a dangling intern id, an `M`/`F` disagreement, an out-of-range
enum ordinal — any of these aborts the scan/query with a corruption error (v5 silently
skipped undecodable rows and shrank query results — post-mortem §37). An offline
integrity checker (M↔F↔U↔R sweep) is out of scope for v0, stated.

## The columnar image cache (the hot representation)

The bridge to paper-faithful execution (`30-execution.md` D1):

- A **relation image** is **all columns** of a relation, decoded from one sequential
  `F`-prefix scan into whole-slab, 128-byte-aligned SoA vectors (one allocation per
  store, freed as a whole — the arena discipline without the arena type), plus the
  row count.
  At 60–120 GB/s of scan bandwidth this is single-digit milliseconds per 100 MB — the
  number that makes the whole cache design sound. **Column bases are staggered**: the
  image staggers successive column slab bases by odd multiples of the line size so no
  two columns of one relation are congruent mod 16 KB (the L1D set stride) — lockstep
  multi-column scans otherwise alias into one 8-way set, the documented 10–20×
  pathological case (`docs/reference/apple-silicon-performance.md`, Category 5).
  Immutable once built. Positions in the image are **dense scan ordinals**; `row_id`s
  exist only in LMDB keys and never appear in images (COLT offsets are image positions;
  the guard-probe path reads `F` directly and never needs a translation).
  **Decision: full-width images, cache key `(relation_id, storage_tx_id)`.**
  **Alternative:** per-field-scope images. **Why it lost:** scope keys are combinatorial
  (defeating sharing and the "tiny key space" claim), overlapping scopes duplicate
  columns, and at ≤1 GB whole relations are cheap. **Reverses if:** a wide-relation
  workload appears (it won't; BCNF relations are narrow).
- **Generation correctness:** a reader's generation T is the storage tx id read from
  `_meta` **inside its own snapshot** — never an in-process counter. This closes the
  open-snapshot/read-counter race that could poison the shared cache.
- The cache is a field of the `Db` handle, shared by reader threads through `&Db`
  (the handle is `Send + Sync`; no `Arc` of the cache itself is needed since one
  handle exists per path). Two readers at the same T racing to build the same image:
  both may build; insert-if-absent, the loser adopts the winner's `Arc` and drops its
  own (accepted waste, no latch). The insert re-checks the newest generation under
  the lock, so a reader racing a commit cannot re-insert an evicted generation.
- **Eviction:** at each state-changing commit, the writer drops all entries older than
  the new generation from the map. Readers still pinned at older generations keep their
  `Arc`s alive until their transactions end; a long-lived old-generation reader that
  needs an *unbuilt* image builds it query-locally without caching (accepted — writes
  are bursty and rare). There is **no memory-pressure eviction, ever** — the scale
  axiom, stated.
- **Filters:** on a cold relation with a filtered query, one *storage* scan produces
  both the cached unfiltered image and the query-local survivor view (the filter is a
  second pass over the decoded in-memory columns — the storage scan is the expensive
  part); on a warm relation the view is computed by scanning the cached image (NEON
  filter kernels). Views are survivor-position vectors in retained-capacity buffers,
  never cached; the prepared query additionally memoizes its views per (generation,
  resolved filters), so a warm re-execution skips even the in-memory re-scan.
- Invariant test (from the v5 regression, post-mortem §26): two sequential read
  transactions with no intervening write share identical image instances; plus the
  concurrent families in `50-validation.md`.

## Memory discipline

Images are whole-slab allocations freed as wholes; no per-value heap objects in
storage or images. Query scratch belongs to prepared queries (`30-execution.md`).
Steady-state process heap = LMDB's mmap + the newest generation's images +
per-prepared-query pools + a constant.

## Operations

Backup = file copy of the environment (or `mdb_copy`) while the writer is quiesced.
Compaction and space reclamation = ETL into a fresh database (`60-api.md` export/import
surfaces). The LMDB file never shrinks; the dictionary leaks by accepted design. That
is the entire operational story, deliberately.

## Store-size anatomy and compaction (docs/architecture/40-storage.md, measured 2026-07-03)

The S-scale corpus store measured 101 MB against SQLite's 13.6 MB for the same
logical content. `mdb_stat` anatomy, so nobody re-derives it:

- **~39 % freelist churn**: 2,529 of 6,463 pages were free — CoW residue from
  the ~43 chunked bulk-load commits. LMDB never shrinks its file; length
  reflects peak usage.
- **~5–6 `_data` entries per fact by design**: fact (`F`) + membership hash
  (`M`) + unique guard (`U`) + one back-reference (`R`) per FK — 761,786
  entries for ~152,700 facts. This is deliberate rent for O(1) commit-time
  constraint checks and stays.
- **16 KB pages** on Apple Silicon (LMDB uses the OS page size) — chunkier
  B-tree overhead than SQLite's 4 KB pages with varint-packed rows.

The churn half is recoverable: `Db::compact(dest)` writes a live-pages-only
sequential copy through LMDB's `mdb_env_copy2(MDB_CP_COMPACT)` (copy-and-swap,
never in-place; refuses an existing destination; the copy is a first-class
writable store). The bench corpus cache loads into a scratch sibling and
compacts into place, so cached corpora ship live-sized. Auto-compaction of
live stores stays a non-goal — the door is tool-driven.
