# PRD 05 — Aggregates: folds, measure, Pack, Allen — the boundary contracts as theorems

**Depends on:** 04.
**Modules:** `lean/Bumbledb/Query/Aggregates.lean`, `Countermodels.lean`.
**Authority:** the aggregate contracts currently scattered across
`20-query-ir.md` (checked sums, empty-global, group keys) and the
engine's pinned behavior (the signature-table test, the Pack sweeps,
the Allen exhaustive suites — the theorems here are what those pins
sample).
**Representation move:** Level 0 for aggregation — the contracts stop
being prose bullets and become the fold laws.

## Context (decided shape)

Definitions:
- `Group` — the non-aggregate head terms as the grouping key; grouping
  as the fibering of the distinct binding set (from 04's answers
  machinery, pre-projection).
- `AggOp := count | countDistinct v | sum v | min v | max v | pack v |
  argMax v k | argMin v k | measureFold op v` — the executable ops.
- `checkedAdd/checkedSum` (port) — bounded arithmetic with `none` on
  out-of-range (the Overflow(Aggregate) spec).
- `pack : List (Interval α) → List (Interval α)` — sort by start,
  coalesce overlapping-or-adjacent, emit maximal segments.
- `classify : Interval α → Interval α → Basic` — the 13-relation Allen
  classifier, DEFINED (endpoint comparisons), with masks as 13-bit
  finite sets.

Theorems:
1. `agg_over_distinct_bindings` — every op folds the DISTINCT binding
   set of its group (set semantics through aggregation; Bridge: the
   dedup regimes + DistinctWitness elision, whose licence PRD 07
   proves).
2. `empty_global_no_answer` — a global aggregate over the empty
   binding set yields the empty answer set (not a zero row; port
   `count_empty` etc. and REPLACE their SQL-flavored `SUM [] = 0`
   reading with the engine's actual contract — the artifact modeled
   `sum [] = 0`; the engine emits NO answer: the model follows the
   engine, and the divergence from the artifact is recorded in the
   module doc).
3. `checkedSum_sound` (port `checkedAdd_sound`) — success implies the
   mathematical sum within bounds; the i128-accumulator argument
   stated abstractly (fewer than 2^64 terms cannot overflow the wide
   accumulator; only finalization narrows).
4. `measure_fold_laws` — Sum/Min/Max over `measure v` inherit 02's
   ray refusal: a ray in the group makes the query erroneous, never a
   value (model errors as `Option`-poisoning at the group level —
   MeasureOfRay's spec).
5. `pack_canonical` — pack output is sorted, pairwise-disjoint,
   non-adjacent, and maximal.
6. `pack_extensional` — ⋃ points (pack ivs) = ⋃ points ivs (the
   support-union law; Bridge: interval/sweep.rs, the r18 suites).
7. `pack_adjacency` — `[0,2), [2,5)` coalesce (half-open adjacency
   continues a run) — stated as a lemma, since it is THE boundary the
   docs kept explaining.
8. `allen_jepd` — classify is jointly exhaustive and pairwise disjoint
   over nonempty intervals (the engine's 8192-mask exhaustive suite is
   this theorem's sample; prove it once, generally).
9. `allen_converse_involution` — converse ∘ converse = id on masks;
   `classify (swap) = converse (classify)`.
10. `argmax_ties_all_kept` — Arg restriction retains every extreme-
    attaining binding, deduplicated as answers (the ArgMax contract).

## Technical direction

`pack` and `classify` are DEFINED functions (computable — PRD 13
evaluates them); their specs are the theorems. Port the artifact's
aggregate section but note its `sum []` divergence honestly (item 2).
Allen: prove JEPD by decidable case analysis over the endpoint
trichotomies — elementary but fiddly; if the general proof resists,
narrowing to the two concrete element domains is the recorded
fallback (law 5), NOT a sorry.

## Passing criteria

- `[shape]` All ten theorems checked; `pack`/`classify` computable
  (`#eval`-able on literals — include two `example` evaluations);
  zero sorry/axioms; `scripts/lean.sh` 0.
- `[shape]` The artifact-divergence note (empty-global) in the module
  doc, citing the engine's contract as the authority.
- `[gate]` CI green.

## Doc amendments

None yet — PRDs 11/12 delete against these names.
