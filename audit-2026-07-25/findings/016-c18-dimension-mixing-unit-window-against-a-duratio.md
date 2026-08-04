## C18 dimension mixing (unit window against a Duration bound) passes BOTH TS tiers — the ban table's two-tier doctrine has a missing row

bug | medium | CONFIRMED | ts-surface-fresh
outcome: fixed 4239208e

### Summary

The unit `capacity()` overload accepts a `duration(field)` dependent bound at both the type tier and the construction tier, even though the engine unconditionally refuses that pairing as `CapacityDimensionMixing` (C18, ruled 2026-07-24: "a count of facts bounded by a span of time is a dimension error"). The TS surface's own module doc (ts/src/capacity.ts:11-28) declares the ban table "enforced REPRESENTATIONALLY in two tiers" with the engine's validation reserved for "a hostile FFI caller" — this row is enforced by the final authority alone, so a well-typed, mint-honest caller learns of the error only at `Db.create`/`Db.open`.

### Evidence (verified against real code)

- **Type tier passes** — probe compiled under the project tsconfig with exit 0 (probe presence in the compilation confirmed via `tsc --listFiles`):
  ```ts
  const Room = relation("Room", { id: u64.fresh, span: interval(i64) })
  const Booking = relation("Booking", { id: u64.fresh, room: u64 })
  capacity(on(Room, "id"), within(0n, duration("span")), on(Booking, "room")) // tsc: clean
  ```
  - ts/src/statements.ts:364-368 — the unit overload intersects only `UnitWindowBan<W> & BoundsOnTarget<W, B>` into the window.
  - ts/src/capacity.ts:192-197 — `UnitWindowBan` matches only `{ kind: "floor", lo: 1n }` (the `{1..*}` row).
  - ts/src/capacity.ts:244-250 — `BoundsOnTarget` for a `DurationRef` hi checks only that the field is an interval field of the target (`BoundOnTarget<K, "interval", B>`); no verdict pairs the unit instance with the Duration bound, though the overload statically has the exact window type (`hi: DurationRef`) and knows it is the unit instance.
- **Construction tier passes** — the probe constructs the statement (`weight: {kind:"unit"}, window: {kind:"range", hi:{kind:"durationField", field:"span"}}`) and `schema()` builds without a throw:
  - ts/src/statements.ts:399-403 — the only weight×window construction arm checks `weight.kind === "unit" && window.kind === "floor" && lo === 1n` (the `{1..*}` row only).
  - ts/src/statements.ts:321-344 — `assertBoundsOnTarget` validates roster membership and field kind (`durationField` → interval), never the weight-bound dimension pairing.
  - `grep -rn "dimension\|C18" ts/src/` finds no enforcement outside capacity.ts prose.
- **Engine refuses** — `Db.create` on the probe schema fails with exactly:
  > statement 3: a unit (count) window against the Duration bound on field 1 — a count of facts bounded by a span of time is a dimension error (ruled 2026-07-24, C18): weigh the source with `[Duration(field)]`, or bound by a u64 field or literal
  - crates/bumbledb/src/schema/validate.rs:900-913 — `Bound::TargetDuration` arm: `if weight == Weight::Unit { return Err(StatementErrorKind::CapacityDimensionMixing { field }.at(id)) }`.
- **Doctrine acknowledges the row** — ts/src/capacity.ts:447: "(Duration weights pair with Duration-capable bounds — C18)" — the rule is documented TS-side but not carried. The sibling weight-sensitive row (`{1..*}`) IS split per-aggregate at both tiers, proving the pattern.

### Failure scenario / impact

A user writing calendar capacity forgets the `weigh(duration(...))` argument — `capacity(on(Room, "id"), within(0n, duration("span")), on(Booking, "room"))` compiles clean, constructs, renders, and survives until `Db.create`/`Db.open`, the coldest refusal point, despite everything needed to refuse at the keystroke being statically present in the overload. This contradicts the SDK's stated two-tier enforcement contract (engine validation is for hostile FFI callers only).

### Suggested fix

- **Type tier**: intersect a `UnitBoundBan<W>` into the unit overload's window parameter (ts/src/statements.ts:366): `W["window"] extends { readonly hi: DurationRef<string> } ? BannedWindow<"a count of facts bounded by a span of time is a dimension error (C18) — weigh the source with weigh(duration(field)), or bound by a u64 field or literal"> : unknown` — the exact shape `UnitWindowBan` already has.
- **Construction tier**: one more arm beside the `{1..*}` check at ts/src/statements.ts:399: `if (weight.kind === "unit" && window.kind === "range" && window.hi.kind === "durationField") { throw ... }` with the same C18-naming diagnostic the engine emits, plus a test asserting both tiers refuse (type-level via the existing verdict-assertion pattern, runtime via the untyped-caller path).