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
S | relation_id | "rows"            -> u64            row count         (reader: the planner; S contains nothing else)
```

Plus `_meta` (format version, schema fingerprint, storage tx id) and `_dict` (forward
`blake3(tag‖bytes) → id`, reverse `id → bytes`; collision axiom in `10-data-model.md`).

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

## Write path

Per inserted fact: encode → `M` get (hit ⇒ idempotent no-op) → `F` put (row_id from the
in-memory high-water) → `M` put → `U` puts per constraint (including serial
auto-uniques) → `R` puts per outgoing FK → dict puts for novel strings. Counters —
row_id high-waters, row counts, **serial `Q` sequences**, storage tx id — are maintained
in memory during the write transaction and flushed once at commit. Serial `alloc` reads
`Q` once at first use per (relation, field), then increments in memory (a transaction
sees its own allocations; aborts leave `Q` untouched); explicit-value inserts advance
the in-memory mark past the supplied value (mixed explicit/generated allocation tracks
the running maximum). Failed transactions leave nothing: LMDB atomicity is the whole
crash-consistency story — and it is *tested* (crash/reopen family, `50-validation.md`).

**Delete path, with the same specificity:** encode fact → `M` get → row_id (absent ⇒
idempotent no-op) → **Restrict check**: for each unique constraint of this relation
that any FK targets, prefix-scan `R | this_rel | constraint | key` — any entry is a
restrict violation (in-transaction interaction with the deleting of referencing facts
is part of the constraint-timing OPEN) → delete `F`, `M`, this fact's `U` entries
(guard keys re-derived by slicing the constrained fields out of `fact_bytes` — never a
scan), and this fact's outgoing `R` entries → decrement in-memory row count. Dictionary
entries are never removed (accepted leak).

**Storage tx id:** advances **once per commit that changed logical state**; a commit
consisting only of idempotent no-ops does not advance it and does not invalidate any
image. It lives in `_meta` and commits atomically with the data.

Bulk load (`60-api.md` surface): sorts and uses append mode on fresh databases per the
decision above; otherwise it is insert semantics in one transaction.

**Corrupt data is a hard error, never a skip:** an `F` value whose length differs from
the schema's fact width, a dangling intern id, an `M`/`F` disagreement, an out-of-range
enum ordinal — any of these aborts the scan/query with a corruption error (v5 silently
skipped undecodable rows and shrank query results — post-mortem §37). An offline
integrity checker (M↔F↔U↔R sweep) is out of scope for v0, stated.

## The columnar image cache (the hot representation)

The bridge to paper-faithful execution (`30-execution.md` D1):

- A **relation image** is **all columns** of a relation, decoded from one sequential
  `F`-prefix scan into arena-backed, 128-byte-aligned SoA vectors, plus the row count.
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
- The cache is owned by the environment handle, shared by reader threads via `Arc`
  clones. Two readers at the same T racing to build the same image: both may build;
  insert-if-absent, the loser adopts the winner's `Arc` and drops its own (accepted
  waste, no latch).
- **Eviction:** at each state-changing commit, the writer drops all entries older than
  the new generation from the map. Readers still pinned at older generations keep their
  `Arc`s alive until their transactions end; a long-lived old-generation reader that
  needs an *unbuilt* image builds it query-locally without caching (accepted — writes
  are bursty and rare). There is **no memory-pressure eviction, ever** — the scale
  axiom, stated.
- **Filters:** on a cold relation with a filtered query, one scan produces both the
  cached unfiltered image and the query-local survivor view; on a warm relation the
  view is computed by scanning the cached image (NEON filter kernels). Views are
  survivor-position vectors, arena-backed, never cached.
- Invariant test (from the v5 regression, post-mortem §26): two sequential read
  transactions with no intervening write share identical image instances; plus the
  concurrent families in `50-validation.md`.

## Memory discipline

Images are arena-allocated and freed as wholes; no per-value heap objects in storage or
images. Query scratch belongs to prepared queries (`30-execution.md`). Steady-state
process heap = LMDB's mmap + the newest generation's images + per-prepared-query arenas
+ a constant.

## Operations

Backup = file copy of the environment (or `mdb_copy`) while the writer is quiesced.
Compaction and space reclamation = ETL into a fresh database (`60-api.md` export/import
surfaces). The LMDB file never shrinks; the dictionary leaks by accepted design. That
is the entire operational story, deliberately.
