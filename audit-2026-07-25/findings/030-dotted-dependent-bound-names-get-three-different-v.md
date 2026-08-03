## Dotted dependent-bound names get three different verdicts across the three authoring walls

incoherence | low | CONFIRMED | capacity-surface
outcome: fixed 5eb216de + cb8af25f

### Summary

The path spelling (`a.b`) in a capacity statement's WEIGHT slot is a first-class typed refusal on all three authoring walls, each naming the pinned-column composition idiom (ruling 6). The identical spelling in the dependent-BOUND slot gets three unrelated verdicts: the TS SDK refuses with the same teaching idiom; the spec resolver has no path check and falls through to a generic `UnknownField`; the macro dies with the untargeted "trailing tokens after the window bounds" panic. The spec resolver is asymmetric within one file — `Resolver::weight` checks for dots, `Resolver::bound` does not — and the macro is asymmetric within one parser.

### Evidence (verified)

**Weight slot — typed idiom-naming refusal everywhere:**
- `crates/bumbledb-theory/src/schema/spec.rs:850-856` — `Resolver::weight` checks `if name.contains('.')` and pushes `SpecIssue::WeightPathRefused` before resolution; its `Display` (spec.rs:527-533) spells the full pinned-column example (`Device(model, watts) <= Model(id, watts); Pool(id) <=[watts]{0..supply} Device(pool);`).
- `crates/bumbledb-macros/src/lib.rs:1062-1074` — `parse_weight` peeks for `.` and panics with the same idiom-naming message.
- `ts/src/capacity.ts:304-311, 421-430` — `assertRowLocal` (construction tier) plus the type-level `PathBan` (capacity.ts:134) on `weigh()`.
- Golden test exists for the weight side only: `crates/bumbledb/tests/schema_spec.rs:984-1004`.

**Dependent-bound slot — three different verdicts:**
- TS: `ts/src/capacity.ts:439-441, 449-450` — `ref()` and `duration()` take `F & PathBan<F>` and route through `assertRowLocal(field, "dependent bound")` / `"Duration measure"`, throwing the same teaching message ("the vocabulary is closed at the row (ruling 6): pin the column with a two-column containment ... and name the local field"); `within()` re-asserts at capacity.ts:401.
- Spec: `crates/bumbledb-theory/src/schema/spec.rs:876-892` — `Resolver::bound` goes straight to `self.field(...)` with no dot check; `field` (spec.rs:658-668) fails the exact-name roster lookup and emits `UnknownField`, whose `Display` (spec.rs:446-453) is the shrug: ``statement N: relation `Pool` has no field `device.supply` ``.
- Macro: `crates/bumbledb-macros/src/lib.rs:1111-1123` — `{0..device.supply}` has `parse_bound` (1006-1019) consume `device` as `BoundSpec::Field`, leaving `.supply` to trip the generic `"schema!: trailing tokens after the window bounds"` assert.

**Contract weakened:** spec.rs:19-21 promises the foreign host "one round trip" with every failure typed; the bound-slot path spelling gets a message that misdiagnoses the mistake (the field may well exist — on the *other* relation).

**Doctrine nuance (corrects the finding's evidence, not its claim):** ruling 6 as written (`docs/design/capacity-laws.md:347-351`) closes the WEIGHT vocabulary at the row specifically. The TS surface generalized the law to dependent bounds; the two Rust walls and the doctrine text lag it. Whichever reading is canonical, the three walls disagree today.

### Failure scenario / impact

A foreign host (non-TS bindings, ETL tooling) sends `BoundSpec::Field("device.supply")` in the hi slot over the SchemaSpec surface and gets ``relation `Pool` has no field `device.supply` `` — a generic unknown-name verdict for a mistake the SDK and the weight bracket both diagnose precisely with the paste-back repair. A macro author writing `{0..device.supply}` gets "trailing tokens after the window bounds", which points at the grammar, not the idiom. Severity is low: no wrong descriptor is ever produced (all three walls do refuse), only the teaching quality diverges.

### Suggested fix

Mirror the weight's dot check at the top of `Resolver::bound` (spec.rs:876) — either a `BoundPathRefused` issue or a slot-parameterized refusal reusing `WeightPathRefused`'s Display shape naming the pinned-column idiom — one arm, plus the golden-message test alongside `a_path_weight_is_refused_naming_the_pinned_column_idiom` (tests/schema_spec.rs:984). In the macro, `parse_window`/`parse_bound` can keep refusing at parse but should peek for `.` after a Field bound (exactly as `parse_weight` does at lib.rs:1062) and panic with the idiom-naming message instead of "trailing tokens". Optionally, restate ruling 6's row-closure in capacity-laws.md to cover both the weight and dependent-bound slots, matching the TS wall's reading.