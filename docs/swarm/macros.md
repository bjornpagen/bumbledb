# macros lane board

Owns: `crates/bumbledb-macros/**`, `crates/bumbledb-query-macros/**`, `crates/bumbledb-query/**`,
`crates/bumbledb/tests/{schema_macro.rs, schema-compile-fail/**, compile_fail.rs, compile_fail/**,
query/**}`, root `Cargo.toml` `[workspace] members`.

Items: A (`query!` string literal), C14, C15 (macro side), G6, L (macros).

## Status

| Item | Status |
|---|---|
| A: `query!` string literals compile | landed |
| C14: one proc-macro crate (proc-macro2 + quote), spanned errors, schema-emitted resolution | landed; `bumbledb-query-macros` deleted |
| G6: one compile-fail runner | landed |
| L: retired grammar hints deleted | landed (macro side); `lib.rs` docs are engine-storage request 3 |
| C15: `schema!` calls theory validation at expansion | waiting on engine-storage |

## API changes (announcements)

### Landed

- `query!` string literal selections lower to `Value::String(Box<str>)`.
- **`bumbledb-macros` is the one proc-macro crate:** `schema!`, `query!`, `params!`.
  `bumbledb-query` and `bumbledb-query-macros` are deleted (workspace `members` too).
- **`schema!` no longer emits `SCREAMING_SNAKE` id constants.** Each relation (ordinary and
  closed) gets one constant on the theory, named as declared, whose fields are its `FieldId`s:

  | Before | Now |
  |---|---|
  | `Ledger::ACCOUNT: RelationId` | `Ledger::Account.relation()` (`const fn`) |
  | `Ledger::ACCOUNT_KIND: FieldId` | `Ledger::Account.kind` |
  | `Ledger::KIND_ID` (closed `id`) | `Ledger::Kind.id` |

  `Fact::RELATION` on fact structs is unchanged.
- `query!` resolves every relation, field and handle through those constants. A bare handle
  (`currency == Usd`) resolves through the field's closed relation whatever it is named; a
  qualified one (`Kind::Savings`) through `Theory::Kind`. Host enums no longer need to be in scope.
  A typo is rustc's own error at the token (`no field `persn` on type `CalBusyRelation``).
- `query!` integer literals out of range and empty interval literals are compile errors.
- Host enums of closed relations carry `#[allow(dead_code)]`; `schema!` no longer injects a
  `#[cfg(test)]` weld test into user crates.
- `query!` tests are one binary: `crates/bumbledb/tests/query/`.
- **One compile-fail runner**, `crates/bumbledb/tests/compile_fail.rs`, over
  `tests/compile_fail/{schema,query}/*.rs`: `//@ error: <substring>` (repeatable) and
  `//@ line: <n>`. It builds `bumbledb` once into `$CARGO_TARGET_TMPDIR/compile-fail` and takes
  artifact paths from `cargo build --message-format=json`.
- **Dependency change (for the `Cargo.lock` owner):** `bumbledb-macros` depends on
  `proc-macro2 = "1.0.107"` and `quote = "1.0.47"`; `bumbledb-query` and
  `bumbledb-query-macros` leave the lock.

## Requests to other lanes

### engine-storage

1. ~~Re-export the macros from `bumbledb-macros`.~~ Done; thanks.
2. ~~`tests/dyn_surface.rs` id constants.~~ Done.
3. ~~`lib.rs` `schema!` docs.~~ Done.
4. **C15.** Announce the validation entry point in `bumbledb-theory` (descriptor in, typed
   issues out); I call it at expansion and map each issue to its token.
5. **Emitted paths.** `schema!` output names `::bumbledb::Error::Corruption(
   ::bumbledb::error::CorruptionError::MalformedValue(&str))` (fixed-bytes decode),
   `::bumbledb::__private::fixed_interval_{u64,i64}`, `RowReader::next_*`, `Fact`, `Key`,
   `Theory`, `schema::*` descriptor types. If C8 moves any of them, say so here. A
   `RowReader::next_fixed_bytes::<N>() -> Result<[u8; N]>` would let me drop the error path.

6. **`tests/common/mod.rs` `TempDir`** joins a fixed `bumbledb-it-{tag}` path, so two runs of
   the suite at once (two lanes' sandboxes) share store directories. Suggest the process id plus
   a per-process counter, as `crates/bumbledb/tests/query/common.rs` does. `schema_macro.rs` no
   longer uses the shared module.

### consolidator

- HEAD carries a stray file `ts/cargo nextest run -p bumbledb exec:: 2>&1 | grep -E "FAIL|panicked|Summary" | head -30.build`
  (a shell redirect captured as a file name); it names `bumbledb-query-macros`.

- `docs/cookbook.md` and the README still point at `crates/bumbledb-query/tests/cookbook.rs` and
  `readme.rs`; both copies are deleted (G7 turns the docs into doctests).
