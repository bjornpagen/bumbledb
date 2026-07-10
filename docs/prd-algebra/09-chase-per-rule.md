# PRD 09 — The chase, per rule

**Depends on:** 05; and `docs/prd/` 11–12 (the chase rewrite + surfaces) landed.
**Modules:** wherever `docs/prd/11-chase-rewrite.md` placed the elimination pass
(normalization-adjacent), extended over the rule list.
**Authority:** `40-execution.md` (the elimination pass, once the prior set
lands it), `30-dependencies.md`.
**Representation move:** the theory rewrites the program. Containment-licensed
atom elimination is sound per rule for exactly the reason it is sound at all —
every committed state models every statement — and a union's rules are
independent conjunctive bodies, so the chase distributes over them with no new
theory.

## Context (decided shape)

- The elimination fixpoint from the prior set runs **per rule**, independently;
  a rule shrinking below its cover requirements re-validates like any rule.
- New opportunity unique to rules, taken: **rule subsumption.** If rule A's
  body, after elimination, is a homomorphic image of rule B's (B's conditions ⊆
  A's on identical head projection), then A ⊇ B in denotation and **B is
  deleted** — classical UCQ minimization, restricted to the cheap witness the
  DNF path actually produces (identical atom multisets differing only by a
  filter that elimination removed). Full CQ-homomorphism minimization is
  NP-hard and **refused**; the restricted witness is a normalized-form
  containment check, O(rules²) at prepare with rules ≤ 16.
- EXPLAIN reports eliminated atoms (per rule, with statement ids — prior set's
  surface) and deleted rules (with the subsuming rule's index).

## Technical direction

1. Loop the existing pass over rules; no cross-rule state.
2. Subsumption: normalized-body comparison after elimination; delete, re-check
   the head alignment invariant (deleting a rule never changes the head).
3. The differential off-switch from the prior set covers both passes.

## Passing criteria

- `[test]` A two-rule query where one rule's containment-implied atom is
  eliminable: eliminated in that rule only; results identical with the pass
  forced off.
- `[test]` DNF residue: a lowered `(φ ∨ true-by-elimination)` pair where
  elimination makes one rule subsume the other — the subsumed rule is deleted;
  results identical; EXPLAIN names it.
- `[shape]` No homomorphism search exists (grep: the subsumption witness is
  normalized-form equality-modulo-eliminated-filters, nothing recursive).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`40-execution.md`: the elimination section gains "per rule" and the
subsumption paragraph with the refused general form and its reason.
