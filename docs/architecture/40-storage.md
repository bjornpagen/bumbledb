# 40 — Storage

LMDB is the only durable backend, accessed through `heed`. Durability, write atomicity,
and reader snapshot isolation come from real LMDB transactions — no in-memory shadow
stores, no fake transaction layers. Single writer, many readers, inherited from LMDB.

## Design inputs (why this layout)

The v5 layout stored every field value 4–6 times across ten namespaces and did ~15
B-tree puts per inserted fact; its per-(field,row) column cells turned a few MB of
column data into hundreds of thousands of LMDB point-gets — 80% of traced execution
(post-mortem `todo/20`–`todo/24` at `1b65ae8^`). The governing observation: an encoded
fact is already a fixed-width row that can be sliced into columns for free. **One
sequential scan of a row-major table yields every column.** All layout decisions follow
from that.

## Key layout (one `_data` database, first-byte namespaces)

```
F | relation_id | row_id            -> fact_bytes     facts, row-major, monotonic row_id
M | relation_id | fact_hash         -> row_id         membership + handle (blake3 of fact_bytes)
U | relation_id | constraint | key  -> row_id         unique guards
R | target_rel  | constraint | key | source_rel | source_row -> ()   reverse-FK guards
Q | relation_id | field_id          -> next_u64       serial sequences
S | relation_id | stat              -> value          row counts, stats
```

Plus `_meta` (format version, schema fingerprint, storage tx id) and `_dict`
(forward hash→id with verify, reverse id→bytes).

- `fact_bytes` = concatenation of fixed-width encoded fields (bool 1, enum 1, others 8;
  strings/bytes as intern ids). Field slicing is offset arithmetic, O(1), no decode.
- Encodings are order-preserving (big-endian u64, sign-flipped i64) so composite keys
  sort correctly wherever ordering matters.
- **What does not exist, deliberately** (each was a v5 namespace with no reader or no
  justification): per-field column cells (`C`), handle→fact duplicates (`H`),
  handle→row_id side tables (`P`), always-on per-field value accelerators (`A`).
  Secondary access paths may return later as *declared, opt-in* accelerators, only with
  a benchmark that demands them.

**Decision: row-major durable facts.** **Alternative:** persist true columnar segments
(build columns at write time). **Why it lost:** at ≤100s of MB, one sequential scan per
relation per write-generation is microseconds-to-milliseconds; persistent columns buy
that back at the cost of write-path complexity and a second durable representation to
keep consistent. The image cache (below) makes the read side columnar anyway.

## Write path

Per inserted fact: one `F` put, one `M` put, guard puts for declared constraints, dict
puts for novel strings. Counters (`row_id` high-water, row counts, storage tx id) are
maintained in memory during the write transaction and flushed **once at commit** — not
read-modify-written per fact (v5 did 3 counter RMWs per insert). Bulk load sorts by key
and uses LMDB append mode where key order allows. Failed transactions leave nothing:
LMDB atomicity is the whole crash-consistency story.

Every committed logical write advances the **storage tx id** — the invalidation token
for everything downstream.

## The columnar image cache (the hot representation)

The bridge between LMDB and paper-faithful Free Join (`30-execution.md` D1):

- A **relation image** is the set of requested columns of a relation, decoded from one
  sequential `F`-prefix scan into arena-backed, 128-byte-aligned SoA vectors, plus the
  row count. Immutable once built.
- The cache lives at **environment scope**, keyed by
  `(relation_id, field_scope, storage_tx_id)`, shared by all read transactions via
  cheap clones (`Arc`). A read transaction at tx-id *T* gets images for *T*, building
  and inserting them on first use. A write commit bumps the tx id; stale-generation
  images fall out by key. LMDB's MVCC guarantees the builder scanning inside a read
  txn sees exactly generation *T* — correctness needs no locks beyond the map's.
- Filters push down into the image build when an image is being built anyway (scan,
  test, keep survivors) — but filtered *views* are query-local; only unfiltered images
  are cached, keeping the cache key space tiny and hit rates high.

**This cache is the fix for the old repo's quietest failure:** v2 specified exactly
this cross-transaction cache; v5 silently constructed it per-read-transaction, making
every query rebuild every image, then spent four PRD suites optimizing the rebuild. The
invariant worth a test: two sequential read transactions with no intervening write share
identical image instances.

## Memory discipline

Images are arena-allocated and freed as wholes; no per-value heap objects anywhere in
storage or images. Query-side scratch comes from prepared-query arenas
(`30-execution.md`). The dictionary decode path allocates only in the caller's result
buffer. Target: the steady-state process heap is images + LMDB's mmap + a constant.
