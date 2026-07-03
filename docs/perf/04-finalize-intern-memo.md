# PRD 04 — Finalize intern memo and buffer dedup

Authority: `30-execution.md` (finalize, the allocation contract),
`docs/architecture/40-storage.md` (the dictionary), suite README finding 2.
Independent of PRDs 00–03.

## Purpose

`ResultBuffer::push_word` resolves every String/Bytes cell through
`dict::resolve` — an LMDB B-tree lookup **per emitted row**. fk_walk pays
60.9 µs of its 118.9 µs resolving one distinct holder name 109 times; the
hot-set p99s (9.6–11 ms) carry the same tax at 50k rows. Resolve each distinct
intern id **once per finalize**, and store its bytes **once per buffer**.

## Technical direction

- `PreparedQuery` gains a reused scratch memo mapping intern word →
  `(start: u32, len: u32)` into the output buffer's `bytes`:
  - Reuse the existing open-addressed word-map machinery
    (`exec/wordmap.rs`) rather than `std::collections` — arity-1 keys, value =
    packed `(u32, u32)`. It already has the clear-retaining-capacity
    discipline (`wordmap.rs` `clear`).
  - Cleared at the top of every `finalize` call (offsets are only valid for
    the buffer being filled; the caller owns and may clear/reuse the buffer
    between executions).
- `finalize` (`api/prepared.rs`) threads the memo into the per-cell decode.
  New flow for `ValueType::String`/`Bytes` in `push_word`:
  1. Memo hit ⇒ push `Cell::String { start, len }` pointing at the **existing**
     bytes range — no dict lookup, no byte copy. (Cells referencing shared
     ranges are already the layout; nothing else changes.)
  2. Miss ⇒ `dict::resolve`, validate UTF-8 as today, append bytes, insert
     `(word → (start, len))`, push the cell.
  This makes buffer bytes deduplicated as a side effect: K rows sharing one
  memo string carry its bytes once instead of K times — smaller buffers, and
  `Row::get` is unchanged (ranges were always indirection).
- **Observability.** Add `names::DICT_RESOLVE` (`Category::Storage`, point
  event, a0 = intern word, a1 = byte length) at the `dict::resolve` call sites
  inside `push_word` — fires only on memo misses, so the event count *is* the
  distinct-resolution count. Zero-cost when the trace feature is off, as
  always.
- **Allocation contract.** The memo grows to the high-water of distinct
  strings per finalize, then stabilizes — same sanction class as the sink and
  COLT pools. Extend the alloc gate's query set with a projection returning a
  string column across rotating params: two warm cycles, then zero
  allocations/deallocations across four more (the memo and buffer must both be
  at capacity).
- `execute_collect`/`explain`/`profile` inherit the fix for free (they all
  route through `finalize`).

## Non-goals

A cross-execution or cross-generation resolution cache (intern ids are stable,
but a persistent cache is an unbounded-memory policy decision — out; the
per-finalize memo already collapses the per-row tax, which is the measured
problem). Dictionary layout changes. Caching in `Snapshot::scan` (the ETL
surface is a stream, not a hot path).

## Passing criteria

- Trace-based test (obs lane): a query emitting K = 1,000 rows sharing exactly
  1 distinct string produces exactly **1** `dict_resolve` event; a query
  emitting rows over D = 16 distinct strings produces exactly **16**; a second
  execution of the same prepared query produces the same counts again (the
  memo clears per finalize — no stale offsets).
- Dedup is real: after the K-rows/1-string execution,
  `ResultBuffer` byte length == the one string's length (test via the public
  buffer API: sum the distinct strings' lengths and compare).
- Correctness: existing result-content tests pass unchanged. The cross-tag
  trap was resolved by BOTH available means and documented on `ResolveMemo`:
  intern ids mint from one shared counter (`dict::intern` reads
  `dict_next_id` for strings and bytes alike), so words are tag-disjoint by
  construction — and the memo keys on `(word, tag)` anyway, so tag
  disambiguation never depends on that allocation detail.
- The extended alloc gate passes in release. Full `verify` S test green.
  `scripts/check.sh` green.
