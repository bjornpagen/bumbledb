# DNF fold-preservation cites the projection theorem, not the fold theorem
- id: 205
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: docs
- components: docs/architecture/20-query-ir.md, lean/Bumbledb/Query/Denotation.lean, lean/Bumbledb/Exec/Dedup.lean, crates/bumbledb/src/api/prepared/build.rs
- status: fixed (2026-08-13)

## Summary
The query-IR architecture doc claims DNF lowering is both answer-preserving and fold-preserving, and cites `Denotation.lean: dnf_preserves_denotation` for that sentence. That theorem is proved only for projection `ruleAnswers`. The aggregate or-transparency law lives in `Dedup.lean: dnf_rekey_transparent`. A reader who treats the citation as covering Sum/Count domains can miss the R2 re-key (shared slot arrays vs head projection).

## Lean spec
```780:783:lean/Bumbledb/Query/Denotation.lean
theorem dnf_preserves_denotation (C : Classify) (r : Rule)
    (I : Instance) {ρ : ParamEnv} :
    ∀ t, t ∈ ruleAnswers C r I ρ ↔
      ∃ r', r' ∈ r.lower ∧ t ∈ ruleAnswers C r' I ρ
```

`ruleAnswers` is the projection denotation. The fold law:

```1355:1358:lean/Bumbledb/Exec/Dedup.lean
theorem dnf_rekey_transparent (C : Classify) (r : Rule) (I : Instance)
    (ρ : ParamEnv) (keys : List KeyTerm) (slots : List VarId)
    (fold : List (Option Value) → Set (List Value) → AnswerTuple) :
    aggAnswersDNF C r.lower I ρ keys slots fold =
      aggAnswersOn C r I ρ keys slots fold
```

No `dnf_*` theorem exists in `Query/Aggregates.lean`.

## Normative docs
```748:752:docs/architecture/20-query-ir.md
answer-preservingly (`lean/Bumbledb/Query/Denotation.lean:
dnf_preserves_denotation`) and **fold-preservingly**: the disjunct rules
share the written rule's variable scope and slot layout, and the union dedup
re-keys on those shared slot arrays, so distribution never changes a fold
domain (the or-transparency law, § aggregation; ruled 2026-07-23, R2).
```

The aggregation section (`:286-306`) states R2 correctly; the DNF section's citation does not point at the fold theorem.

## Rust implementation
DNF-derived multi-rule sinks use the shared-slot union key (`build.rs` DNF provenance; `exec/sink/tests/aggregate.rs` "Sum folds the written rule's distinct full bindings"). That matches `dnf_rekey_transparent`, not `dnf_preserves_denotation`.

## Why this matters
Wrong citation can license a head-projection fold over DNF arms (the hand-written multi-rule regime), changing `Sum` over an `or` of conditions. The engine got this right; the architecture citation does not.

## Verification (2026-08-12)
Re-read both theorems and the DNF paragraph. **Confirmed.** `wrong-side: docs`. The aggregation section states R2 correctly; the DNF section cites the projection theorem for a fold claim.

**Lean** (`lean/Bumbledb/Query/Denotation.lean:780-783`): `dnf_preserves_denotation` is `ruleAnswers` (projection). Fold law (`lean/Bumbledb/Exec/Dedup.lean:1345-1358`): `dnf_rekey_transparent` on shared slot arrays. No `dnf_*` theorem in `Query/Aggregates.lean`.

**Docs** (`docs/architecture/20-query-ir.md:748-752`): DNF lowering is “answer-preservingly (`dnf_preserves_denotation`) and **fold-preservingly**”. Aggregation (`:286-306`) states the or-transparency law and the hand-written vs DNF key split correctly.

**Rust** (`crates/bumbledb/src/api/prepared/build.rs:136-140`): DNF-derived multi-rule sinks re-key on shared slot arrays — `dnf_rekey_transparent`, not head-projection `ruleAnswers`.

## Related
- 202 (another docs overclaim on Dedup licences)

## Resolution (2026-08-13)
`20-query-ir.md` cites `dnf_preserves_denotation` for projection and `dnf_rekey_transparent` for the fold law.
