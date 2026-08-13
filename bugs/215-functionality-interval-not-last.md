# Non-final interval FD is pointwise in Lean, refused in Rust
- id: 215
- severity: low
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: spec
- components: lean/Bumbledb/Dependencies.lean, lean/Bumbledb/Schema.lean, crates/bumbledb/src/schema/validate.rs, docs/architecture/30-dependencies.md
- status: open (do not fix)

## Summary
`intervalSplit` is position-blind: a single interval field anywhere in the projection yields the pointwise `PointwiseKey` reading. The engine requires that interval to be the *last* projection position (`FunctionalityIntervalNotLast`) because the neighbor probe needs a scalar prefix. Lean records the refusal as making the pointwise-non-final reading "moot for accepted theories." An implementer of `holds` from the spec without the gate would enforce WITHOUT OVERLAPS on a key the storage layer cannot probe.

## Lean spec
`Dependencies.lean:69-76`: "an FD with exactly ONE interval position written NON-FINALLY receives the POINTWISE reading — the set-canonical split is position-blind by design. That reading is moot for accepted theories: the engine refuses the non-final shape at declaration (`FunctionalityIntervalNotLast`)."

## Normative docs
`30-dependencies.md:427-428`: "at most **one** interval-typed field, and it must be the **final** projection position (the neighbor probe needs the scalar prefix as its group)."

## Rust implementation
```576:583:crates/bumbledb/src/schema/validate.rs
    if let Some(pos) = interval_position
        && pos != projection.ordered().len() - 1
    {
        return Err(StatementErrorKind::FunctionalityIntervalNotLast {
            relation: relation_id,
            field: projection.ordered()[pos],
        }
        .at(id));
    }
```

## Why this matters
Low on the sealed engine path. High if someone ports validate from Lean `Statement.judgment` alone: they would accept `R(window, id) -> R` as pointwise and then lack an ordered determinant group for neighbor probes.

## Verification (2026-08-12)
Re-read the position-blind split, the last-position architecture rule, and `FunctionalityIntervalNotLast`. **Confirmed.** `wrong-side: spec`.

**Lean** (`lean/Bumbledb/Dependencies.lean:61-76`): exactly one interval field anywhere yields the pointwise reading; “moot for accepted theories” because the engine refuses non-final at declaration. `Schema.lean:361-376`: `intervalSplit` is field-set / position-blind.

**Docs** (`docs/architecture/30-dependencies.md:427-428`): “at most **one** interval-typed field, and it must be the **final** projection position (the neighbor probe needs the scalar prefix as its group).”

**Rust** (`crates/bumbledb/src/schema/validate.rs:576-583`): `pos != projection.ordered().len() - 1` → `FunctionalityIntervalNotLast`.

## Related
- 213 (multi-interval default)
