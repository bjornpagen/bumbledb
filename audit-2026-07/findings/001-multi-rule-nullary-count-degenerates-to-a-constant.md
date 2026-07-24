## Cross-rule nullary Count is definitionally constant 1 — deliberate and pinned, but silently admitted by validation, undocumented as a footgun, and outside every theorem's vocabulary

category: design-doctrine / theorem-coverage | severity: medium | verdict: PLAUSIBLE | finder: r2:lean-unswept-modules
outcome: fixed 34f96cd8

### Summary

In the multi-rule (union-regime) aggregate sink, the nullary `Count` contributes nothing to the head-projection dedup key, so for a fold-free head `find (g, Count)` the fold domain per group is exactly one element and Count is always 1, regardless of how many bindings each rule derives. The original finding called this critical lean–rust drift; verification shows it is neither drift nor hidden — it is the documented across-rules fold-domain law, deliberately pinned by Rust unit tests, faithfully mirrored by the Lean glue, and the cited corpus case's answers are correct even under single-rule semantics. What survives is a real design/doc gap: validation silently admits a query shape that is definitionally uninformative (the analogous `Arg`-across-rules case is refused with a typed error), the user-facing consequence (an `or` that DNF-lowers to two rules collapses `Count() by g` to 1) is stated loudly nowhere, and the union aggregate fold is covered by no theorem — only by executable glue and corpus agreement, which by construction cannot disagree with the engine.

### Evidence (all verified against the code)

Mechanism — real:
- `crates/bumbledb/src/exec/sink/aggregate/new.rs:379-392` — `union_span` returns `None` for `SinkSpec::Agg { over_slot: None, .. }`: the nullary Count contributes no words to the union dedup key.
- `crates/bumbledb/src/exec/sink/aggregate/fold_row.rs:36-45` and `:167-183` — in the union regime `dedup_key` gathers only the union spans and the seen-set gate returns early on a repeat, so only the first row per head projection folds. For head `(g, Count)` the key is just `g`; the Count accumulator increments once per group.
- `lean/Bumbledb/Conformance.lean:707-714` — `headRow` maps `.agg .count => pure ⟨.bool, false⟩` (constant filler); `evalUnion` (`:781-797`) dedups head rows and `projectUnionGroup` (`:764`) computes Count as `group.length` — 1 for a fold-free head. Lean and Rust agree exactly.

Not drift — deliberate at every layer:
- `crates/bumbledb/src/api/prepared/tests/rules.rs:289-293` — test `the_all_count_head_counts_the_singleton_union` pins the behavior with the comment: "every binding projects to the empty head tuple, so the union has exactly one element and Count is 1 — the naive model's constant-filler semantics, **pinned**."
- `crates/bumbledb/src/api/prepared/tests/rules.rs:170-222` — `aggregates_fold_the_union_of_head_projected_bindings` pins cross-rule Count as |head-projected union| (Count = 3, not the per-rule 2+2).
- `docs/architecture/20-query-ir.md:283-287` — "Across rules, aggregates read the head: the fold domain is the union of the rules' binding sets projected to the head" — two bullets above the Count definition at `:301`, so "Count = |the group's binding set|" is not contradicted once the across-rules clause fixes the domain. Restated in `docs/research/aggregate-comparisons.md:64-66`.

Corpus claim — refuted:
- `lean/conformance/cases/hand-union-aggregate-fold.json` — both rules bind only `var 0` (rule 1: relation 4, field 2 = var 0, field 7 = literal true; rule 2: relation 7, field 0 = var 0). Under the documented set semantics ("every aggregate folds the group's set of distinct full bindings", 20-query-ir.md:277) the per-group distinct-binding set is a singleton, so the recorded count = 1 is correct **even as a single-rule query**. The original finding's 93–110 "natural counts" are fact multiplicities (my recount from the instance: 92–109 for rule 1), which set semantics deliberately erases — the docs' own postings example says counting facts requires the fresh ids to differ **in the binding**, i.e. the id must be bound.

Surviving gaps — confirmed:
- `crates/bumbledb/src/ir/validate/validate.rs:415` — `ArgAcrossRules` is refused with a typed error because the cross-rule semantics is undefined; fold-free nullary Count across rules — definitionally constant 1 under the head-projection representation — is admitted silently. Same doctrine, opposite treatment, no recorded trigger.
- `lean/Bumbledb/Query/Syntax.lean:232-236` — `Rule.finds : List VarId`: the theorem model cannot express an aggregate head, so `union_regime_head_projection` (`lean/Bumbledb/Exec/Dedup.lean:567-597`) covers pure projection heads only. The `Dedup.lean:77-84` module-doc note records the Count vocabulary gap but argues soundness for **key equality** only; the fold-domain consequence (Count = 1) has no theorem, only the executable glue and the corpus — which agree with the engine by construction and therefore cannot flag this class of decision if it was ever wrong.
- Nowhere in `docs/architecture/20-query-ir.md` (which states the join-multiplicity aggregate footgun "loudly", `:294-299`) or the aggregation research doc is the concrete consequence stated: splitting `Count() by g` across rules — including any `or` that DNF-lowers to 2+ rules — returns 1 for every inhabited group, where the single-rule form (with the id bound) returns the group cardinality.

### Failure scenario

A user with `find (account, Count) where Posting(id: p, account: account, ...)` (Count = postings per account, correct single-rule) adds an `or` condition that DNF-lowers the program to two rules. The head projection drops `p`, the union seen-set keys only `account`, and every group's Count silently becomes 1. Every layer of the verification stack — Rust tests, Lean glue, conformance corpus — agrees, because all of them implement the same pinned representation choice; only a normative theorem over aggregate heads or a validation refusal could surface the surprise, and neither exists.

### Suggested fix

Pick one, recording the trigger either way:
1. **Refuse at validation** (the Arg precedent): a nullary `Count` in a fold-free head of a 2+-rule program is definitionally uninformative under the head-projection law — refuse it with a typed error alongside `ArgAcrossRules`, with the modeling answer "one Count query per disjunct, host-merged" (exactly the Arg doctrine at 20-query-ir.md:317-321). This is the representation-first move: make the uninformative query unrepresentable rather than answer it with a constant.
2. **If the pinned semantics stays**: state the footgun loudly in 20-query-ir.md § aggregation (the section already does this for join multiplicity), and give the union aggregate fold a normative Lean denotation with at least one theorem tying `evalUnion` to it — the current coverage (glue + corpus, no theorem, and a Dedup.lean note that addresses key equality but not the fold domain) leaves the whole union-aggregate semantics outside the proved lattice.
