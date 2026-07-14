import Bumbledb.Query.Aggregates

/-!
# Exec/Sweep — the sweep as a fold (Level 1, PRD 06)

The sweep modeled as a fold; coverage and Pack correctness under the
disjoint+ordered premise — the `DisjointDeterminantProof` theorem.
Algorithmic essence only: the mechanism fence bans batching, buffers,
scratch, SIMD, pipelining, memos, and LMDB from this file forever.

This file is a scaffold stub (PRD 01): the theorems land in PRD 06.
-/
