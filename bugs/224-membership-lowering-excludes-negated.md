# membership_lowering_preserves assumes membership-free negation; the engine does not
- id: 224
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: spec
- components: lean/Bumbledb/Query/Membership.lean, lean/Bumbledb/Bridge.lean, crates/bumbledb/src/ir/normalize/normalize.rs, docs/architecture/20-query-ir.md
- status: fixed (2026-08-13)

## Summary
The Bridge-cited seam theorem `membership_lowering_preserves` requires every negated atom to be membership-free. The engine lowers negated membership bindings to `AntiProbe` and executes them. A companion theorem `membership_lowering_preserves_negated` / `surface_antiprobe_filters` covers the full roster against the anti-probe form, and `20-query-ir.md` cites that split correctly. The executable `ruleAnswers`/`eval_sound` path — and the Bridge *premise* for the named theorem (“over the full term roster”) — still do not. Conformance fences the fragment (214).

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
`20-query-ir.md:255-261` treats negated membership as accepted IR. `:488-494` cites `membership_lowering_preserves` for positive bindings and `membership_lowering_preserves_negated` for the anti-probe form — the polarity split is documented. The Bridge row for the named theorem still overclaims “full term roster” (`Bridge.lean:266-268`).

## Rust implementation
`ir/normalize/normalize.rs` `lower_atom` is role-blind: negated membership becomes `AntiProbe` with filters. Tests: `exec/run/tests/intervals.rs` `negated_membership_rejects_only_covered_events`.

## Why this matters
The named `ruleAnswers` refinement does not licence the engine's negated-membership answers. A bug in `AntiProbe` matching would still satisfy `membership_lowering_preserves` (vacuously on the fenced fragment). The spec's executable denotation and the engine's accepted IR are not the same language.

## Verification (2026-08-12)
Re-read both lowering theorems, Bridge, `20-query-ir.md`, and `lower_atom`. **Confirmed**, rewritten: architecture docs cite the companion theorem; the gap is `eval_sound`/`ruleAnswers` plus the Bridge premise overclaim. `wrong-side: spec`.

**Lean** (`lean/Bumbledb/Query/Membership.lean:1417-1433`): `membership_lowering_preserves` has `(hneg : ∀ a, a ∈ r.negated → Atom.membershipFree Γ a)`. Companion (`:1591-1595`) `membership_lowering_preserves_negated` against `antiProbeRuleAnswers`, no that hypothesis. `eval_sound` (`Denotation.lean:1656-1663`) equates list eval with `ruleAnswers` under safety and measure-free bindings.

**Docs:** `20-query-ir.md:488-494` correctly splits the two theorems. Bridge (`lean/Bumbledb/Bridge.lean:266-268`) cites `membership_lowering_preserves` with English “over the full term roster” and points at `lower_atom`; a separate row (`:271-274`) cites `surface_antiprobe_filters`.

**Rust** (`crates/bumbledb/src/ir/normalize/normalize.rs:220-235`): `lower_atom` is role-blind (“positive or negated — the rules are identical”); negated membership becomes `AntiProbe` with filters.

## Related
- 214 (corpus fence)
- 221 (another unmodeled negation rewrite)

## Resolution (2026-08-13)
Bridge `membership_lowering_preserves` names the membership-free-negation hypothesis. Engine negated membership is `membership_lowering_preserves_negated` / `surface_antiprobe_filters` / `normalize.rs::AntiProbe`. Conformance eval uses `surfaceMatchesB`. Ticket 214 un-fence depends on this.
