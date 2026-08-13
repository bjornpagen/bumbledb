# `query!` interval literals emit a two-argument `Value::Interval*` constructor
- id: 303
- severity: medium
- confidence: confirmed
- area: correctness
- components: crates/bumbledb-query-macros/src/lib.rs, crates/bumbledb-theory/src/value.rs, crates/bumbledb/src/ir/render.rs
- status: fixed (2026-08-13)

## Summary

The documented `query!` grammar admits half-open interval literals `start..end`. The emitter writes `Value::IntervalU64(start, end)` / `Value::IntervalI64(start, end)`, but those enum variants take a **single** `Interval<T>`. Any query using that spelling fails to compile (`E0061`). `schema!` wraps `Interval::new` correctly; `ir::render` still prints `start..end`, so a render → `query!` round-trip of an interval literal is also dead.

## Evidence

Broken emission:

```1427:1433:crates/bumbledb-query-macros/src/lib.rs
            Lit::Interval { start, end } => {
                let variant = if start.signed || end.signed {
                    "IntervalI64"
                } else {
                    "IntervalU64"
                };
                format!("{value}::{variant}({}, {})", int_text(start), int_text(end))
            }
```

The type is unary; the theory crate even has a `compile_fail` that `Value::IntervalU64(7, 7)` is illegal:

```40:43:crates/bumbledb-theory/src/value.rs
    IntervalU64(crate::Interval<u64>),
    /// A nonempty half-open `[start, end)` over I64. Construction follows
    /// [`Value::IntervalU64`].
    IntervalI64(crate::Interval<i64>),
```

`schema!` does the right wrap:

```2217:2221:crates/bumbledb-macros/src/lib.rs
        Value::IntervalU64(interval) => {
            let (start, end) = interval.bounds();
            format!(
                "{path}::IntervalU64(::bumbledb::Interval::<u64>::new({start}, {end})\
                 .expect(\"schema! interval literals are nonempty\"))"
```

The parser accepts the token form (`finish_int` on `..` in `bumbledb-query-macros/src/lib.rs` around 505–512). The module docs advertise `start..end`. Render emits the same spelling (`ir/render.rs` ~514–518). The notation corpus never uses interval literals (params only), so CI never compiled this arm.

## Why this is a bug

A documented, parsed, rendered surface is unusable. Hosts cannot write `5 in 0..10` or `Allen(w, INTERSECTS, 0..10)` in `query!`. Pasting renderer output that contains an interval literal also fails. This is not a silent wrong runtime result — it is a compile-time hole in the blessed query sugar, but it is still a functional defect of the advertised grammar.

## How to trigger / repro sketch

```rust
bumbledb::schema! { pub S; relation R { x: u64, w: interval<u64> } }
let _q = bumbledb_query::query!(S { (x) | R(x, w), 5 in 0..10; });
```

Expected: a `Query` value. Actual: rustc `E0061` — `enum variant takes 1 argument but 2 arguments were supplied` (and a type error treating the integers as `Interval<u64>`).

Same for `Allen(w, INTERSECTS, 0..10)` and for `field == 1..2` selections.

## Related

- `schema!` `value_tokens` (correct)
- `docs/architecture/20-query-ir.md` query notation (`start..end`)
- `crates/bumbledb-query/tests/notation_corpus.rs` (gap: no interval-literal cases)

## Verification (2026-08-12)

Confirmed. `finish_int` (`bumbledb-query-macros/src/lib.rs:505-514`) accepts `start..end` as `Lit::Interval`. `Lit::lit` emits `{value}::{variant}({}, {})` (`lib.rs:1427-1434`) with `value = "::bumbledb::Value"`. That type is `bumbledb_theory::Value` (`crates/bumbledb/src/value.rs` re-export): `IntervalU64(Interval<u64>)` / `IntervalI64(Interval<i64>)` — unary. The theory crate's own `compile_fail` (`value.rs:36-39`) is `Value::IntervalU64(7, 7)`, the same shape. `schema!` wraps `Interval::new` (`bumbledb-macros/src/lib.rs:2217-2221`). `ir/render.rs:514-518` still prints `start..end`. Module docs advertise the spelling (`query-macros/src/lib.rs:104-120`). No `query!` corpus case contains an interval literal, so CI never compiled the arm. Severity stays **medium** (advertised grammar does not compile; not a silent wrong runtime result).

## Resolution (2026-08-13)

`query!` now wraps interval literals in `Interval::new` the same way `schema!` does, so `start..end` compiles; a notation test covers PointIn, Allen, and field-equality spellings.
