## C18 dimension gate enforces one of three mixing directions while four doctrine sites state the full pairing law

incoherence | high | CONFIRMED | capacity-surface
outcome: fixed a2c555d8

### Summary

The C18 dimension-mixing refusal fires only on a unit (count) weight against a `Duration` bound. The other two mixed pairings — a u64-field weight bounded by a time span, and a Duration weight bounded by a u64 field — validate, seal, fingerprint, and are enforced at every commit as dimensionally meaningless comparisons. Four doctrine sites, including the doc comment on the very error variant users read, state an exclusive pairing table that the code does not enforce; three shipped tests pin the opposite, narrow reading. The estate did both halves of capacity-cutover item 11 ("enforce Duration↔Duration agreement in validate or document that u64 is u64") inconsistently.

### Evidence (all verified against v0.9.0 on branch bugbash-perf)

**The gate** — `/Users/bjorn/Documents/bumbledb/crates/bumbledb/src/schema/validate.rs:887-914`:
- `Some(Bound::TargetField(field))` (889-899) checks only `descriptor.value_type != ValueType::U64` — no cross-check against `weight`, so `Weight::DurationOf` + u64-field bound passes.
- `Some(Bound::TargetDuration(field))` (900-913) returns `CapacityDimensionMixing` only under `if weight == Weight::Unit` (line 909-911), so `Weight::Field` + Duration bound passes.
- No downstream agreement check: `SealedCapacity { weight_tail, bound_tail }` (1008-1012) carries the two tails independently, and Capacity.lean:93-97 explicitly declares the dimension gate "validator mechanism; `CapacityLaw` carries none of them as conjuncts" — the validator is the only wall.

**Empirically confirmed**: a temporary test constructing both cross-pairings over the `power_tree()` fixture — `Pool(id) <=[watts]{0..Duration(span)} Device(pool)` (Weight::Field + Bound::TargetDuration) and `Pool(id) <=[Duration(busy)]{0..supply} Device(pool)` (Weight::DurationOf + Bound::TargetField) — passed `validate()` on both (1 passed, 0 failed; file reverted after the run).

**The macro's identical hole** — `/Users/bjorn/Documents/bumbledb/crates/bumbledb-macros/src/lib.rs:1758-1805` (`check_bound_typing`): the `BoundSpec::Field` arm (1767-1780) never reads `weight`; the `BoundSpec::Duration` arm refuses only `matches!(weight, WeightSpec::Unit)` (1793-1801).

**The doctrine sites stating the full table**:
- `/Users/bjorn/Documents/bumbledb/crates/bumbledb/src/error.rs:379-384` — the `CapacityDimensionMixing` doc: "the legal pairings: Duration weights under Duration or literal bounds, u64 weights under u64-field or literal bounds — u64 is u64." This exclusive table refuses both accepted cross-pairings.
- `/Users/bjorn/Documents/bumbledb/docs/design/capacity-laws.md:406-407` — "C18 (dimensions): Duration weights pair with Duration-capable bounds; a count window with a Duration bound is a typed validation refusal."
- `/Users/bjorn/Documents/bumbledb/docs/architecture/30-dependencies.md:268-270` — "Duration weights pair with Duration-capable bounds; a unit (count) window against a Duration bound is a typed validation refusal (ruled 2026-07-24, C18)."
- `/Users/bjorn/Documents/bumbledb/docs/cookbook.md:1574-1578` — "Duration weights pair with Duration-capable bounds, ruled 2026-07-24, C18."

**The unresolved ruling** — `/Users/bjorn/Documents/bumbledb/docs/design/capacity-cutover.md:352` item 11 (verbatim): "`<=[watts]{0..Duration(span)}` and `<={0..Duration(span)}` are representable (both sides u64 under the encoding); enforce Duration↔Duration agreement in validate or document that u64 is u64." The estate did both: the tests pinned "u64 is u64", the error/doctrine text pinned the strict table.

**Tests pinning the narrow reading** (all carry the comment "C18 refuses only the count-window-vs-Duration-BOUND direction"):
- `/Users/bjorn/Documents/bumbledb/crates/bumbledb/src/schema/tests/valid.rs:835-853`
- `/Users/bjorn/Documents/bumbledb/crates/bumbledb/tests/schema_macro.rs:1340-1348`
- `/Users/bjorn/Documents/bumbledb/crates/bumbledb/src/storage/commit/tests/marks.rs:769-772`

The only refusal test is the Unit direction: `/Users/bjorn/Documents/bumbledb/crates/bumbledb/src/schema/tests/reject.rs:1662-1671`.

### Failure scenario / impact

A host declares `Pool(id) <=[watts]{0..Duration(window)} Device(pool)` believing the pairing law documented on `CapacityDimensionMixing` (and in three docs) will refuse it. It validates, seals, and is enforced at every commit as a law comparing summed watts against a nanosecond span — commits are accepted or rejected on a dimensionally meaningless comparison, silently, with no refusal at any wall (validate, macro, or judge; the Lean spec deliberately carries no dimension conjunct). Symmetrically, `[Duration(booked)]{0..head_count}` bounds summed nanoseconds by a person count. The error variant's own doc comment is the user-facing contract, and it is false for two of the three mixing directions.

### Suggested fix

Rule it one way and make every wall agree:
- **(a) Enforce the full table** (matches error.rs and the three doc sites): in `validate_capacity`, refuse `Weight::Field` + `Bound::TargetDuration` and `Weight::DurationOf` + `Bound::TargetField` as `CapacityDimensionMixing`; mirror both in the macro's `check_bound_typing`; update the three "refuses only the count-window direction" test comments and add the two refusal tests (reject.rs + macro-panic test). Note `DurationOf` under `Bound::Lit` stays legal per the table ("Duration weights under Duration **or literal** bounds").
- **(b) Keep "u64 is u64"** (matches the shipped tests): rewrite error.rs:379-384, capacity-laws.md:406-407, 30-dependencies.md:268-270, and cookbook.md:1574-1578 to state that only the unit-vs-Duration direction is refused, and close cutover item 11 with that documentation.

Option (a) matches the stated ruling text and the user-facing error contract; option (b) matches the shipped tests. Either way, the four doctrine sites, two gates, and three pinning tests must land in the same commit.