# membership_lowering_preserves assumes membership-free negation; the engine does not
- id: 224
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: spec
- components: lean/Bumbledb/Query/Membership.lean, lean/Bumbledb/Bridge.lean, crates/bumbledb/src/ir/normalize/normalize.rs, docs/architecture/20-query-ir.md
- status: open (do not fix)

## Summary
The Bridge-cited seam theorem `membership_lowering_preserves` requires every negated atom to be membership-free. The engine lowers negated membership bindings to `AntiProbe` and executes them. A companion theorem `surface_antiprobe_filters` / `membership_lowering_preserves_negated` covers the full roster against the occurrence form, but the executable `ruleAnswers`/`eval_sound` path — and the theorem Bridge names — does not. Conformance fences the fragment (214).

## Lean spec
```1417:1433:lean/Bumbledb/Query/Membership.lean
/-- **THE membership-lowering theorem — the seam-closer.** …
whenever the negated atoms are membership-free — the one
fragment the pre-lowered RULE syntax can spell (recorded narrowing: a
negated membership binding has no pre-lowered rule form; …
theorem membership_lowering_preserves … 
    (hneg : ∀ a, a ∈ r.negated → Atom.membershipFree Γ a) :
```

Bridge (`Bridge.lean:266-268`) cites this theorem (and `surface_antiprobe_filters`) against `normalize.rs::lower_atom`. `eval_sound` is over `ruleAnswers` after lowering, with measure-free bindings — not negated membership.

## Normative docs
`20-query-ir.md` treats negated membership as accepted IR (safety + anti-probe). No "membership-free negation" restriction on the surface.

## Rust implementation
`ir/normalize/normalize.rs` `lower_atom` is role-blind: negated membership becomes `AntiProbe` with filters. Tests: `exec/run/tests/intervals.rs` `negated_membership_rejects_only_covered_events`.

## Why this matters
The named refinement theorem does not licence the engine's negated-membership answers. A bug in `AntiProbe` matching would still satisfy `membership_lowering_preserves` (vacuously on the fenced fragment). The spec's executable denotation and the engine's accepted IR are not the same language.

## Related
- 214 (corpus fence)
- 221 (another unmodeled negation rewrite)
