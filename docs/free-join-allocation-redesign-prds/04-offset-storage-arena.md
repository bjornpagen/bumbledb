# PRD 04: Offset Storage Arena

## Purpose

Delete per-child offset vector allocation by storing offsets in a compact arena.

## Required Design

Offsets must be represented as:

- implicit range for full unfiltered sources;
- singleton for one offset;
- arena range for multiple offsets.

## Required Work

- Add one `Vec<u32>` or `Vec<usize>` offset pool to `ColtArena`.
- Add APIs to append offset slices and return `OffsetRange`.
- Add APIs to iterate offsets from `Range`, `Singleton`, and `Offsets` without allocating.
- Convert new arena node tests to use these APIs.

## Passing Criteria

- No new `Vec<usize>` is allocated per child node in the arena path.
- A duplicate-heavy force fixture stores singleton children without allocating child offset vectors.
- A many-offset child fixture stores one range in the arena pool.
- A full source with no filters uses an implicit range and stores no explicit offsets.
- Global gates pass.
