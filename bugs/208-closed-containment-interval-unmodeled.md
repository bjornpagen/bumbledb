# Closed+interval containment is a Lean judgment; Rust refuses it v0
- id: 208
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: spec
- components: lean/Bumbledb/Dependencies.lean, lean/Bumbledb/Schema.lean, crates/bumbledb/src/schema/validate.rs, docs/architecture/30-dependencies.md
- status: open (do not fix)

## Summary
A containment with an interval-typed projection and a closed side is a well-formed `Coverage` / `Containment` judgment in Lean (`Statement.judgment` via `intervalSplit`). The engine and architecture docs refuse that shape at declaration (`ClosedContainmentInterval`). Lean notes the v0 refusal as unstated mechanism; `holds` is still defined on theories the engine will never seal.

## Lean spec
`Statement.judgment` (`Dependencies.lean:275-288`) dispatches on `Header.intervalSplit`: one interval field on both sides → `Coverage`. Closedness is not a conjunct. Narrowing (`Dependencies.lean:96-99`): "`ClosedContainmentInterval` … refuses interval-typed projections under a closed target outright — a v0 refusal this model does not restate."

## Normative docs
`30-dependencies.md:460-463`: "Interval positions on a containment with a closed side (either side) are **refused v0**: a pointwise judgment against a virtual extension would mix the coverage walk with virtual storage, and a constant source's coverage demand has no delete-time re-judgment path."

## Rust implementation
```699:720:crates/bumbledb/src/schema/validate.rs
    // Interval positions on closed containments: refused v0. ...
    if (source_closed || target_closed)
        && !interval_positions(target_fields, &target.projection).is_empty()
    {
        return Err(StatementErrorKind::ClosedContainmentInterval {
```

## Why this matters
Lean can prove coverage facts about closed+interval containments that no accepted engine schema can carry. Conformance judgment cases and Admission inhabitants that ignore the gate will not match `SchemaDescriptor::validate`. Sound direction if `holds` is consumed only on accepted theories — that premise is prose, not a type.

## Verification (2026-08-12)
Re-read `Statement.judgment`, the v0 architecture refusal, and `validate_containment`. **Confirmed.** `wrong-side: spec`.

**Lean** (`lean/Bumbledb/Dependencies.lean:275-288`): `Statement.judgment` dispatches on `Header.intervalSplit`; closedness is not a conjunct. Narrowing (`:96-99`): `ClosedContainmentInterval` “refuses interval-typed projections under a closed target outright — a v0 refusal this model does not restate.”

**Docs** (`docs/architecture/30-dependencies.md:460-463`): interval positions on a containment with a closed side (either side) are “refused v0”.

**Rust** (`crates/bumbledb/src/schema/validate.rs:699-720`): `(source_closed || target_closed) && !interval_positions(...).is_empty()` → `ClosedContainmentInterval`.

## Related
- 207 (closed-target key)
- 213, 215 (other gate-vs-denotation splits)
