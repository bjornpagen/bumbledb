# sdk-015: `query!` param style tracked as two bools

- **Severity:** medium
- **Tree:** sdk (rust macros)
- **Status:** FIXED(c3c2884b)
- **Source:** audit/sdks.md #15
- **Depends on:** none (parallel-safe; same crate as sdk-014 — one fixer may take both)

## The bug

`crates/bumbledb-query-macros/src/lib.rs:1265-1270, 1276-1304` — `saw_named` and `saw_index` booleans track which param style the query uses. Four states, of which two are valid after first use and one at start; mixing styles is a runtime parse error discovered by inspecting the pair.

## Why it's wrong

A two-bool product for a three-state machine (Insight 4): `saw_named && saw_index` is representable, and the mixing refusal is a flowchart over the flags instead of a transition that cannot be taken.

## The fix

Per `audit/CONTRACT.md §C6` (Rust `query!`): a sum —

```rust
enum ParamStyle {
    Empty,
    Named(Vec<Name>),   // or the existing name registry
    Index,
}
```

First resolution moves `Empty → Named/Index`; a resolve against the other arm is the (unchanged) spanned mixing error, emitted from the match arm rather than from flag inspection. Both bools delete.

## Acceptance criteria

- [ ] Gone: `rg -n 'saw_named|saw_index' crates/bumbledb-query-macros/src` → no matches.
- [ ] Unchanged tests: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-query` green; the mixing-error compile-fail fixture (if present) byte-identical; if absent, ADD one (`compile-fail/params_mixed_styles.rs`) as the new lock.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-query`.

## Constraints

- Spanned diagnostics byte-identical for existing fixtures. Internal representation only.
