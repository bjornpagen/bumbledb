# sdk-027: `query!` `HeadTerm::Agg` is `over: Option` plus `measure: bool` — leftover `has_over`

- **Severity:** medium
- **Tree:** sdk (rust macros)
- **Status:** FIXED(c3c2884b)
- **Source:** audit/sdk-rest.md #5
- **Depends on:** none (own crate; the engine FindTerm four-case is already the target). Coordinate message-wise with sdk-004 / sdk-008 (`has_over` death) but this file does not touch the ABI.
- **Conflicts with:** none (macros crate; sdk-014/015 are adjacent in the same file — land with or after them to avoid merge noise)

## The bug

`crates/bumbledb-query-macros/src/lib.rs:330-338` — engine `FindTerm = Var | Aggregate | Measure | AggregateMeasure`, but the macro stores three of those cases as one product:

```rust
enum HeadTerm {
    Var(Name),
    Measure(Name),
    Agg {
        op: AggOp,
        over: Option<Name>,
        measure: bool,
    },
}
```

- `parse_agg` (`:594-629`) returns Count as `Agg { op: Count, over: None, measure: false }` before reading an argument; folds store `over: Some`, measure-folds set `measure: true`.
- `Count` with `over: Some(_)` and `Sum` with `over: None` are representable. `measure: true` on Count is representable.
- Emission (`:1750-1767`) re-discovers the engine sum: `None => Aggregate { over: None }`, `Some if measure => AggregateMeasure`, `Some => Aggregate { over: Some }`.

The parse never produces the illegal pairs; every later match re-learns what the type threw away. Wave 1 filed ParsedRule (sdk-014) and param-style bools (sdk-015) in this file and missed the FindTerm encoding — leftover `has_over`.

## Why it's wrong

Insight 4: two independent knobs (`Option` + `bool`) admit four-plus states, a handful valid. Insight 6: the parser knew Count-nullary / fold-with-over / measure-fold and discarded the proof. sdk-004 / sdk-008 kill the same encoding on the C++ / ABI / NAPI side; `query!` still mints it.

## The fix

Per `audit/CONTRACT.md §C6` (`query!` sums; Count carries no `over`; folds require it):

```rust
enum HeadTerm {
    Var(Name),
    Measure(Name),
    Agg { op: AggOp, over: Name },          // Sum/Min/Max/Pack — over required
    Count,                                   // nullary
    AggMeasure { op: AggOp, over: Name },    // Sum/Min/Max of Duration(v)
}
```

(or any four-case split that matches `FindTerm`). `parse_agg` inhabits the right constructor. Emission is a total match, no `if measure`. `over: Option` and `measure: bool` delete.

## Acceptance criteria

- [ ] Gone: `rg -n 'over: Option<Name>|measure: bool' crates/bumbledb-query-macros/src/lib.rs` → no HeadTerm matches.
- [ ] Unchanged tests: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-query` green with zero assertion edits; compile-fail suite still pins Count/measure walls (`measure_under_pack.rs` etc.).
- [ ] Green: `cargo test -p bumbledb-query`; `cargo test -p bumbledb-query-macros` if that package has tests.

## Constraints

- Semantics identical, including Count-nullary and measure-folds under Sum/Min/Max only. Does **not** change the engine `FindTerm` or C ABI (sdk-008 owns `has_over` death). Coordinate with sdk-014 if both touch `lib.rs` in one wave. No Program vocabulary.
