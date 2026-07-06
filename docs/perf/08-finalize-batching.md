# PRD 08 — Finalize batching: materialize results by column, not by cell

## Purpose

`finalize` walks sink rows calling `ResultBuffer::push_word` per cell —
a `ValueType` match, a `Result` return, and a byte append, per cell. At
50k+ output rows this is a real tax: skew 617.5 µs (36% of the family's
execute), spread 579.9 µs, chain 14.8 µs. The rows are already dense u64
words in sink memory; all-word outputs can move as bulk copies, and
interned columns can resolve column-wise through the memo.

## Technical direction

`api/prepared.rs` (`finalize`, `ResultBuffer`) + small sink-side accessors.

- **Classify the find row shape once per prepare** (not per finalize):
  `all_words` = no `String`/`Bytes` columns in `finds`. Store on the
  prepared query.
- **All-words fast path.** The output cell encoding for word types is the
  word itself (u64/i64-biased/enum/bool widths — check `push_word`'s
  actual byte layout and match it exactly; if enum/bool cells are stored
  narrower than 8 bytes, the "bulk" copy is a per-column typed loop, not
  a memcpy — implement per-column loops over the row-major sink rows
  writing the buffer's cell encoding directly, with capacity reserved
  once from `rows × row_stride`). For `ProjectionSink`, iterate
  `seen.iter()` and copy per column; for `AggregateSink`, keep
  `finalize_into`'s per-group row assembly but write into the buffer
  through the same per-column cell writers, reserving once. The
  `ResultBytesOverflow` check moves to the single reservation (total size
  computed up front — one checked multiply/add chain instead of per-cell
  checks).
- **Interned columns.** For rows containing `String`/`Bytes`: resolve
  column-wise — first pass copies word columns as above; second pass per
  interned column walks rows resolving through `ResolveMemo` (each
  distinct intern hits LMDB once per finalize, as today — semantics and
  the DICT_RESOLVE event unchanged). This keeps the memo hit loop tight
  and branch-monomorphic per column instead of re-matching type per cell.
- **Error semantics unchanged**: `Overflow { find }` from aggregate
  finalization must carry the same find index; a failed intern resolve
  surfaces the same typed error. The buffer must be left `clear()`-safe on
  error (document: partially-written buffers are discarded by the caller
  as today — verify and add a test asserting a clean re-execute after an
  Overflow error in the batch path).
- **Sort/ordering**: none exists (results are sets); do not introduce any.
- **Tests**: result-byte equivalence — run every read family query shape
  (word-only, string-bearing, aggregate, empty result) through old and new
  finalize (`#[cfg(test)]` reference retained per the 00 law) asserting
  byte-identical `ResultBuffer` contents and identical errors at the
  overflow boundary.

## Passing requirements

1. Equivalence tests green; functional gates green.
2. Measured (traced samples vs baseline):
   - skew `finalize` ≤ 150 µs (baseline 617.5); skew p95 ≤ 1,400 µs.
   - spread `finalize` ≤ 200 µs (baseline 579.9).
   - chain `finalize` ≤ 6 µs (baseline 14.8); fk_walk `finalize` ≤ 1.2 µs
     (baseline 2.2 — dict-resolve bound; the gate is the word-column part).
   - No family regresses >5%.

## Out of scope

Sink internals (01–06), result *ordering* (none exists — stays none),
`Row`/`Cell` read-side API (unchanged), dictionary storage.
