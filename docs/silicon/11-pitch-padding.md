# PRD 11 — Pitch padding: kill the stagger rule before it kills a scan

## Purpose

bumblebench exp 10 found a live layout bug in our own rule book. The
feared 16 KB set-aliasing pathology the stagger rule was written against
does not meaningfully exist (≤ 1.55× on lockstep scans, 7.3× only under
full serialization) — but the rule itself (offset parallel columns by odd
multiples of 128 B) CREATES a real one: stream-prefetch trackers alias on
the low bits of the 16 KB page number, so power-of-two column pitches
with 1–3-line staggers run 4–6× slower on DRAM-tier lockstep scans
(8.13 vs 1.78 ns/row measured, 8 columns). The cure is one page of pitch:
`pitch = span + 16 KiB`. Four pre-registered discriminators pinned the
mechanism (stagger=16384 fast, stagger=8/32 mild, stagger=64 severe,
+64 K pad fine); DRAM-bank and SLC-set explanations predict the opposite
ordering.

## Technical direction

`crates/bumbledb/src/image.rs` (column region layout at build),
`crates/bumbledb/src/image/view.rs` (column addressing),
`docs/architecture/40-storage.md`.

- **Find the rule.** Locate the stagger implementation (the odd-128 B
  offset applied between column regions at image build) and every
  constant/comment that encodes it. It dies in this PRD.
- **The replacement rule, exactly:** when laying out consecutive
  same-scan column regions, if a column's span (bytes from the start of
  one column region to the start of the next — the effective pitch a
  multi-column scan strides by) lands within ±384 B of a multiple of
  16 KiB AND the span is ≥ 64 KiB, pad the region so the pitch becomes
  `span + 16,384 B`. Below 64 KiB spans the columns are cache-resident at
  scan time and no tracker interference was measured — do not pad (disk
  is not free). All existing 8 B alignment guarantees hold; the pad is
  pure dead space at region tail.
- **Addressing stays derived.** `view.rs` computes column bases from the
  layout header — the pad must flow through the recorded offsets, not
  through recomputed arithmetic (one source of truth; no parallel pitch
  formula in the reader).
- **Prove it in-tree.** Add an `#[ignore]`d evidence test (the
  `image_build_split_evidence` pattern): build an image whose column
  spans are engineered to the pathological geometry (pow-2 span ≥ 1 MiB
  per column, 8 columns), run an 8-column lockstep scan, and measure
  ns/row with the old stagger layout (reconstructed locally in the test)
  vs the new pitch layout — assert ≥ 2× improvement (bumblebench measured
  4.6× in isolation; engine overhead dilutes it; 2× is the honest gate).
- **Fix the doc.** `40-storage.md`: delete the stagger rule text; write
  the pitch rule with the mechanism (prefetch-tracker aliasing on 16 KB
  page-number bits) and the discriminator evidence, citing exp 10. Also
  correct any "128 B cache line" statement in the architecture docs that
  describes the L1D: the durable phrasing is "64 B L1D lines behind a
  128 B L2/SLC/DRAM granule" (layout math in this codebase keys on the
  128 B outer granule — that part was and remains correct).

## Passing requirements

1. Evidence test green (≥ 2× on the engineered pathological scan,
   proxy-bracketed, min-of-5 in-test reps).
2. Measured (vs post-10, min-of-5): scan-heavy families (range, stats,
   spread) hold or improve — the bench stores may not hit the pathological
   geometry at their sizes; if no family moves, the evidence test IS the
   win and `## Result` says so explicitly with the bench-store span table
   (column spans → padded or not).
3. Disk footprint on the bench stores grows ≤ 1% (one page per padded
   column region; the store-suite footprint number is re-recorded).
4. grep gates: no odd-128 stagger constant remains in `image.rs`;
   `40-storage.md` contains "16" KiB pitch rule and no stagger rule;
   architecture docs contain no L1D-128 B claim.
5. Cold decode / `fill_columns` unchanged ±2% (layout reads flow through
   recorded offsets); verify green (2,468 cases through rebuilt images —
   set semantics independent of layout); commit_batch within 5%.

## Out of scope

Any migration (humans own stored-data transitions; the format is
unstable by decree — new builds simply produce the new layout); DMP
pointer-encoding (bumbledb's packed values are tagged indexes, not
canonical pointers — recorded as considered-and-not-applicable, with the
one-line rationale in `## Result`); allocator changes for transient
buffers.
