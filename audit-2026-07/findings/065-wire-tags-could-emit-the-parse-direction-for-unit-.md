## wire_tags! could emit the parse direction for unit variants, closing its own admitted drift gap

category: unification | severity: medium | verdict: CONFIRMED | finder: ts:bridge

### Summary

The `wire_tags!` macro (`ts/crate/src/tags.rs:47-74`) is the bridge's single source for wire-tag vocabulary, but it emits only the OUT direction: per-tag consts, the exhaustive `tag(&E) -> &'static str` map (the compile tripwire), and the `TAGS` roster. The module doc itself admits the IN direction is unprotected (`tags.rs:17-24`): "The compile tripwire does NOT cover that direction: the parsers keep a catch-all `other =>` refusal arm … a new variant that satisfies this table still needs its parser arm by hand — a miss surfaces at runtime as an 'unknown … kind' refusal." For every enum whose variants are unit-like, the table's pattern column is already a constructor expression, so the table contains everything needed to also emit `parse(tag: &str) -> Option<E>` — which would delete the hand-maintained inverse matches in `marshal.rs` and extend the compile tripwire to both directions for those enums. This is the repo's own representation-first doctrine (`docs/design/representation-first.md`) applied to the crate's centerpiece: the tag tables are the representation; the hand-written parser arms are residual control flow the representation could erase.

### Evidence (all verified against the working tree)

- `ts/crate/src/tags.rs:47-74` — the macro body: emits `$const_name` consts, `tag()`, `TAGS`. No parse function.
- `ts/crate/src/tags.rs:17-24` — the module doc's explicit admission that the parse direction is outside the tripwire and misses surface only at runtime, caught (if at all) by the TS-side integration suite. The stated reason — napi `Object`s cannot be built in a plain `cargo test`, so no in-crate round-trip pins the parsers — explains the missing *test*, not the missing *generated code*: the `kind` strings arrive as plain `String` after `req()`, so a generated `parse(&str)` needs no napi at all.
- `ts/crate/src/marshal.rs:674-686` — `head_term_in`'s inner match: 8 arms `tags::head_op::SUM => HeadOp::Sum, … tags::head_op::PACK => HeadOp::Pack` plus an `other =>` refusal. This is the exact inverse of the `head_op` table at `tags.rs:173-183`, and `HeadOp` is unit-only (`crates/bumbledb/src/ir.rs:280-289`). The `head_op` table's own doc (`tags.rs:170-172`) declares "the old verbatim-duplicate table is dead" — the string-table duplicate died, but this structural-match duplicate of the same table survives one file over.
- `ts/crate/src/marshal.rs:428-435` — the interval-element match, exact inverse of `tags.rs:107-110`; `IntervalElement` is unit-only (`crates/bumbledb-theory/src/schema.rs:72-75`).
- `ts/crate/src/marshal.rs:416-419` — `value_type_in`'s four scalar arms, inverse of the unit subset of the `value_type` table (`tags.rs:95-98`).
- `ts/crate/src/marshal.rs:769-775` — `comparison_in`'s seven scalar `CmpOp` arms (Eq/Ne/Lt/Le/Gt/Ge/PointIn — one more than the finding counted, strengthening it); only `Allen { .. }` carries payload.
- `Direction` (`crates/bumbledb/src/error.rs:920-925`) and `StatementKind` (`crates/bumbledb-theory/src/schema.rs:455-461`) are unit-only but OUT-only today (grep of marshal.rs shows only `statement_kind_out`/`tags::direction::tag` usage at marshal.rs:1005, 1127), so for these two a generated `parse` is future-proofing symmetry, not deleted duplication.

One technical correction to the finding as filed: a `$pat:pat` macro fragment cannot be interpolated in expression position, so `Some($pat)` does not compile from the existing capture. The finding's own fallback — "add a sibling arm for unit-only enums" — is the correct mechanism: capture the variant as `$variant:path`, which substitutes validly in both pattern position (unit-variant path patterns) and expression position (constructors).

### Failure scenario

A new unit variant lands in core (e.g. a ninth `HeadOp`). The `tag()` tripwire fires as designed, the author adds the table row, `tags.json` and the TS unions update, and the crate compiles clean — but `head_term_in`'s hand match at marshal.rs:674-686 was never touched. Every TS program using the new op in a rule head is refused at runtime with "unknown head op", and nothing in-crate can catch it (the module doc concedes no in-crate round-trip exists). The same hole exists independently for `IntervalElement`, the scalar `ValueType` arms, and the scalar `CmpOp` arms. The drift the macro was built to kill (cleanup-0.5.0 U3 kill 10) survives in inverted form.

### Suggested fix

Add a sibling `wire_tags!` arm (or a `wire_tags_unit!` macro) for unit-only enums that captures each variant as `$variant:path` and emits, in addition to the existing consts/`tag()`/`TAGS`:

```rust
pub(crate) fn parse(tag: &str) -> Option<$enum_ty> {
    match tag { $($tag => Some($variant),)+ _ => None }
}
```

Apply it to `head_op`, `interval_element`, `statement_kind`, and `direction` wholesale, and split the mixed tables (`value_type`, `cmp_op`) into a unit section (parse-generated) and a payload section (hand arms stay, per the tags.rs:31-33 doctrine that payload marshaling remains by hand). `head_term_in`'s inner match, the interval-element match, and the scalar value-type/cmp arms each become a one-line `parse(...).ok_or_else(unknown-kind refusal)?` (payload arms fall through before the refusal). A new unit variant then breaks compile in exactly one place — the table — for BOTH wire directions.
