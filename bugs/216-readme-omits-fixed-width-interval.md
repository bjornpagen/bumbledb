# README type table omits interval&lt;E, w&gt;
- id: 216
- severity: low
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: docs
- components: README.md, docs/architecture/10-data-model.md, lean/Bumbledb/Values.lean, crates/bumbledb-theory/src/schema.rs
- status: fixed (2026-08-13)

## Summary
The product README's "six types" signature table lists `interval<E>` (16-byte start‖end) and has no row for the fixed-width family `interval<E, w>` (one-word start, end derived). Lean, `10-data-model.md`, and the Rust type descriptor all treat `interval<E, w>` as a distinct type that changes the encoding (admission rule).

## Lean spec
`ValueType.intervalFixed` (`Values.lean:510-515`), `FixedU64`/`encodeAt` one-word arm (`:561-562`), `FixedU64.not_ray`, `fixed_measure_const_u64`.

## Normative docs
`10-data-model.md:19-22`, `:56-73`: `Interval(element, w)` is 8 bytes, the START only; width is the type; Q2 bound. README (`README.md:444-454`) table: only `interval<E>`.

## Rust implementation
`bumbledb-theory` `TypeDesc` / schema value types include fixed-width intervals; `Interval::fixed`; one-word encoding/decode.

## Why this matters
Readers of the README (the first type roster most people see) will not know a width parameter is a type and a fingerprint input, or that `interval<u64, 1>` is not `interval<u64>`. Architecture docs are complete; the product summary is not.

## Verification (2026-08-12)
Re-read the README table, `10-data-model.md`, Lean `intervalFixed`, and `Interval::fixed`. **Confirmed.** `wrong-side: docs`. Cookbook intro (`docs/cookbook.md:7`) also says “six value types” while deferring to the architecture chapter that *does* include the family.

**Lean** (`lean/Bumbledb/Values.lean:510-515`, `:561-562`): `ValueType.intervalFixed`; `encodeAt` one-word arm; `FixedU64.not_ray`.

**Docs:** `docs/architecture/10-data-model.md:19-22`, `:56-73`: `Interval(element, w)` is 8 bytes, START only. README table (`README.md:444-454`): `interval<E>` only; header “six types”.

**Rust:** `bumbledb-theory` `Interval::fixed`; one-word encode/decode of the start.

## Related
- 209 (another README/architecture encoding summary gap)

## Resolution (2026-08-13)
README type table includes `interval<E, w>`. Cookbook intro names both interval families.
