## TS wall has no C18 twin at either tier — unit capacity with a duration() bound constructs and dies only at Db.create

missing-free-feature | medium | CONFIRMED | capacity-surface
outcome: fixed 4239208e

### Summary

The TS SDK's capacity ban table is enforced in two tiers per its own doctrine (ts/src/capacity.ts header, lines 11-28): a type-tier refusal on the mint/overload, and a construction-tier twin for computed values, with the engine as final authority for hostile FFI callers. Every engine capacity refusal has a TS twin at both tiers — except C18 (dimension mixing: a unit/count window bounded by a `duration()` field). `capacity(on(Room, "id"), within(0n, duration("span")), on(Booking, "room"))` type-checks, constructs, freezes, composes into `schema()`, and is refused only by the engine's `CapacityDimensionMixing` at `Db.create`/`Db.open`.

### Evidence (verified)

- **Type tier cannot express C18.** `UnitWindowBan` (ts/src/capacity.ts:192-197) matches only `{kind:"floor", lo:1n}` — a `range` window with `hi:{kind:"durationField"}` passes. `BoundsOnTarget` (ts/src/capacity.ts:244-250) has no weight type parameter; it only checks the named field is an interval of the TARGET roster, so it cannot judge unit-weight × Duration-bound.
- **Construction tier's sole weight×window cross-check is the {1..*} ban.** ts/src/statements.ts:399-403: `if (weight.kind === "unit" && window.kind === "floor" && window.lo.kind === "lit" && window.lo.value === 1n)`. `assertBoundsOnTarget` (statements.ts:338-342) again only checks the interval kind. No other cross-judgment exists.
- **Zero C18 enforcement in TS.** grep for `dimension`/`C18` over ts/src and ts/test finds no capacity enforcement; the one C18 mention is the `duration()` docstring at ts/src/capacity.ts:447 ("Duration weights pair with Duration-capable bounds — C18") — cited, not enforced.
- **The engine refusal exists.** crates/bumbledb/src/schema/validate.rs:910: `Bound::TargetDuration` with `weight == Weight::Unit` returns `StatementErrorKind::CapacityDimensionMixing` (error.rs:384; display at error/display.rs:569-573, naming the `weigh(Duration(field))` repair).
- **The Rust macro surface enforces C18 at authoring time.** crates/bumbledb-macros/src/lib.rs:1753-1797 carries the C18 dimension gate at expansion, with a compile-fail test at crates/bumbledb/tests/schema-compile-fail/capacity_unit_window_duration_bound.rs. TS is the lone authoring surface deferring C18 to the engine boundary.

### Failure scenario / impact

A TS author writes the unit+`duration()` statement. It compiles, constructs, and the whole schema is rejected one boundary later — at `Db.create`/`Db.open` — with an engine `SchemaError` instead of the construction-site teaching error every neighboring ban produces (window bans in `within()`, path bans in `weigh()`/`ref()`/`duration()`, weight typing in `assertWeightOnSource`, bound typing in `assertBoundsOnTarget`, the `{1..*}` unit ban). In a deploy path this is a runtime failure where the SDK's stated two-tier doctrine — and the Rust macro's actual behavior — promise an authoring-time one.

### Suggested fix

Two spots, matching the file's established pattern:

1. **Construction tier** (ts/src/statements.ts, after the {1..*} check at :399): `if (weight.kind === "unit" && window.kind === "range" && window.hi.kind === "durationField") throw ...` naming C18 (ruled 2026-07-24) and the `weigh(duration(...))` repair — mirroring the engine's display.rs:569 wording.
2. **Type tier** (ts/src/capacity.ts): a `UnitDurationBoundBan<W>` alongside `UnitWindowBan` on the unit overload — `W["window"]["hi"] extends { kind: "durationField" }` → a `BannedWindow<...>` verdict naming the same repair. (`BoundsOnTarget` stays weight-agnostic; the weight-sensitive row rides the overload, exactly as the {1..*} ban does per design § 6.)

Land with a test asserting both the type-tier `@ts-expect-error` and the construction-tier throw, per the campaign's every-change-lands-with-its-test policy.