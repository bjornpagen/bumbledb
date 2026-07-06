# SQLite — File format (fileformat2.html)

Source: https://www.sqlite.org/fileformat2.html — fetched 2026-07-06

## Pages and B-trees
- Page size: power of two, 512–65,536 (default 4096). 100-byte database
  header on page 1. B-tree page header: 8 bytes (leaf) / 12 (interior);
  2-byte cell pointer array sorted by key; cell content grows from page end.
- Four page types: interior/leaf × table/index. Table B-trees keyed by
  64-bit rowid; WITHOUT ROWID tables are index B-trees keyed by the record.

## The record format (why SQLite rows are small)
- Record = varint header size + one varint serial type per column + values.
- Varints: 1–9 bytes, big-endian 7-bit groups.
- Serial types: NULL=0 bytes; ints stored in 1/2/3/4/6/8 bytes by magnitude;
  constants 0 and 1 stored in ZERO bytes (types 8/9); floats 8 bytes;
  TEXT/BLOB length encoded in the serial type itself ((N-13)/2 odd = text).
- Consequence: a row of small integers costs ~1 byte per column plus ~1
  header byte per column — varint compression at rest, paid for with
  per-row header decode on every read.
- Indexes on rowid tables store (indexed columns..., rowid) as a record;
  every secondary index entry re-encodes the indexed values.

## Durability structures
- Rollback journal (original-page copies + checksums) or WAL: 32-byte WAL
  header, 24-byte frame header per page write, cumulative Fibonacci-weighted
  checksums, salts rotated per checkpoint; -shm wal-index for frame lookup.

## Size ledger (for the 4.7x store comparison)
| element | bytes |
|---|---|
| leaf page header | 8 |
| cell pointer | 2 |
| record header | ~1/column varint |
| small int value | 0–4 typical |
| rowid key | 1–9 varint |
| WAL frame overhead | 24/page |
- vs bumbledb: fixed 8-byte words (no varints, O(1) slicing), plus M(37B
  key)+U+R materialized entries per fact — decode-free reads bought with
  ~4.4–4.7x resting bytes (measured S/M scales).
