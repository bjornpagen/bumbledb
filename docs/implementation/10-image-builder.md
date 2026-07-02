# PRD 10 — The Relation Image Builder

Authority: `docs/architecture/40-storage.md` (image section: full-width, dense
ordinals, 128-byte alignment, staggered bases, corrupt = hard error),
`00-product.md` (machine model).

## Purpose

Decode one relation, once, into the immutable SoA columnar image that execution runs
on.

## Technical direction

- `image` module. `RelationImage { columns: per-field SoA vectors, row_count }`,
  built by `build(&ReadTxn, &Schema, rel) -> Result<Arc<RelationImage>>` from one
  PRD 09 `scan`: 1-byte fields decode into `Vec<u8>`-backed columns, 8-byte fields
  into `Vec<u64>`-backed columns storing the **encoded** big-endian-decoded u64 word
  (one canonical in-memory word form: the byte-order-normalized encoding, so
  comparisons are integer compares; document the word form precisely — for I64 the
  sign-flipped biased word preserves order under u64 compare, which is what kernels
  use).
- Memory: one arena allocation per image sized up front from `row_count` (`S`); column
  base addresses **128-byte aligned and staggered** — successive column bases offset
  by `(k * 128) mod 16384` with k odd per column index, so no two bases are congruent
  mod 16 KiB (cite Category 5 of the hardware reference in a comment).
- Positions are dense scan ordinals 0..row_count; row_ids are not stored (nothing
  reads them — architecture rule 3).
- Any decode error (PRD 09 corruption, dangling intern id is NOT checked here — ids
  are opaque words at this layer) aborts the build with the error.
- The image is immutable after build: fields private, accessor methods return `&[T]`
  slices; `Arc` is the sharing unit.

## Non-goals

Caching (PRD 11). Filtering (PRD 12). String materialization (ids stay ids until
result decode, PRD 25).

## Passing criteria

- Unit tests: built columns equal per-field decode of the same scan; positions dense
  under row_id holes; alignment assertions (every column base % 128 == 0; no two bases
  congruent mod 16384 for a 12-column fixture relation); u64-word order matches logical
  order for I64 columns (sorted sample property); zero-row relation builds an empty
  image; corruption from the scan propagates as an error.
- Global commands green.
