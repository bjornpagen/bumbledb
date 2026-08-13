# Docs cite Lean grounding theorems for a negated complement fold Lean does not model
- id: 221
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: docs
- components: docs/architecture/40-execution.md, lean/Bumbledb/Exec/Rewrites.lean, crates/bumbledb/src/plan/ground/evaluate.rs
- status: open (do not fix)

## Summary
The engine folds negated closed atoms to a complement id-set at prepare (`fold_negated`). `40-execution.md` describes that complement rule and cites `Rewrites.lean: grounding_preserves_answers` / `elimination_sound` as covering "both rewrites" (elimination and evaluation). Lean explicitly leaves the negated complement fold unmodeled: preservation is proved for positive grounding only. The architecture citation overclaims the spec.

## Lean spec
```94:99:lean/Bumbledb/Exec/Rewrites.lean
* **The negated complement fold is unmodeled**
  (`plan/ground/evaluate.rs::fold_negated`): the complement rewrite
  needs the domain
  guarantee (`domain_within_ids`) and a negated membership the
  condition grammar cannot write; the modeled step grounds positive
  occurrences only.
```

`grounding_preserves_answers` is the positive-atom fold.

## Normative docs
`40-execution.md:539-559`: evaluation "marks prepare-evaluable closed-relation occurrences `Role::Folded`"; "Both rewrites — and any chain of them … preserve the query's answers (`lean/Bumbledb/Exec/Rewrites.lean: grounding_preserves_answers`, `elimination_sound`, composed by `rewrite_composition`)." Complement rule at `:658-672` (domain guarantee, empty complement ⇒ rule dead) has no separate Lean citation.

## Rust implementation
`evaluate.rs:40-41`, `:206-249` `fold_negated`: empty survivor set deletes the anti-probe; full extension kills the rule; keyed `k ∈ S` complement otherwise; domain_within_ids required.

## Why this matters
A wrong complement (off-by-one on sealed ids, missing domain guarantee) is a query-answer bug on `!Closed(...)` atoms. The Lean theorems cited as the rewrite's licence do not mention negation. Level-1 law ("each semantics-bearing algorithm … PROVED equal to its denotation") is unmet for this fold.

## Related
- 205 (docs citing the wrong Lean theorem)
