# Kùzu — Factorization (blog, part 2 of the CIDR 2023 series)

Source: https://kuzudb.github.io/blog/post/factorization/ (kuzudb.com
offline; GitHub Pages mirror) — fetched 2026-07-06.

## The idea
- Factorization compresses INTERMEDIATE results of m-n joins as symbolic
  Cartesian products, exploiting conditional independence ("given a fixed
  b value, all a's and c's are conditionally independent") — Dan Olteanu's
  group's theory (ICDT test-of-time).
- 2-hop example: 20,000 flat tuples become 400 stored values
  (2*(100+100) instead of 2*100*100).

## Mechanism
- Vectorized processor passes factorized vectors: flat vectors (single
  value via curIdx) x unflat vectors (value sets); a tuple batch denotes
  the Cartesian product of its vector sets.
- Wins: ~50x less data movement in the example; 200 vs 20,000 predicate
  evaluations; count(*) computed as 100x100 without enumeration; min/max
  over one factor costs |factor| comparisons.
- Sideways information passing: build side hands a nodeID bitmap filter
  to probe-side scans — sequential I/O without full-file scans.

## Numbers (LDBC SNB SF100, 2-hop aggregation vs Umbra)
- 10% selectivity: Kuzu 3.89s vs Umbra 230.35s; 100%: 31.98s vs timeout.

## Relevance to bumbledb
- The bag-semantics dual of our set-semantics D2 skip: factorization
  counts multiplicities symbolically; we stop at the first witness
  because sets need no second one. Each engine made its multiplicity
  story structural, in opposite directions.
- Their "count without enumerating" beats us on COUNT-heavy m-n queries;
  our skip beats enumeration on distinct-projection queries.
