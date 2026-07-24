## SchemaWarning is computed, sealed, and then unreachable through the Db lifecycle

category: incoherence | severity: medium | verdict: CONFIRMED | finder: engine:schema-api

### Summary

Schema validation computes a non-fatal diagnostic — `SchemaWarning::RedundantSuperkey`, whose own doc comment says the redundant key "adds determinant writes without strengthening the theory" (`crates/bumbledb/src/schema.rs:536-545`) — and seals it into the `Schema` witness (`warnings` field, schema.rs:530; populated by `redundant_superkeys` at `crates/bumbledb/src/schema/validate.rs:243`). But every production path that runs validation — `Db::create`, `Db::open`, `Db::ephemeral` (`crates/bumbledb/src/api/db/open.rs:22`, `:36`, `:68`) — hands the sealed Schema straight into `assemble` and never surfaces the warnings. The handle's only route to the sealed witness is the crate-private accessor `Db::schema()` (`crates/bumbledb/src/api/db.rs:337`, `pub(crate)`, doc-commented "reader: `crate::verify_store`"). No method on `Db`, `Snapshot`, `WriteTx`, or `PreparedQuery` exposes warnings; the crate-root bindings roster (`crates/bumbledb/src/lib.rs:172-176`) exports `SchemaError` but omits `SchemaWarning`. The only consumers of `Schema::warnings()` in the repository are two tests (`src/schema/tests/valid.rs:56`, `tests/schema_macro.rs:1115`).

### Evidence

- `crates/bumbledb/src/api/db.rs:336-339` — `pub(crate) fn schema(&self) -> &Schema { &self.schema }`: the sealed witness is crate-private on the handle.
- `crates/bumbledb/src/api/db/open.rs:22,36,68` — `let schema = schema.descriptor().validate()?;` then `assemble(...)`: warnings computed, then buried.
- `crates/bumbledb/src/schema.rs:529-531` — `warnings: Box<[SchemaWarning]>` field comment: "Non-fatal declaration diagnostics sealed alongside the witness."
- `crates/bumbledb/src/schema.rs:632-636` — the public `Schema::warnings()` accessor exists but is only reachable on a `Schema` value the host obtained itself.
- `crates/bumbledb/src/lib.rs:172-176` — root roster re-exports `Schema`, `SchemaDescriptor`, `SchemaError` (line 170) etc., but not `SchemaWarning`; `ValidateDescriptor` is likewise absent from the roster (it lives only at `bumbledb::schema::ValidateDescriptor`, schema.rs:56).
- Doc contract checked: `docs/architecture/70-api.md:730-733` ("Schema warnings: an accepted sealed schema exposes `Schema::warnings()`... warnings are never errors and never alter the fingerprint") names the surface; `docs/architecture/70-api.md:307-308` places `.validate()` "inside `Db::create`/`Db::open`" — the entry that discards the schema; the normative bindings roster (`70-api.md:320-325`) lists `SchemaError` but not `SchemaWarning`. `docs/architecture/30-dependencies.md:442-446` confirms the warning's purpose is exactly the write-amplification diagnostic and that it is "diagnostics only".

Two corrections to the original finding, neither of which changes the verdict: `SchemaWarning` is publicly nameable via `bumbledb::schema::SchemaWarning` (`pub mod schema`, lib.rs:107 — the integration test at `tests/schema_macro.rs:1091` imports it by that path), so the type is off-roster rather than hidden; and there are two test consumers of `warnings()`, not one.

### Failure scenario

A host declares `Account(id) -> Account` plus `Account(id, holder) -> Account`. Validation seals a `RedundantSuperkey` warning; `Db::create` succeeds; every commit thereafter pays the redundant determinant's writes. No API call on the handle can ever report the diagnostic — it exists only inside a private field behind a `pub(crate)` accessor. The one escape hatch is re-running `Ledger.descriptor().validate()` through the `ValidateDescriptor` trait (itself absent from the documented bindings roster), i.e. validating the same schema twice to read a value the handle already owns. This is the incoherence: the documentation positions validation as internal to `Db` construction, and positions `Schema::warnings()` as the diagnostics surface, but construction swallows the only object that carries it.

### Suggested fix

Expose the sealed diagnostics on the handle — `pub fn schema_warnings(&self) -> &[SchemaWarning]` on `Db<S>` (one slice borrow of the already-owned witness, zero recompute, zero allocation) — and add `SchemaWarning` to the lib.rs root roster next to `SchemaError`, updating the `70-api.md` bindings roster to match. Alternatively (representation-first): have `Db::create`/`open` return the warnings alongside the handle so the diagnostic is impossible to miss at the one moment it is actionable.
