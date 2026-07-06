# SQLite — Query optimizer (optoverview.html)

Source: https://www.sqlite.org/optoverview.html — fetched 2026-07-06

## Joins: nested loops, always
- "SQLite implements joins as nested loops." Default order = FROM order;
  reordered by the Next Generation Query Planner (NGQP), a polynomial-time
  graph algorithm — "plan queries with 50- or 60-way joins in a matter of
  microseconds." Cost-based, minimizes estimated work.
- One index per table per query (exception: the OR-union optimization).
- Automatic transient indexes ("query-time indexes") built O(N log N) when
  expected lookups > log N — SQLite's stand-in for hash joins.

## Predicate machinery (the language surface bumbledb refused)
- Indexable term forms include =, IS, <, <=, >, >=, IN(list/subquery),
  IS NULL, LIKE/GLOB.
- LIKE 'x%' compiles to a range: virtual terms column >= 'x' AND < 'y'
  (six preconditions incl. collation + no leading wildcard); '%x%' cannot
  use an index at all — it is always a scan.
- OR: same-column ORs rewrite to IN; else per-subterm index lookups
  unioned by rowid (cost-gated) — SQL's disjunction is UCQ underneath.
- Skip-scan (needs ANALYZE, ~18+ duplicates), covering indexes ("can make
  many queries run twice as fast" by skipping the rowid lookup),
  BETWEEN → two virtual range terms, MIN/MAX single-lookup, subquery
  flattening (28 conditions), push-down, coroutines.
- Statistics: sqlite_stat1 selectivity per index; STAT4 histograms only
  with special compile flags. Default estimates are coarse.

## Load-bearing contrasts for bumbledb
- Their whole optimizer exists to order nested loops over B-trees; the
  worst case of pairwise plans on cyclic/skewed inputs is the gap WCOJ
  attacks (our triangle/spread/graph numbers).
- ORDER BY/GROUP BY ride index order — SQLite fuses sorting into access
  paths; bumbledb has no ordering at all (host sorts).
