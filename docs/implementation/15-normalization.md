# PRD 15 — Normalization

Authority: `docs/architecture/20-query-ir.md` (normalization section + its Deviation
block), `30-execution.md` (inputs from normalization).

## Purpose

Lower a `ValidatedQuery` into the paper-form conjunctive query execution consumes:
distinct-variable atom occurrences, per-atom filters, residual comparisons.

## Technical direction

- `ir::normalize`. `normalize(&ValidatedQuery) -> NormalizedQuery`. Infallible — the
  witness guarantees every input is lowerable (types are proof-carrying; do not return
  Result).
- `NormalizedQuery { occurrences: Vec<Occurrence>, residuals: Vec<PlacedComparison> }`;
  `Occurrence { occ_id: OccId(u16), relation: RelationId, vars: Vec<(FieldId, VarId)>
  /*distinct vars only*/, filters: Vec<FilterPredicate> /*PRD 12's type*/ }`.
- Lowering rules, exactly per doc: (1) atom occurrences numbered (self-joins natural);
  (2) a variable repeated within one atom keeps its first field binding as the
  variable position and lowers subsequent positions to same-fact field-equality
  filters; (3) `Literal`/`Param` bindings lower to (field, Eq, constant) filters —
  params as symbolic constants resolved at bind time (the `FilterPredicate` constant
  slot becomes `Const::{Word(u64/u8), Param(ParamId)}` — extend PRD 12's type here and
  update PRD 12's evaluator to take a param-resolution slice); range comparisons whose
  one side is a single-atom var and other side a literal/param also lower to filters
  (pushdown); (4) everything remaining — var-vs-var across atoms — goes to
  `residuals`, each tagged with its two VarIds (node placement is PRD 17's job).
- String/Bytes literal constants stay raw bytes in the normalized form; resolution to
  intern-id words happens per-execution (PRD 25 wiring, doc rule: miss = empty).
  Represent as `Const::PendingIntern(Box<[u8]>, tag)`.

## Non-goals

Plan construction. Filter *evaluation* changes beyond the `Const` extension.

## Passing criteria

- Unit tests: repeated-var atom lowers to one var + equality filter and executes
  correctly through PRD 12's evaluator on a fixture image; literal binding lowers to a
  filter; two same-relation atoms get distinct occ_ids and independent filters; a
  var-literal range comparison in `predicates` lands as a filter, a var-var cross-atom
  comparison lands in residuals; normalized occurrence vars are duplicate-free
  (property over generated inputs); zero-binding atom → occurrence with empty vars and
  no filters.
- Global commands green.
