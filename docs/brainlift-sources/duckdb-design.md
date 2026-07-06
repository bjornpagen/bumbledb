# DuckDB — Design and positioning

Sources: https://duckdb.org/why_duckdb + Raasveldt & Mühleisen, "DuckDB: an
Embeddable Analytical Database" (SIGMOD 2019 demo),
https://mytherin.github.io/papers/2019-duckdbdemo.pdf — fetched 2026-07-06

## Positioning
- "SQLite for analytics": in-process, no server, single-file native format,
  zero external dependencies, two-file amalgamation-style distribution, MIT.
- The stated motivation: SQLite "performs poorly on OLAP tasks due to its
  tuple-at-a-time execution model" — the embedded-OLAP gap.

## Engine
- Columnar-vectorized execution: "queries are still interpreted, but a
  large batch of values (a 'vector') are processed in one operation" —
  interpretation amortized over vectors (DataChunk = columnar batch),
  cache-friendly, SIMD-able, low per-tuple overhead.
- Morsel-driven parallelism (multi-core). Full SQL surface with parser +
  cost optimizer.
- Custom bulk-optimized MVCC for ACID; native format has secondary (ART)
  indexes for point lookups; larger-than-memory via buffer management and
  spilling; lakehouse-format reach.
- Testing: "millions of queries, adapted from SQLite, PostgreSQL, and
  MonetDB" + TPC-H/DS.

## Relevance to bumbledb
- DuckDB proves the embedded-columnar-vectorized bet at industrial scope;
  it is what "accept the whole problem" looks like for OLAP (MVCC, buffer
  manager, compression, parallelism, full SQL — the same machinery classes
  Kùzu carries for graphs).
- bumbledb occupies the seam DuckDB leaves: durable fsync-per-commit
  writes + commit-time relational integrity as the system of record for
  an app, with OLAP-class reads at app scale — but RAM-bound, single-core,
  conjunctive-only, one user.
- Both share the vectorized-over-interpreted thesis; bumbledb removes the
  interpreter entirely (monomorphized executor) at the cost of a fixed
  query algebra.
