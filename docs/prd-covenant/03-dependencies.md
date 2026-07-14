# PRD 03 — Dependencies: theories, keys, containment, coverage, equality, partition

**Depends on:** 02.
**Modules:** `lean/Bumbledb/Schema.lean`, `Dependencies.lean`,
`Countermodels.lean` additions.
**Authority:** `30-dependencies.md`'s semantic content (builds its
replacement); the artifact's containment/equality/coverage theorems
(port targets); the constitution's corrected claims (disjoint cover vs
exact partition; key-backed correspondence).
**Representation move:** Level 0 for the dependency theory — the
heart of the covenant. After PRD 11, `30-dependencies.md` is a reading
guide over THIS module.

## Context (decided shape)

Schema.lean:
- `Header`, `Theory` (relations + declared statements), `Instance`
  (a database state: each relation ↦ a finite set of facts), ground
  axioms as sealed finite extensions (`Closed` relations are
  `Instance`-independent constants of the theory).
- `Fact`, projections over field sets, selections as equality-only
  predicates (the accepted σ fragment, stated as a definition — the
  acceptance boundary is part of the model).

Dependencies.lean — definitions:
- `View R φ X` = the selected projected value set (the artifact's
  View, ported).
- `Functionality` (scalar): no two distinct facts agree on the
  determinant projection. `PointwiseKey`: scalar-prefix groups have
  pairwise-disjoint interval point sets (via `Values.points`).
- `Containment A φ X B ψ Y` = `View A φ X ⊆ View B ψ Y` with the
  target-key acceptance premise carried separately (acceptance ≠
  denotation — model both, distinctly, as the validator does).
- `Coverage` — pointwise support inclusion per scalar group (the
  artifact's IntervalContains, ported to in-tree Values).
- `KeyBackedEquality` — the structure (mutual containment + both
  projections keyed), ported.
- `ExactPartition` — target disjointness + two-sided support equality,
  ported (`exactTiling_iff_exactPointPartition` becomes the in-tree
  equivalence).
- `holds : Theory → Instance → Prop` — a committed instance models
  its theory (the final-state judgment's SPEC; Txn consumes this).

Theorems (ported + new; each lands in Bridge):
1. `contains_iff_view_subset` (port).
2. `containsEq_iff_view_ext` (port) + `bare_eq_not_unique`
   countermodel (port).
3. `keyed_eq_unique_correspondence` — existence + uniqueness both
   directions (port of `KeyBackedEquality.unique_target/_source`,
   restated as one bijection-on-σ-subsets theorem; the composite-
   projection generality explicit: determinants are field SETS).
4. `functionality_unique_witness` — a key proves at most one fact per
   determinant tuple; and its non-theorem twin stated as a note: keys
   prove uniqueness, never existence.
5. `pointwise_key_disjoint` — per-group pairwise point disjointness.
6. `coverage_is_support_inclusion` (port) + `one_way_overhang`
   countermodel (port of the [0,10)/[0,20) overshoot — the tiling
   over-read's killer, now in-tree).
7. `mutual_coverage_support_equality` — both directions ⟹ equal point
   supports per group.
8. `exact_partition_iff` — disjointness + mutual coverage ⟺ exact
   partition (the five-statement idiom's theorem; Bridge row: cookbook
   recipe 26's commit matrix).
9. `selection_monotonicity` — source strengthen / target weaken (port
   both directions + the invalid-converse notes).
10. `no_closure` stated as a MODEL NOTE, not a theorem: acceptance
    resolves exact field sets; logical superkey implication is true
    (prove the one-line implication) but deliberately UNSPENT by
    acceptance — the model records the gap between entailment and
    acceptance explicitly (this is the D1 evaluation's future seat).

## Technical direction

Port from `docs/formal/GPT55DependencyTheory.lean` by STATEMENT — the
imports it relied on do not exist; rebuild the minimal base (the View/
key/coverage definitions) in-tree, then re-prove. Keep instances
abstract (`Instance` as a function to `Set Fact` with a finiteness
token carried where a theorem needs it) — finiteness premises named,
not ambient. Names obey the language law (functionality, determinant,
containment, ground axiom, answer).

## Passing criteria

- `[shape]` All ten items present and checked; the two countermodels
  in `Countermodels.lean`; zero sorry/axioms; `scripts/lean.sh` 0.
- `[shape]` Acceptance and denotation are DISTINCT definitions (grep:
  the target-key premise is a hypothesis, not baked into Containment).
- `[shape]` The entailment-vs-acceptance note (item 10) present with
  the superkey implication proved and marked unspent.
- `[gate]` CI green.

## Doc amendments

None yet — PRD 11 deletes against these names.
