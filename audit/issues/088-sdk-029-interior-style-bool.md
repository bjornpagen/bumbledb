# sdk-029: `query!` interior-atom style is `Option<bool>`

- **Severity:** low
- **Tree:** sdk (rust macros)
- **Status:** FIXED(c3c2884b)
- **Source:** audit/sdk-rest.md #7
- **Depends on:** none (parse-local; same file as sdk-014/015/027 — land with them to avoid merge noise)
- **Conflicts with:** sdk-014, sdk-015, sdk-027 (same `lib.rs`)

## The bug

`crates/bumbledb-query-macros/src/lib.rs:1362-1369,1376-1399` — `interior_style` walks an interior/rec atom's bindings into `Option<bool>`:

```rust
fn interior_style(atom: &Atom) -> Parse<Option<bool>> {
    let mut style: Option<bool> = None;
    // Some(true)  = all bare (ordered dense)
    // Some(false) = all numeric labels (sparse / selection)
    // None        = empty binding list
```

Mixing is a runtime parse error on a pair of flags, the same coordinate as sdk-015's `saw_named` / `saw_index`. Three valid states, encoded as the nullable bool's three inhabitants. Every later reader decodes `Option<bool>` as a style enum.

## Why it's wrong

A bool-plus-null where a three-case sum belongs (Insight 4). The parse already distinguishes the styles; the type says "maybe a bool" (Insight 6). Accidental relative to sdk-015, which filed the param-style twin and stopped at params.

## The fix

Per `audit/CONTRACT.md §C6` (param style is a sum, not two bools) applied to this second site:

```rust
enum BindingStyle { Empty, Bare, Numeric }
fn interior_style(atom: &Atom) -> Parse<BindingStyle>
```

Mixing is unrepresentable. Diagnostics stay the same strings.

## Acceptance criteria

- [ ] Gone: `rg -n 'Option<bool>' crates/bumbledb-query-macros/src/lib.rs` → no `interior_style` match; `interior_style` returns a named enum.
- [ ] Unchanged tests: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-query` green; compile-fail fixtures `mixed_predicate_bindings.rs` / `explicit_dense_positions.rs` still pin mixing, same messages.
- [ ] Green: `cargo test -p bumbledb-query`.

## Constraints

- Parse-local only; zero IR/wire change. Coordinate with sdk-014/015/027 if they touch `lib.rs` in the same wave. No Program vocabulary.
