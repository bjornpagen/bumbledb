# Kùzu — Worst-case optimal joins (blog, part 3 of the CIDR 2023 series)

Source: https://kuzudb.github.io/blog/post/wcoj/ (kuzudb.com is offline;
GitHub Pages mirror) — fetched 2026-07-06. Company ceased operations 2025;
domains lapsed — archive: https://web.archive.org/web/*/kuzudb.com/blog/wcoj.html

## The problem
- Since Selinger 1979, DBMSs join tables pairwise. On cyclic many-to-many
  queries the intermediate results are "polynomially larger" than outputs:
  the triangle example has 1M two-paths on a 2001-edge graph with at most
  ~89.5K triangles.
- AGM bound (2007/2008): max output = N^(rho*), rho* = fractional edge
  cover number. Triangle: rho* = 1.5 → Θ(N^1.5); binary plans hit O(N^2).

## Generic Join
- Join COLUMNS, not tables: pick a variable order, extend tuples one
  variable at a time via multiway intersections of adjacency lists —
  never materialize the 2-path intermediate.

## Kùzu's integration
- "Multiway ASPJoin" operator (accumulate → k-2 build phases with
  semijoin filters → probe with sorted-list intersections), mixed with
  binary hash joins by the optimizer; WCOJ complements binary plans,
  used for cyclic parts.

## Numbers
- web-BerkStan (685K nodes, 7.6M edges), triangle count, M1 Air:
  WCOJ 1.62s vs binary joins 51.17s — 31.6x, 41M output triangles.
- "In larger densely cyclic queries, binary join plans just don't work."

## Relevance to bumbledb
- Same lineage as Free Join; bumbledb's triangle family = this exact
  stress (ours: 0.17 ratio vs SQLite's pure nested loops at S).
- Kùzu picks WCOJ per-cyclic-subplan; Free Join (our engine) unifies
  binary and WCOJ in ONE plan formalism (binary2fj + factor), so there
  is no operator switch to mispick.
