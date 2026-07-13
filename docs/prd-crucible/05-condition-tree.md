# PRD 05 — The vocabulary reclaimed: PredicateTree becomes ConditionTree

**Depends on:** 04 (the word "predicate" must already mean the real
thing before the squatter is evicted — the tree must never hold two
meanings at once in the same direction).
**Modules:** `crates/bumbledb/src/ir.rs` (`PredicateTree`,
`Rule.predicates`, `MAX_PREDICATE_DEPTH`), `error.rs`
(`PredicateNestingTooDeep`), `ir/validate/`, `ir/normalize/` (dnf, fold,
place_comparisons), `ir/render.rs`, `crates/bumbledb-query/src/lib.rs`
(the emitter's field name), every test constructing rules, `docs/
architecture/20-query-ir.md` + notation prose.
**Authority:** the vocabulary discipline (one word, one concept); the
owner's inelegance mandate. "Condition" chosen over the collision set:
`filter` is the view layer's, `guard` is storage's, `where` is not a
noun.
**Representation move:** none — this is a pure rename, and it is in the
set BECAUSE renames of load-bearing vocabulary are exactly what rots
when done timidly. Done completely, in one PRD, grep-zero.

## Context (decided shape) — the rename ledger, exhaustive

- `PredicateTree` → `ConditionTree` (the type, every import, every
  match).
- `Rule.predicates: Vec<PredicateTree>` → `Rule.conditions:
  Vec<ConditionTree>` — THE PUBLIC IR FIELD MOVES. Zero compat: hosts
  (the notation emitter, tests, bench querygen/builder) update in the
  same PRD.
- `MAX_PREDICATE_DEPTH` → `MAX_CONDITION_DEPTH` (+ its doc lines).
- `ValidationError::PredicateNestingTooDeep` →
  `ConditionNestingTooDeep` (+ display arm text: "condition trees
  nest…").
- The `bumbledb-query` emitter writes `conditions:` into its generated
  IR block; its module docs' grammar prose follows.
- Docs: `20-query-ir.md`'s grammar block (`predicates:` field, the
  "predicate trees" prose, the DNF section), `40-execution.md`'s
  references to "predicate trees" if any, README grammar table if it
  names the field.
- NOT renamed (each verified as a different concept, listed in the
  commit body): `FilterPredicate` (view-layer residuals — the view
  vocabulary, recorded), `resolve_predicates` (api/prepared — it
  resolves the view-layer FilterPredicates; renaming it
  `resolve_filters` IS in scope and correct — do it, it removes the last
  engine identifier that says "predicate" and means comparisons),
  `RESOLVE_FILTERS` obs name (already correct), Datalog prose using
  "predicate" for relations/atoms in the paper-fidelity docs (correct
  usage, stays).

## Technical direction

Compiler-driven, one motion: rename the type and field, chase every
error, then the string surfaces (display text, docs, emitter output,
render goldens if any golden text contains the word — verify: the
notation renders operators, not the field name; expected zero golden
churn, assert it). `resolve_predicates → resolve_filters` rides along
with its obs span already named `resolve_filters` — the code catches up
with its own telemetry.

## Passing criteria

- `[shape]` `grep -rn "PredicateTree\|MAX_PREDICATE_DEPTH\|
  PredicateNestingTooDeep\|resolve_predicates" crates docs` → zero hits;
  `grep -rn "\.predicates" crates` → zero (the field is `conditions`).
- `[shape]` `grep -rni "predicate" crates/bumbledb/src` hits only:
  `Predicate`/`PredicateColumn` (PRD 04's type), Datalog-correct prose,
  and `FilterPredicate` (the recorded view vocabulary — its own rename
  is REFUSED here: the view layer's word is load-bearing across
  40-execution and the memo docs; one vocabulary migration per campaign).
- `[test]` Zero golden edits in the notation suites (proves the rename
  never leaked into rendered text); full workspace suite green with
  identical counts.
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

As listed in the ledger — `20-query-ir.md` is the load-bearing one.
