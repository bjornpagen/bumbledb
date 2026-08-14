# sdk-014: `query!` `ParsedRule` is a kind-flag plus optional name — `expect` where a sum belongs

- **Severity:** medium
- **Tree:** sdk (rust macros)
- **Status:** OPEN
- **Source:** audit/sdks.md #14
- **Depends on:** none (parallel-safe; own crate)

## The bug

`crates/bumbledb-query-macros/src/lib.rs:341-357`:

```rust
enum RuleKind {
    Bare,
    Interior,
    Recursive,
}

struct ParsedRule {
    kind: RuleKind,
    /// ... `None` for a bare main rule.
    name: Option<Name>,
    head: Vec<HeadTerm>,
    items: Vec<Item>,
}
```

`Bare + Some(name)` and `Interior + None` are representable. Emission then does `rule.name.clone().expect("interior rules carry a name")` (`lib.rs:1883`, and again near `:1933`) — a panic on an illegal state the type admitted. The parser never produces those pairs; every later match re-learns what the type threw away.

## Why it's wrong

Parse-don't-validate (Insight 6): the parse KNOWS whether a name exists at the moment it reads the keyword; encoding that as a flag plus a nullable sidecar discards the knowledge and re-derives it with `expect`s downstream (Insight 4: the product admits 6 states for 3 meanings).

## The fix

Per `audit/CONTRACT.md §C6` (Rust `query!`): the sum from the audit, name inside the carrying constructors —

```rust
enum ParsedRule {
    Bare { head: Vec<HeadTerm>, items: Vec<Item> },
    Interior { name: Name, head: Vec<HeadTerm>, items: Vec<Item> },
    Recursive { name: Name, head: Vec<HeadTerm>, items: Vec<Item> },
}
```

(or a shared `RuleBody { head, items }` payload struct if the three bodies stay identical). `RuleKind` deletes; the `expect`s delete; emission matches.

## Acceptance criteria

- [ ] Gone: `rg -n 'enum RuleKind' crates/bumbledb-query-macros/src` → no matches; `rg -n 'expect\("interior rules carry a name"\)' crates/bumbledb-query-macros/src` → no matches; `rg -n 'name: Option<Name>' crates/bumbledb-query-macros/src` → no matches on the rule struct.
- [ ] Unchanged tests: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-query` green INCLUDING the compile-fail suite (`named_head_without_keyword` and phase-order fixtures byte-identical — the diagnostics must not change).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-query`.

## Constraints

- Macro OUTPUT (generated code and spanned error messages) byte-identical — this is an internal parse-representation change only. No Program vocabulary.
