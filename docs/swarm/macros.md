# macros lane board

Owns: `crates/bumbledb-macros/**`, `crates/bumbledb-query-macros/**`, `crates/bumbledb-query/**`,
`crates/bumbledb/tests/{schema_macro.rs, schema-compile-fail/**, compile_fail.rs, compile_fail/**,
query/**}`, root `Cargo.toml` `[workspace] members`.

Items: A (`query!` string literal), C14, C15 (macro side), G6, L (macros).

## Status

| Item | Status |
|---|---|
| A: `query!` string literals compile | landed |
| C14: one proc-macro crate (proc-macro2 + quote), spanned errors, schema-emitted resolution | in progress |
| G6: one compile-fail runner | todo |
| L: retired grammar hints deleted | todo |
| C15: `schema!` calls theory validation at expansion | waiting on engine-storage |

## API changes (announcements)

### Landed

- `query!` string literal selections lower to `Value::String(Box<str>)`.

### Planned

- C14 merges `bumbledb-query-macros` into `bumbledb-macros` and deletes `bumbledb-query`.
  Exact `crates/bumbledb/Cargo.toml` / `lib.rs` lines for engine-storage follow here once the merged
  crate is committed.
