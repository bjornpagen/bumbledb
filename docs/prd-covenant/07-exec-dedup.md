# PRD 07 — Dedup, proved: the seen-set and the two elision licences

**Depends on:** 04 (answers, rule union).
**Modules:** `lean/Bumbledb/Exec/Dedup.lean`, `Countermodels.lean`.
**Authority:** the sink-owned union (exec/sink.rs — "the sinks are
where union lives"); the two witness types whose licences become
theorems: `DistinctWitness` (bound-fields-cover-a-key ⟹ seen-set
elision sound) and `DisjointWitness` (provably-disjoint rule arms);
the union-elision REFUTATION record (measured, stays doc-side — noted
here as deliberately unmodeled: performance, not semantics).
**Representation move:** Level 1 for deduplication — every elision the
engine performs names a theorem, completing what the constitution's
witness types started.

## Context (decided shape)

Definitions:
- `seenFold : List Binding → List Binding` — first-occurrence
  filtering (the seen-set as a fold).
- `BoundFieldsCoverKey rule occ` — the distinct-bindings elision law's
  premise, stated against 03's Functionality: every participating
  occurrence's bound fields contain a declared key's field set of its
  relation.
- `DisjointArms q` — no instance produces the same answer tuple from
  two different rules (the semantic property `provably_disjoint_rules`
  approximates syntactically).

Theorems:
1. `seenfold_is_set_semantics` — folding answers through the seen-set
   equals the answer SET (04's `queryAnswers`): dedup-by-fold is the
   denotation (Bridge: the aggregate/projection sinks' seen-sets).
2. `distinct_witness_licence` — under `BoundFieldsCoverKey` for every
   occurrence of a rule, distinct facts yield distinct full bindings;
   hence folding WITHOUT the seen-set computes the same aggregate as
   folding the distinct set (Bridge: `DistinctWitness`,
   `AggregateSink::without_seen_set`). State via 05's
   `agg_over_distinct_bindings`.
3. `distinct_premise_load_bearing` — countermodel: one unkeyed
   occurrence, duplicate bindings from distinct fact pairs, a `Sum`
   that would double-count under elision. The bag-semantics accident
   the witness forecloses, made concrete.
4. `disjoint_witness_licence` — under `DisjointArms`, cross-rule dedup
   is a no-op: the union of per-rule answer sets equals the
   concatenation's set (Bridge: `DisjointWitness`; the note that the
   engine SPENDS this witness only diagnostically today — the spanning
   union seen-set stayed by the measured refutation — with the
   refutation cited as doc-side authority, not restated).
5. `union_regime_head_projection` — when rules share the union
   seen-set, dedup keys on the projected HEAD tuple, not the full
   binding (the union_spans law; Bridge: the multi-rule union regime).
6. `syntactic_disjointness_sound` — a faithful abstraction of the
   `provably_disjoint_rules` check (constant-discriminated arms over
   one relation family — model the check's core rule, not its code)
   implies `DisjointArms`. The soundness direction only; completeness
   is explicitly a non-goal (the checker may refuse true disjointness
   — record as a note).

## Technical direction

`Binding`/answer machinery reused from 04 — no parallel definitions
(zero duplication is an in-tree law too). Item 6 models the CHECK at
its rule level: pick the strongest simple abstraction the Rust check
implements (read plan/fj/provably_disjoint.rs first; if its rule is
richer than constant-discrimination, model exactly what it does and
record the reading in the module doc). Keep every proof elementary —
these are finite-set pigeonhole arguments.

## Passing criteria

- `[shape]` All six theorems + the double-count countermodel checked;
  zero sorry/axioms; `scripts/lean.sh` 0.
- `[shape]` Theorems 2 and 4 carry their premises as hypotheses named
  after the witness types (grep `distinct_witness_licence` statement
  for the premise).
- `[shape]` The module doc's Bridge notes name the exact Rust
  consumers; the union-refutation stays cited-not-restated.
- `[gate]` CI green.

## Doc amendments

None yet — PRD 12 thins the dedup prose against these names.
