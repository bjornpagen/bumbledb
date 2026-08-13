# TargetKeyAccepted accepts any declared FD; Rust closed targets require FieldId(0)
- id: 207
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: spec
- components: lean/Bumbledb/Dependencies.lean, crates/bumbledb/src/schema/validate.rs, docs/architecture/30-dependencies.md, crates/bumbledb/src/schema/tests/reject.rs
- status: open (do not fix)

## Summary
Lean `TargetKeyAccepted` is exact field-set match against any declared functionality of the target. For a closed target the engine refuses every projection except the synthetic id `FieldId(0)`, even when a user-declared payload key has the same field set. Lean records this as "acceptance strictly narrower, sound direction" but leaves `TargetKeyAccepted` broader than the gate theorems spend. Architecture docs match Rust, not the Lean definition.

## Lean spec
```183:185:lean/Bumbledb/Dependencies.lean
def TargetKeyAccepted (T : Theory) (target : Atom) : Prop :=
  ∃ K, Statement.functionality target.relation K ∈ T.statements ∧
    sameFields K target.projection
```

Recorded narrowing (`Dependencies.lean:89-95`): "a user-declared non-id key on a closed relation satisfies `TargetKeyAccepted` here yet Rust refuses the containment." Theorems such as `accepted_target_key_spent` and Oracle probe pricing quantify over this broader predicate.

## Normative docs
`30-dependencies.md:452-456`: IND into a closed target has "no key search" — "Y must be exactly the synthetic id (the handle is the one probe-able identity of a closed relation)."

## Rust implementation
```1353:1360:crates/bumbledb/src/schema/validate.rs
    if let Some(rows) = target_relation.extension.as_deref() {
        if target.projection.len() != 1 || target.projection[0] != FieldId(0) {
            return Err(StatementErrorKind::ClosedTargetNotHandle {
                target: target.relation,
                projection: target.projection.clone(),
            }
            .at(id));
```

Reject tests: `schema/tests/reject.rs` `ClosedTargetNotHandle` cases (~1143+).

## Why this matters
A schema that Lean would call `TargetKeyAccepted` (containment into a closed relation on a declared payload key) is a validate-time error in the engine. Admission/Oracle theorems that assume `TargetKeyAccepted` overstate what the engine admits. Sound direction for runtime, but the Lean acceptance premise is not the engine's.

## Related
- 208 (closed + interval containment also Lean-permissive / Rust-refused)
