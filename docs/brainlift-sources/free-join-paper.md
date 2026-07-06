# Free Join — "Free Join: Unifying Worst-Case Optimal and Traditional
# Joins" (Wang, Willsey, Suciu — SIGMOD 2023)

Source: in-tree, docs/free-join-paper/arXiv-2301.10841v2 (full LaTeX).
arXiv: https://arxiv.org/abs/2301.10841

## The claim
- WCOJ emerged as asymptotically faster on cyclic queries but "less
  efficient than the old paradigm... on the typical acyclic queries
  found in practice"; systems (Umbra, EmptyHeaded, Graphflow, Kùzu)
  respond with HYBRIDS: WCOJ for cyclic subparts, binary joins elsewhere.
- Free Join instead UNIFIES the paradigms: one plan space containing
  both binary plans and Generic Join as extreme points ("a new type of
  plan, a new data structure (which unifies the hash tables and tries),
  and a suite of optimization techniques"). binary2fj converts any
  left-deep binary plan; factor() hoists toward GJ.

## COLT
- Column-Oriented Lazy Trie: the unified structure — a hash trie forced
  level-by-level, only for keys actually probed; unforced levels remain
  column offsets. Generalizes both the hash table (1 level) and the
  full trie (all levels).

## Evaluation (their Rust system vs binary join and GJ)
- On the JOB/IMDb benchmark: geometric-mean speedup 2.94x over binary
  join and 9.61x over Generic Join (paper macros imdbavgfjbj/imdbavgfjgj);
  a few queries slightly slower than binary join.
- LSQB: up to 15.45x (cyclic q3) and 13.07x (acyclic q4) over binary
  join; up to 4.08x over GJ.
- COLT ablation: geomean speedup with maxima of 11.01x/26.29x over
  eager alternatives.

## What bumbledb takes and changes (docs/architecture/30-execution.md)
- Takes: the plan formalism (nodes/subatoms/covers), binary2fj+factor,
  COLT laziness, vectorized batched execution.
- Deviations (documented D1-D5): LMDB storage + generational images
  (paper assumes in-memory columns); no DuckDB optimizer (own DP);
  cover rule restricted to exactly-new-vars (wrong-results hazard under
  dynamic choice, found + pinned by test); selections as prepended trie
  levels; D2 suffix-skip exploiting SET semantics (paper is bag-ish);
  batch-of-128 two-phase probing tuned for Apple Silicon MLP.
