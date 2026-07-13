# PRD 06 — The closure idiom: recursion punted on the record

**Depends on:** 05 (recipes are written in the final vocabulary).
**Modules:** `docs/cookbook.md` + `crates/bumbledb-query/tests/
cookbook.rs` (the token-sync suite and roster count),
`docs/architecture/20-query-ir.md` (the refusal record).
**Authority:** the census law (nothing ships without a sighting; the
2026-07 recursion analysis found none — both censused apps' query
surfaces collapse onto the current IR); the refusal-with-trigger
convention.
**Representation move:** a punt without a recorded trigger is silence;
a punt with a working idiom, a recipe, and a reactivation condition is
an engineering decision. This PRD makes the recursion punt the latter —
and makes the dogfooding period a fair census instrument.

## Context (decided shape)

Two cookbook recipes, house format (doc block + token-identical compiled
copy + tests), added to the roster:

1. **"The closure idiom"** — host-driven semi-naive over ∈-sets:
   ```
   frontier = {root}
   loop:
       next = query(parent ∈ frontier, child)   // one set-param query
       new  = next − seen
       if new.is_empty() { break }
       seen ∪= new; frontier = new
   ```
   The recipe text derives WHY this is honest at census scale (depth-
   bounded hierarchies, each iteration a microsecond set-selection
   query), names its failure modes verbatim from the refusal trigger
   (unbounded depth; closure composed into a larger plan; the
   chain-window class), and points at the recorded refusal. The compiled
   copy is a real schema (`Node { id, parent }`-shaped, house-idiomatic)
   plus the loop as a doc-tested host function; the test drives a small
   tree and asserts the reachable set.
2. **"The chart of accounts"** — hierarchical accounts + subtree rollup:
   the closure idiom composed with one `Sum` query over the accumulated
   set (`Posting(account ∈ subtree) → Sum(amount)`). The ledger
   workload's real recursion case, shown solved on the current engine.

Plus the refusal, recorded in `20-query-ir.md` beside the rules-shape
section: **engine recursion — refused.** The derivation (no census
sighting; the closure idiom covers depth-bounded hierarchies; recursive
commit-time judgments fail the acceptance gate categorically, so the
constraint-side motivation is void) and the trigger (a real workload
where the idiom measurably fails: unbounded/large depth, closure that
must compose with further joins inside one plan for performance, or
interval-intersection-along-paths — the chain-window class). Pointer to
PRD 07's design paper as the pre-paid execution plan.

## Technical direction

Follow the cookbook's own conventions exactly: recipe numbering appended
(roster 23 → 25), doc block and compiled copy token-identical (the sync
test enforces), each recipe's comment naming the theorem/idiom it buys,
cross-references both ways (recipe ↔ refusal record). The rollup recipe's
test asserts a hand-computed subtree sum. No engine code changes
anywhere in this PRD.

## Passing criteria

- `[test]` Cookbook suite green at roster 25; the two new recipes'
  tests drive real data (a 3-level tree; a 3-level account hierarchy
  with postings) and assert exact result sets/sums.
- `[shape]` `grep -n "refused" docs/architecture/20-query-ir.md` shows
  the recursion refusal with all three trigger clauses;
  `docs/cookbook.md` cross-references it; `grep -rn "recursion"
  docs/cookbook.md` hits only the two recipes' honest framing.
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

This PRD is its amendments; additionally the repo `README.md` cookbook
line updates its recipe count.
