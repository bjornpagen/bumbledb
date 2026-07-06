# JOB — "How Good Are Query Optimizers, Really?" (Leis et al., VLDB 2015)

Source: https://www.vldb.org/pvldb/vol9/p204-leis.pdf — fetched 2026-07-06
(PDF extract was partial; structural facts cross-checked against the
paper's abstract/intro.)

## What JOB is
- 113 queries over 33 structural templates against the REAL IMDb dataset —
  chosen precisely because "real-world data exhibits complex correlations"
  that uniform synthetic benchmarks (TPC-H) hide.
- Predicate surface is SQL's: LIKE patterns, IN lists, OR, IS NULL, ranges,
  and every query projects MIN() over (mostly varchar) columns.

## Findings
- Cardinality estimation is the dominant failure: errors grow roughly
  exponentially with join count, routinely reaching orders of magnitude;
  PostgreSQL and commercial optimizers alike.
- Cost-model quality matters far less than cardinality quality; perfect
  cost functions cannot rescue bad cardinalities.

## Relevance to bumbledb
- The measurement intent (correlated skewed data punishes join orders) is
  exactly the pressure our `joins` scenario reconstructs in-language; the
  predicate surface (LIKE/OR/NULL/string-MIN) is exactly what the design
  refuses — hence the standing ruling (00-product): "JOB may return as a
  stress suite, never as the ratchet."
- Our own estimator honesty numbers (worst est/actual pinned <= 3.3x on
  acyclic ledger families; 5,172x documented tier for the cyclic
  triangle) are the same phenomenon JOB measures, priced per family in
  the report rather than hidden in a planner.
