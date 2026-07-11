# 50 — Storage

LMDB is the only durable backend (decision recorded in `00-product.md`), accessed
through `heed`. Durability (fsync per commit; no `NOSYNC`/`WRITEMAP`/`MAPASYNC`), write
atomicity, and reader snapshot isolation come from real LMDB transactions. Single
writer, many reader threads, one process (`00-product.md`).
**Decision: `heed`.** **Alternative:** `lmdb-rkv`/raw FFI. **Why it lost:** heed is the
maintained thin binding; raw FFI buys nothing at this layer. **Reverses if:** heed
becomes a correctness or maintenance liability.

Environment constants are decisions, not knobs: `map_size` is fixed at 4 GB
(comfortably above the 1 GB scale axiom, allocated sparsely by the OS), and
`max_readers` at 1024 — inter-query parallelism is the scaling axis and `MDB_NOTLS`
binds reader slots to open transaction objects, so LMDB's default 126 would cap
concurrent snapshots, not threads; the raise costs a measured 64 bytes of lock file
per slot (~64 KiB total), and the snapshot past the table is the typed `ReadersFull`
error naming the limit.

## Design inputs (why this layout)

The first governing observation: an encoded fact is already a fixed-width row
sliceable into columns for free — **one sequential scan of a row-major table yields
every column.** A layout that instead stores field values severally across many
namespaces turns megabytes of column data into hundreds of thousands of LMDB
point-gets; this layout exists to make that shape unwritable.

The second: **guard namespaces are
derived accelerators for the dependency judgments (`30-dependencies.md`), not the
judgments' definitions.** `U` exists so the functionality check is O(log n) per
touched fact; `R` exists so the containment check's target side is O(log n) per
touched key. A namespace is the *plan* an accepted statement promised at the
acceptance gate — which is why statements without such a plan are rejected at
declaration, never discovered here.

## Key layout (one `_data` database, first-byte namespaces)

```
F | relation_id | row_id            -> fact_bytes     row-major facts   (reader: image builds, point-lookup fetch, export scan)
M | relation_id | fact_hash         -> row_id         membership        (reader: insert/delete idempotence, point lookups, WriteTx point reads)
U | relation_id | statement | key   -> row_id         FD guards         (reader: functionality checks — put-conflict and neighbor probes —
                                                      guard-probe lookups, WriteTx key reads, coverage walks)
R | statement | key | source_rel | source_row -> ()   IND reverse edges (reader: target-side containment checks on delete/shrink)
Q | relation_id | field_id          -> next_u64       fresh sequences  (reader: alloc)
S | relation_id | stat              -> u64            counters: stat 0 = row count (readers: the planner,
                                                      and image build's cross-check against the F scan);
                                                      stat 1 = row_id high-water (reader: commit's row-id assignment)
```

Plus `_meta` (format version, schema fingerprint, storage tx id, and the dictionary
next-id counter — the delta's pending-intern design mints provisional ids against it
from read snapshots) and `_dict` (**str-only** — `bytes<N>` values are inline in
facts, never interned, so the key hash carries no type tag: forward
`blake3(bytes) → id`, reverse `id → bytes`; collision axiom in
`10-data-model.md`). Key components are big-endian
(order-sensitive); stored values are not order-sensitive and are little-endian
(dictionary ids big-endian) — pinned here for the offline checker this doc defers.

- Every namespace names its reader above (README rule 3); a namespace with no
  reader is deleted. Declared opt-in accelerator namespaces exist only with a
  benchmark that demands them (OPEN, README).
- `fact_bytes` = the canonical encoding owned by `10-data-model.md`; identity = bytes.
- `fact_hash` = full 32-byte blake3 of `fact_bytes`; an `M` hit is trusted without
  verification (collision axiom, recorded in `10-data-model.md`).
- **`U` guard keys** are the FD statement's projected fields' canonical encodings,
  concatenated in statement order. An interval field (always last —
  `30-dependencies.md` gate) contributes its 16 bytes, so within one scalar-prefix
  group the guard B-tree is **ordered by interval start**: the property the
  pointwise check and the coverage walk stand on. A `bytes<N>` field contributes
  its ⌈N/8⌉ padded words — memcmp order over the uniform-width padded encodings is
  value-byte order, which is all the guard needs (order *operations* on `bytes<N>`
  stay refused at the query surface; sortedness is the index's need, not a
  semantics). `MAX_GUARD_WIDTH` admits the 16-byte interval contribution and the
  widest `bytes<64>` one; width overflow is a declaration-time error.
- **`R` keys are statement-scoped**, not relation-scoped: `statement` is the
  schema-global materialized statement id (`10-data-model.md` fingerprint), and
  `key` is the *target-side* projection value the source fact requires. One source
  fact contributes one `R` entry per containment statement whose selection it
  satisfies — conditional containments write reverse edges only for facts inside
  their σ, so the arm-validity and totality directions of a `==` each get exactly
  the edges they need. Bidirectional statements are two statement ids, symmetric
  entries.
- Key-component widths: `relation_id` u32, `field_id` u16, statement id u16, `row_id`
  u64 — all big-endian; ids assigned by declaration/materialized order and pinned by
  the fingerprint.
- Open-time checks, in order: storage format version, then schema fingerprint — each
  mismatch is a hard failure. No other format version opens and no migration path
  exists (ETL is the story, `70-api.md`; compatibility is never a design input,
  `00-product.md`).

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
canonical fact bytes, arena-backed, recording **net dispositions against committed
state**. During the closure, `insert`/`delete` are pure set arithmetic: encode the
fact, probe `M` (a read-only get) plus the delta to compute the `changed: bool`
return value, and record the *net* effect — a redundant op (insert of a committed
fact, delete of an absent one) records nothing, and an op whose net effect is
nothing *cancels* the pending opposite entry (delete + re-insert of a committed
fact, or insert + delete of an absent one, leaves no entry). The op-time probe is
authoritative because the single-writer mutex holds committed state stable for the
delta's lifetime; last-disposition-wins is a consequence of these rules, not a rule
of its own. **The invariant this buys:** the insert set contains exactly the facts
commit will add and the delete set exactly the facts it will remove — every entry
applies at commit (base state disagreeing with a proved disposition is the
`DispositionDesync` corruption, never a skip), the empty delta is the only no-op
commit shape, and judging a no-op insert is unrepresentable (the judgment-direction
divergence this closes is pinned in `60-validation.md`). `alloc` reads `Q` once at
first use per (relation, field) and increments
in memory (a transaction sees its own allocations); explicit-value inserts advance the
in-memory mark past the supplied value; mixed explicit/generated allocation tracks the
running maximum. **WriteTx point reads** (`70-api.md`: existence of a fact, lookup
through an FD key) are the same committed-state gets overlaid with the delta — they
observe exactly the final-state view the judgment checker will judge, which is what
makes read-modify-write idioms (upsert, check-then-act guards) sound without exposing
query machinery to the write path. **Nothing touches an LMDB data page until commit**
— an abort (error or panic) drops the arena and LMDB was never written, making
"failed writes leave nothing" true by construction, not by rollback.

**Commit applies the delta in one canonical order** — this is what makes dependency
enforcement commit-time final-state judgment (`30-dependencies.md`) with plain eager
mechanics:

1. **Deletes**: per deleted fact — `M` get → row_id → delete `F`, `M`, its `U` entries
   (guard keys re-derived by slicing projected fields out of `fact_bytes` — never a
   scan), and its outgoing `R` entries. Deleted `U` keys are recorded per statement
   (the target-side check set for step 3).
2. **Inserts**: per inserted fact — `F` put (row_id from the in-memory high-water),
   `M` put, `U` puts, `R` puts (per containment statement whose selection the fact
   satisfies). Because every delete has already landed and the insert-set is
   deduplicated, a scalar `U` put conflict here **is** a functionality violation →
   typed error, whole transaction aborts. For a **pointwise FD** (interval-carrying
   guard), the put cannot conflict on exact bytes alone; the insert additionally
   runs the **ordered-neighbor probe** — cursor-seek to (scalar prefix, start):
   predecessor in the same prefix group with `end > start`, or successor with
   `start < end`, is the violation. Two probes, O(log n), same B-tree. Deletes and
   inserts both check what they touch: a live `M` entry whose `F` row or `U` guard
   is missing is the membership-desync corruption, a hard error — never silently
   scrubbed.
3. **Judgment phase** (final-state probes; LMDB write txns read their own writes) —
   one checker, statement-driven, restricted to delta-touched bindings:
   - **Containment, source side:** every inserted fact satisfying a statement's
     source selection probes the target's key guard for its projected tuple; the
     found target fact is checked against the target selection (one `F` get when a
     selection exists). For interval positions, the probe is the **coverage walk**:
     from the guard entry at or before the source interval's start, walk start-
     ordered entries of the prefix group, requiring no gap before the source's end
     — O(log n + segments), sound because the target's pointwise key keeps its
     intervals disjoint (`30-dependencies.md`). The frontier loop is the shared
     segment sweep (`interval/sweep.rs`, one walk for the checker's gap verdict and
     `Pack`'s coalescing fold); the commit site owns only entry-segment location
     and the key-shape trust checks.
   - **Containment, target side:** every target key tuple deleted in step 1 and not
     re-established in step 2 probes its statements' `R` prefixes for surviving
     source entries; for interval positions the deleted-or-shrunk window's `R`
     range is walked and each surviving source is re-checked for coverage against
     the final target state. A surviving requirer → typed error naming the *source*
     fact by its bytes. **Re-establishment is per statement, ψ-qualified:** for a
     dependent statement with a nonempty target selection, a re-landed guard tuple
     counts as re-established only if the establishing fact satisfies that
     statement's ψ (one `F` get per re-established tuple per ψ-carrying dependent;
     empty-ψ dependents use the plain set difference). Without the qualification,
     delete + re-insert of identical key bytes with a changed selection field
     strands sources in a committed state — the unqualified difference is unsound
     under selections.
   - Bidirectional statements run both bullets with the sides swapped — the same
     two code paths, no third.
   Any failure → typed error carrying the statement id, abort. The probe primitive
   ("does any fact match / does no fact match") is shared with the query executor's
   anti-probe (`40-execution.md`) — one mechanism, two callers.
4. **Counters flush**: row_id high-waters, row counts, `Q` sequences, the pending
   dictionary entries and next-id, storage tx id.
5. **LMDB commit** (fsync). The durability boundary parses its errno once (the
   trust-boundary rule, applied to the OS): a raw OS errno out of `mdb_txn_commit`
   — the commit's write/sync syscalls; on macOS the data-page `pwrite`s and
   `fcntl(F_FULLFSYNC)`, whose errno `mdb.c` surfaces raw with no fallback sync —
   is the typed `CommitSync` error naming phase and syscall class, never a bare
   `Lmdb(Io(...))`. The transient form (`F_FULLFSYNC` observed failing under I/O
   pressure) gets a **bounded, observable retry**: the failed commit aborted its
   transaction (nothing persisted), so the whole transaction is rebuilt from the
   immutable plan and re-committed — each retry an obs event
   (`commit_sync_retry`), the escaping error carrying the count. The contract is
   untouched: a retry re-runs the full write-and-sync, so every commit that
   reports success fsynced — no sync mode exists, and none may be born.

User operation order inside the closure is therefore semantically irrelevant; the
delete-before-insert trap and reference-insertion-ordering are unrepresentable. Crash
consistency is LMDB atomicity — *tested* (crash/reopen family, `60-validation.md`).
Dictionary entries are never removed (accepted leak; the delete path never *adds*
one either — a never-interned value proves its fact absent).

Two write-side asymmetries, recorded as decisions rather than left as surprises:
**R-delete verification** — deleting a fact deletes its `R` entries without
verifying they existed (unlike `F`/`M`/`U`, whose absence is the
`MembershipDesync` hard error); a missing `R` entry is not independently
detectable at delete time without re-deriving every statement's edges, and the
class is covered by the offline sweeper, `Db::verify_store` — the same
compensating control that re-verifies the rest of M↔F↔U↔R consistency. **Counter overflow guards** — the fresh ceiling is guarded
(`FreshExhausted` at `u64::MAX`, because hosts can supply explicit fresh values),
while the storage tx id and row-id high-waters are not: they advance by at most one
per commit/insert, so wrapping needs ~2⁶⁴ commits — twelve orders beyond the scale
axiom, and no host input can jump them. The asymmetry is chosen, not overlooked.

**Storage tx id:** advances **once per commit that changed logical state**; a commit
whose delta is empty (all no-ops) does not advance it and does not invalidate any
image. It lives in `_meta` and commits atomically with the data. A successful no-op
commit still flushes any *dirty* fresh marks (`Q` values that advanced past their
committed base — allocations the closure may have returned to the host) in a
counters-only LMDB transaction: the tx id identifies query-visible state (`F/M/U/R`),
and `Q` marks are write-path bookkeeping no query reads, so every image and memo key
stays valid. Pending interns of a no-op commit are dropped — intern ids never escape.

Bulk load (`70-api.md` surface) is the same delta mechanism at scale — chunked into
multiple transactions (4096 facts each; a failing chunk aborts whole, prior chunks
stay committed, and the error carries the committed count). Chunking has a new
stated consequence under bidirectional containments: **a `==` statement's cluster
must be judged whole**, so a chunk boundary that splits a cluster mid-load fails
that chunk's commit loudly (never silently); the documented import order —
dependency-cluster order, owned by `70-api.md`'s ETL section — makes the failure
unreachable for well-formed exports. The fresh-database append-order fast path
stays deliberately unbuilt.

**Corrupt data is a hard error, never a skip:** an `F` value whose length differs from
the schema's fact width, a dangling intern id, an `M`/`F` disagreement, an
out-of-range enum ordinal, an interval with `start ≥ end` — any of these aborts the
scan/query with a corruption error; an engine that silently skips undecodable rows
silently shrinks query results, which is the worse bug. Reopen-trusted counters are
additionally **bounded before they size anything**: the image build caps the claimed
`S` row count by the `_data` DBI entry count (`mdb_stat`, O(1)) — a witness that
over-approximates any one relation's rows because the DBI spans every namespace,
which is exactly what a ceiling is allowed to do — and a claim above it is the typed
`CounterDesync` corruption *before* any size-derived allocation; the F-scan
cross-check stays the exactness guarantee. An offline integrity
checker (M↔F↔U↔R sweep) is out of scope for v0, stated.

## The columnar image cache (the hot representation)

The bridge to paper-faithful execution (`40-execution.md` D1):

- A **relation image** is **all columns** of a relation, decoded from one sequential
  `F`-prefix scan into whole-slab, 128-byte-aligned SoA vectors (one allocation per
  store, freed as a whole — the arena discipline without the arena type), plus the
  row count. **An interval field decodes into two parallel 8-byte columns**
  (start, end) — the image layer has no 16-byte column kind, membership and overlap
  lower to word comparisons over the pair (`40-execution.md`), and every existing
  kernel shape (predicate scan, compaction, gather, fold) applies unchanged. A
  `bytes<N>` field generalizes the same precedent: ⌈N/8⌉ parallel word columns
  (one plain word column for N ≤ 8), with the trailing pad validated zero at
  decode. The multi-byte unit exists only in `fact_bytes` and guard keys, where
  ordering needs it.
  At ~60 GB/s of single-core scan bandwidth a build is single-digit milliseconds per
  100 MB — the number that makes the whole cache design sound. **Column pitches are
  padded off 16 KiB multiples** (measured): L1D set congruence (256 sets × 64 B
  lines, bits 6–13) costs at most 1.55× on real lockstep scans — never the folklore
  10–20×, which requires a fully dependent load chain — while the hazard that
  actually matters is stream-prefetch trackers aliasing on low 16 KiB page-number
  bits: power-of-two-ish pitches with small (1–3 line) staggers cost 4–6× on
  DRAM-tier lockstep scans (8.13 vs 1.78 ns/row). The rule:
  when a column-to-column pitch within a slab is ≥ 64 KiB and lands within 384 B of a
  16 KiB multiple, round it up to the next exact multiple (exact multiples measured
  clean — the poison is the small offset). Immutable once built. Positions in the
  image are **dense scan ordinals**; `row_id`s exist only in LMDB keys and never
  appear in images (COLT offsets are image positions; the guard-probe path reads `F`
  directly and never needs a translation).
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
- Invariant test: two sequential read
  transactions with no intervening write share identical image instances; plus the
  concurrent families in `60-validation.md`.

## Memory discipline

Images are whole-slab allocations freed as wholes; no per-value heap objects in
storage or images. Query scratch belongs to prepared queries (`40-execution.md`).
Steady-state process heap = LMDB's mmap + the newest generation's images +
per-prepared-query pools + a constant. Prepared queries hold current-generation
images only: prepare binds no image at all (`View::Unbound`), and each execution
reaps memoized bindings below its generation — old images die with the last pinned
reader or the first post-commit execution, whichever is later.

## Operations

Backup = file copy of the environment (or `mdb_copy`) while the writer is quiesced.
Compaction and space reclamation = ETL into a fresh database (`70-api.md`
export/import surfaces). The LMDB file never shrinks; the dictionary leaks by
accepted design. That is the entire operational story, deliberately.

## Store-size anatomy and compaction

The store is larger than SQLite's for the same logical content, structurally and by
design — recorded so nobody re-derives it:

- **Freelist churn**: chunked bulk-load commits leave CoW residue as free pages.
  LMDB never shrinks its file; length reflects peak usage.
- **Several `_data` entries per fact by design**: fact (`F`) + membership hash
  (`M`) + one FD guard (`U`) per key + one reverse edge (`R`) per satisfied
  containment direction. This is deliberate rent for O(log n) commit-time
  judgment checks and stays.
- **16 KB pages** on Apple Silicon (LMDB uses the OS page size) — chunkier
  B-tree overhead than SQLite's 4 KB pages with varint-packed rows.

The churn component is recoverable: `Db::compact(dest)` writes a live-pages-only
sequential copy through LMDB's `mdb_env_copy2(MDB_CP_COMPACT)` (copy-and-swap,
never in-place; refuses an existing destination; the copy is a first-class
writable store). The bench corpus cache loads into a scratch sibling and
compacts into place, so cached corpora ship live-sized. Auto-compaction of
live stores stays a non-goal — the door is tool-driven.
