# err-001: `RenderedViolation` is kind-tag plus `direction: Option` plus `measure: Option`

- **Severity:** medium
- **Tree:** err
- **Status:** OPEN
- **Source:** audit/storage-schema.md F9
- **Depends on:** none
- **Conflicts with:** err-002 (both flatten `Violation`; coordinate)

## The bug

`Violation` is already a sum. `render_rejection` (`schema/render.rs:84-92`) matches it, then stuffs the result into `RenderedViolation` (`render.rs:26-41`) where `direction: Option<Direction>` and `measure: Option<u128>` are independently optional. Functionality-with-measure and Capacity-with-direction are representable.

This is the in-process `has_measure + payload` analog. Do **not** duplicate sdk-008 (`bdb_violation.has_measure` + two u64 words is C ABI essential; that issue is the bridge). Do **not** steal sdk-028 (TS/C++ dialect products). `MeasureOfRay { start, end }` is the *right* shape (both words are the ray) and is not this bug.

## Why it's wrong

Insight 7 — tag-plus-all-payloads. The engine sum was parsed and forgotten. Bindings that need a flat record should flatten at *their* boundary, not here.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree. C6/C7: C ABI `has_measure` stays (sdk-008). Engine `RenderedViolation` mirrors `Violation`'s sum (or is a `match` producing per-arm structs).

NAPI `ts/crate/src/marshal.rs` `from_rendered` today copies `direction`/`measure` Options onto `ViolationWire`. After this lands, marshal flattens the engine sum onto that wire at the NAPI boundary (sdk-028 owns the TS/C++ *host* types). Do not keep the engine type flat "for marshal convenience."

## Acceptance criteria

- [ ] Gone: `rg -n 'direction: Option<Direction>' crates/bumbledb/src/schema/render.rs`; `rg -n 'measure: Option<u128>' crates/bumbledb/src/schema/render.rs`.
- [ ] `RenderedViolation` cannot spell a capacity direction or a functionality measure.
- [ ] Unchanged tests: `schema/render/tests.rs` rejection-render tests green (field access may move into arms; assertion *values* unchanged).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Bindings-consumable facts (named decoded values) still attached. C ABI outbound flattening is sdk-008, not this issue. TS/C++ dialect sums are sdk-028. Do not change `MeasureOfRay`.
