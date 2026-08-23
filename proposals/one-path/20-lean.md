# 20 — The Lean consequence

The Lean corpus proves what the system spends. The system no longer spends
the footprint: the model and the theorems about it die with their subject,
and the two theorems the system still spends are kept and re-homed.

## Dies from lean/Bumbledb/Txn/Footprint.lean

- The footprint model: `FKey`, the F/K/C/W classes, `keyDet`, `cTouch`,
  `wTouch`, `KeyDisjoint`, footprint-of-net-delta.
- The W interval machinery: `WInterval`, `wIntervalTest`,
  `wIntervalTest_admitsMeasure`, `nodup_split_by`, `measureAtMost_union`.
- **L6** (footprint soundness), **L7** (footprint stability, acceptance
  form, `republish_verdict_stable`), **L8** (footprint-disjoint
  commutativity, `apply_commutes`) — theorems whose hypothesis is a
  predicate over a structure the wire no longer carries.
- The relaxed family built to license the fast path: `TestedDisjoint`,
  `capacity_stable_tested`, `L7_tested`, `L8_tested`, `KeyDisjoint.tested`.
- The two countermodels in lean/Bumbledb/Countermodels.lean
  (`footprint_rejection_not_stable` at 886-921,
  `commute_cell_exclusion_load_bearing`): they refute claims about the
  deleted design and pin the strictness of a hypothesis that no longer
  exists. Their content is preserved by the deletion record in 10 and by
  git history; countermodels of dead designs are residue.

## Survives, re-homed

- **L9 — component locality.** Its content is that statements never span
  braid components, so cross-braid interleavings are semantically
  invisible; its current statement is phrased through footprint
  disjointness. Restate it directly over the statement graph and the
  judgment: a statement's obligation instances read and write only
  relations inside one component, so judgment and application over one
  braid are invariant under any other braid's history. Same theorem, the
  footprint vocabulary removed. It lives wherever the braid derivation's
  proofs naturally sit — `Txn/Braids.lean` if a new home reads cleaner
  than a gutted `Footprint.lean`; the file name follows the content.
- **L10 — replay idempotence.** Untouched: its statement (contained
  batches apply to the identical state, empty effective delta, accepted
  verdict, no generation advance) never mentioned footprints, and its
  consumers — apply's absorption arm, the replica's pending resolution,
  the writer's pending clear, the whole recovery design — are all alive.
  `opDisposed`, `applyOps_disposed`, `commitOps_disposed`, `replay_heals`
  stay as they are.

## The Bridge ledger

The five rows added for L6–L10 become two: the L6, L7, and L8 rows die
with their theorems; the L9 row's mechanism stays the braid derivation and
its instrument stays the braid goldens plus the interleaving-convergence
gate; the L10 row is untouched (apply + both recovery sites; the
every-prefix double-apply matrix). The asserted `ledger.length` moves
**107 → 104**, and the census re-derives the same count by grep.

## Gates

`bash scripts/lean.sh` whole — lake build with zero sorry/axiom, the
placeholder battery, spec-census (104 rows, token count re-derived), the
conformance corpus, the three-way comparator. The deleted names must be
absent from the census token roster; a surviving stray citation of L6/L7/L8
anywhere in lean markdown or driver comments is a census failure, not a
cleanup note.
