# LMDB — The Lightning Memory-Mapped Database (Howard Chu, mdb-paper)

Source: https://www.openldap.org/pub/hyc/mdb-paper.pdf — fetched 2026-07-06

## Design
- Single-level store: the whole database file memory-mapped; reads are
  pointer dereferences into the map — zero-copy, no app-level cache, the
  OS page cache is THE cache.
- Copy-on-write B+tree: committed pages immutable; writers copy paths to
  fresh pages; two alternating meta pages carry root pointers.
- MVCC, single writer + N readers: readers take a snapshot at txn start,
  never block or get blocked; writer serialization by design.
- NO write-ahead log: consistency from CoW + write ordering + synced meta;
  crash = the previous committed root is simply still there. "Durability
  without explicit recovery" — no replay, ever.
- Free-list ("freeDB") recycles old pages once no reader pins them —
  append-only behavior without unbounded growth.

## What bumbledb inherits from this (docs/architecture/40-storage.md)
- The entire crash-safety story: one fsync'd commit = one consistent
  root; kill -9 tests exercise but never "recover".
- MVCC generational snapshots = what makes images Arc-immutable per
  generation and lets readers pin old generations for free.
- Single-writer = the writer mutex; map-size ceiling (4GB today) is the
  L-scale blocker; F_FULLFSYNC on macOS is where commit latency lives.
- The cost side: CoW B+tree ~4KB pages + per-entry node overhead is part
  of the 420 B/fact resting footprint.
