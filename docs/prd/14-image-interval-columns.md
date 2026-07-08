# PRD 14 — Images: interval columns

**Depends on:** 01.
**Modules:** `crates/bumbledb/src/image.rs`, `crates/bumbledb/src/image/` (decode, view, cache untouched except widths).
**Authority:** `docs/architecture/50-storage.md` (§ image cache — interval decode rule).

## Goal

An interval field decodes into **two parallel 8-byte columns** (start, end). The
image layer has no 16-byte column kind; every existing kernel shape applies
unchanged.

## Technical direction

1. The relation image's column layout is driven by a per-relation
   **field→column map**: `FieldId → ColumnSpan { first_column: u16, width: ColumnWidth }`
   where an interval field's span covers two consecutive 8-byte columns and every
   other field one column of its width. Build the map once from the schema layout;
   export it — the plan witness (field→column maps in `ValidatedPlan`) consumes
   spans, not raw indices, from now on.
2. Decode: the `F`-scan decode loop copies an interval field's first 8 bytes into
   column `first` and second 8 into `first + 1`. The stored halves are already
   order-preserving big-endian words; store them **as u64 words in native order
   for comparison** exactly the way existing 8-byte columns are handled (whatever
   the current decode does for U64/I64 — mirror it per half; the I64 sign-flip is
   already inside the encoding, so both halves compare as u64. Assert this with a
   test, do not re-derive it in code).
3. Corruption check in decode: `start_word < end_word` per row (the encoded
   comparison — u64 compare on the words); violation aborts the image build with
   the corruption error (`50-storage.md`: hard error, never a skip).
4. The width-dispatch panics (`image.rs:116/:130`-style "wrong-width column"
   invariants) extend to the new map; per-column distinct counts, alignment,
   `SET_STRIDE`/pitch padding logic are unchanged (two more 8-byte columns are
   just columns).
5. Filtered views need no change beyond consuming `ColumnSpan` (the new filter
   kinds from PRD 13 read two columns; the view machinery already evaluates
   multi-column filters — extend the filter evaluator to the three interval
   shapes + `PointIn`/`AnyPointIn`, scalar path first; NEON in PRD 17).

## Out of scope

NEON filter kernels (17), COLT levels (COLT keys interval columns as ordinary
8-byte levels only if a plan ever binds them — value-equality joins on interval
vars probe both columns as a two-level key; note this for PRD 15/16, no image
work needed).

## Passing criteria

- `[shape]` No 16-byte column width exists in the image layer; `ColumnSpan` is
  the only field→column interface.
- `[test]` Image build over a relation with `(u64, interval<i64>, enum)` yields
  four columns with the expected words per row (golden values, including a
  negative start — asserting the sign-flip lands in the word such that u64 word
  order equals i64 order).
- `[test]` A hand-corrupted `F` value with inverted interval halves aborts the
  image build with the corruption error.
- `[test]` A `PointIn` filter over a built image survives exactly the rows whose
  interval contains the point (boundary rows: point == start survives, point ==
  end does not).
