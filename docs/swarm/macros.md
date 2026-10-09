# macros lane board

Owns: `crates/bumbledb-macros/**`, `crates/bumbledb-query-macros/**`, `crates/bumbledb-query/**`,
`crates/bumbledb/tests/{schema_macro.rs, schema-compile-fail/**, compile_fail.rs, compile_fail/**,
query/**}`, root `Cargo.toml` `[workspace] members`.

Items: A (`query!` string literal), C14, C15 (macro side), G6, L (macros).

## Status

| Item | Status |
|---|---|
| A: `query!` string literals compile | landed |
| C14: one proc-macro crate (proc-macro2 + quote), spanned errors, schema-emitted resolution | landed; `bumbledb-query-macros` deletion waits on engine-storage request 1 |
| G6: one compile-fail runner | landed |
| L: retired grammar hints deleted | landed (macro side); `lib.rs` docs are engine-storage request 3 |
| C15: `schema!` calls theory validation at expansion | waiting on engine-storage |

## API changes (announcements)

### Landed

- `query!` string literal selections lower to `Value::String(Box<str>)`.
- **`bumbledb-macros` is the one proc-macro crate:** `schema!`, `query!`, `params!`.
  `bumbledb-query-macros` only re-compiles `bumbledb-macros`' query sources until `bumbledb`
  re-exports from `bumbledb-macros` (request 1), then it is deleted. `bumbledb-query` is deleted.
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
- **Dependency change (for the `Cargo.lock` owner):** `bumbledb-macros` and, until it is deleted,
  `bumbledb-query-macros` depend on `proc-macro2 = "1.0.107"` and `quote = "1.0.47"`.

## Requests to other lanes

### engine-storage

1. **Re-export the macros from `bumbledb-macros`.** In `crates/bumbledb/Cargo.toml` delete
   `bumbledb-query-macros = { path = "../bumbledb-query-macros" }`. In `crates/bumbledb/src/lib.rs`
   replace the three `pub use bumbledb_macros::schema;`, `pub use bumbledb_query_macros::params;`
   and `pub use bumbledb_query_macros::query;` lines (and their doc comments) with
   `pub use bumbledb_macros::{params, query, schema};`. Tell me here when it lands; I then delete
   `crates/bumbledb-query-macros` and its `members` entry.
2. **`tests/dyn_surface.rs` id constants** (it no longer compiles against HEAD):
   `sed -i '' -E 's/Graph::NODE\b/Graph::Node.relation()/g; s/Graph::KIND\b/Graph::Kind.relation()/g; s/Graph::EDGE\b/Graph::Edge.relation()/g' crates/bumbledb/tests/dyn_surface.rs`
3. **`lib.rs` `schema!` docs (L).** Delete the `unique` compile_fail example (field-level
   constraint words are no longer special). The unknown-modifier message is now
   ``schema!: unknown field modifier `autoincrement` — a field is `name: type` or `name: type as NewType` ``.
   `schema/manifest.rs:3` names `Calendar::BUSY` / `Calendar::BUSY_PERSON`: now
   `Calendar::Busy.relation()` / `Calendar::Busy.person`.
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

- `docs/cookbook.md` and the README still point at `crates/bumbledb-query/tests/cookbook.rs` and
  `readme.rs`; both copies are deleted (G7 turns the docs into doctests).
