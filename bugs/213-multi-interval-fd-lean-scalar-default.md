# Multi-interval FD is scalar Functionality in Lean, a validate error in Rust
- id: 213
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: spec
- components: lean/Bumbledb/Schema.lean, lean/Bumbledb/Dependencies.lean, crates/bumbledb/src/schema/validate.rs, docs/architecture/30-dependencies.md
- status: fixed (2026-08-13)

## Summary
`Header.intervalSplit` returns `none` when a projection has two or more interval fields, so `Statement.judgment` on that FD is classical `Functionality` (injectivity of the concatenated tuple), not 2-D exclusion. The engine and architecture docs refuse `FunctionalityMultipleIntervals` at declaration. A theory Lean would judge as a scalar key is unsealable in Rust.

## Lean spec
`Schema.lean:367-370`: "Every other shape splits to `none`, truthfully: zero interval fields is the classical reading, and two or more are gate-refused … and take the scalar-reading default." `Statement.judgment` (`Dependencies.lean:277-280`): `none => Functionality`. Module docs say `holds` is consumed on accepted theories only — not encoded in the type.

## Normative docs
`30-dependencies.md:427-433`: FD "at most **one** interval-typed field, and it must be the **final** projection position"; "two interval positions would be 2-D exclusion, which the ordered determinant index cannot answer."

## Rust implementation
```568:573:crates/bumbledb/src/schema/validate.rs
    if positions.len() > 1 {
        return Err(StatementErrorKind::FunctionalityMultipleIntervals {
            relation: relation_id,
            field: projection.ordered()[positions[1]],
        }
        .at(id));
    }
```

## Why this matters
If a hand-built descriptor or a future frontend skipped the gate, Lean would accept overlapping intervals that share a scalar prefix as long as the full (S, i1, i2) tuples differ — not WITHOUT OVERLAPS. The dangerous reading is the Lean default, not the engine. Ambiguous spec for anyone implementing validate from `Statement.judgment`.

## Verification (2026-08-12)
Re-read `intervalSplit`, `Statement.judgment`, the FD gate, and validate. **Confirmed.** `wrong-side: spec`.

**Lean** (`lean/Bumbledb/Schema.lean:361-376`): two or more interval fields → `intervalSplit` is `none`. `Statement.judgment` (`Dependencies.lean:277-280`): `none => Functionality` (scalar injectivity of the concatenated tuple). Module docs say `holds` is consumed on accepted theories only — not in the type.

**Docs** (`docs/architecture/30-dependencies.md:427-433`): FD “at most **one** interval-typed field, and it must be the **final** projection position”; two interval positions would be 2-D exclusion.

**Rust** (`crates/bumbledb/src/schema/validate.rs:568-573`): `positions.len() > 1` → `FunctionalityMultipleIntervals`.

## Related
- 215 (interval-not-last: Lean would read pointwise; Rust refuses)

## Resolution (2026-08-13)
Lean `Header.functionalityAdmitted` refuses two-or-more interval fields (`FunctionalityMultipleIntervals`); `Statement.judgment` is `False`, not scalar Functionality.
