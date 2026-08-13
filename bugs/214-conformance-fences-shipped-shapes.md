# Conformance third oracle fences shapes Lean and the engine already denote
- id: 214
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: unspecified
- components: crates/bumbledb-bench/src/conformance.rs, lean/Bumbledb/Query/Membership.lean, lean/Bumbledb/Query/Aggregates.lean, crates/bumbledb/src/ir.rs, docs/architecture/20-query-ir.md
- status: open (do not fix)

## Summary
The Lean denotation and the Rust engine both define negated membership anti-probes, element-typed param-set membership, and (engine/docs) measure-keyed Arg. The executable three-way conformance corpus excludes all three (`excluded_negated_membership`, `excluded_set_membership`, `excluded_measure_arg_key`). The "third oracle" therefore does not watch the shipped semantics on those fragments.

## Lean spec
`Membership.lean:391-406` `surface_antiprobe_filters`: a negated membership atom rejects iff no fact passes domain bindings and membership filters. Bridge cites `normalize.rs::AntiProbe`. Param-set membership is `Query.paramSet_selects_membership`. Measure Arg keys are missing (201); the other two shapes are proved.

Some later lowering theorems still assume `Atom.membershipFree` on negated atoms (`Membership.lean:450+`) — a second, narrower reading that the fence also papers over.

## Normative docs
`20-query-ir.md` admits negated membership lowering, ParamSet, and R5 measure Arg keys as accepted IR. `60-validation.md` presents the Lean denotation as the third oracle over the checked-in corpus.

## Rust implementation
Engine: `normalize.rs::lower_atom` / `AntiProbe`; `Term::ParamSet`; `ArgKey::Measure`. Corpus builder (`conformance.rs:139-147`, `:749`, `:806`, `:921`, `:1197-1210`) counts and drops those candidates instead of encoding them.

## Why this matters
A regression in negated membership or set-membership matching will not fail `three_way_conformance_over_the_checked_in_corpus`. The census still lists that instrument as watching `Query.matches_def` / `eval_sound` / `program_eval_sound` for the whole query surface.

## Related
- 201 (`excluded_measure_arg_key`)
