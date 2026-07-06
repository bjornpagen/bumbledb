# Kùzu — Vision: what every competent GDBMS should do (blog, part 1)

Source: https://kuzudb.github.io/blog/post/what-every-gdbms-should-do-and-vision/
(kuzudb.com offline; GitHub Pages mirror) — fetched 2026-07-06.
Company: Kùzu Inc. (Univ. of Waterloo spinout, Semih Salihoğlu) ceased
operations mid-2025; project archived, community forks continue.

## Positioning
- "Read-optimized analytical DBMSs for modeling and querying application
  data as a graph"; "GDBMSs are relational in their cores" — compile to
  relational operators over tuples.
- Embeddable-library strategy explicitly modeled on DuckDB ("DuckDB
  revolutionized tabular data science; Kùzu aims to fill the same gap for
  graph analytics").

## The five features every competent GDBMS needs (their list)
1. Predefined pointer-based joins (join indices / adjacency lists, dense
   integer node ids).
2. Many-to-many growing joins → factorization + WCOJ.
3. Recursive joins (Kleene star) — "objectively harder" in SQL.
4. Schema querying (type() over relationships).
5. Semi-structured/URI-heavy data (RDF-ish flexibility).
- Plus the CWI list of 12 modern techniques, of which Kùzu integrates 11
  (columnar, vectorized, compression, buffer mgmt, ...).

## Relevance to bumbledb
- Kùzu = accept all five demands + 11 techniques; bumbledb = refuse
  everything but typed conjunctive queries over BCNF sets and push the
  remainder (WCOJ-class execution, columnar reads, embedded, single
  writer) to the limit.
- Overlap in what both kept: dense integer ids, columnar + vectorized,
  beyond-binary joins, embeddability — independent confirmation those
  are the load-bearing choices for m-n workloads.
