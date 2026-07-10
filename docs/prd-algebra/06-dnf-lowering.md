# PRD 06 — DNF lowering: OR as data

**Depends on:** 05.
**Modules:** `crates/bumbledb/src/ir.rs` (input predicate shape),
`ir/normalize/` (the distribution), `ir/validate/` (the cap).
**Authority:** `20-query-ir.md`.
**Representation move:** the refused middle, recovered. OR tangled mid-rule
across atoms is refused as an execution concept (README refusals) — but any
boolean combination of positive predicates has a disjunctive normal form, and
**DNF of a query is a set of rules**. The surface accepts nested OR; the engine
never sees it. This is the outer-join precedent applied to disjunction: "a
documented decomposition, never a node."

## Context (decided shape)

- The IR's *input* predicate grammar gains one form: `Or(Vec<PredicateTree>)`
  where a `PredicateTree` is `Leaf(Comparison) | And(Vec<Self>) | Or(Vec<Self>)`.
  Negated atoms and membership stay leaf-level (no OR over atoms — atoms
  disjoin by writing two rules, which is what rules are for).
- Validation-time lowering: distribute to DNF; each DNF term becomes a **rule**
  (the rule's atoms are cloned; its predicate list is that term's leaves);
  the result then validates under PRD 05's ordinary roster, including
  `MAX_RULES`. Distribution that exceeds the cap is a typed error naming the
  blowup (`DnfExceedsRules { produced, cap }`) — the exponential case is
  rejected at declaration, exactly like guard-width overflow.
- Duplicate rules after distribution (identical normalized bodies) collapse —
  set semantics at the representation level.
- The *validated* artifact contains no `Or`: `grep` proves the executor and
  planner never learn disjunction existed.

## Technical direction

1. Input tree + recursive distribution in `ir/normalize/` (pure function,
   property-testable: lowering then evaluating ≡ evaluating the tree naively).
2. Rule dedup by the existing normalized-form equality (the duplicate-statement
   machinery's sibling).
3. The typed cap error with the produced-count payload.

## Passing criteria

- `[test]` Property: for randomized predicate trees over small corpora, the
  lowered rule set's union equals naive tree evaluation (the naive model
  evaluates the tree directly — it never lowers).
- `[test]` `(a ∨ b) ∧ (c ∨ d)` produces 4 rules; adding `∨` terms past the cap
  produces the typed error with the count.
- `[shape]` `ValidatedPlan` and everything downstream of validation contain no
  `Or` variant; the executor crate has no reachable disjunction code path.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`20-query-ir.md`: the input grammar, the lowering, the cap, and the refusal
cross-reference ("OR is data or it is nothing").
