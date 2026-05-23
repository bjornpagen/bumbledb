# PRD 05: Storage Transaction And Schema Safety

## 01. Status

Not started.

## 02. Severity

Critical storage correctness.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must write failpoint tests before changing transaction commit logic.

The implementer must audit every public mutating method on `WriteTxn`.

The implementer must not add compatibility behavior.

The implementer must preserve duplicate-insert and absent-delete no-op semantics.

## 04. Dependency Order

PRDs 01 through 04 should be complete first.

This PRD must be complete before any further storage layout rewrite.

PRD 06 depends on poisoned-write behavior to protect partial canonical fact rewrites.

PRD 07 depends on schema safety to protect layout compilation.

PRD 16 depends on reliable transaction diagnostics.

## 05. Problem Statement

The write transaction wrapper commits whenever the user closure returns `Ok`.

A user closure can ignore an error returned by a mutating method and still return `Ok`.

Some mutating methods can do partial work before returning an error.

That partial work can include dictionary entries.

That partial work can include canonical fact entries.

That partial work can include fact-id lookup entries.

That partial work can include access entries.

That partial work can include metadata counters.

If the closure ignores the error, those partial writes can commit.

This violates atomic storage semantics.

Schema-dependent operations can also run against an arbitrary `StorageSchema` after raw environment open.

That can read or write data using a schema that does not match the database fingerprint.

## 06. Code Map

Primary files:

- `crates/bumbledb-lmdb/src/lib.rs`.
- `crates/bumbledb-lmdb/src/storage.rs`.
- `crates/bumbledb-lmdb/src/error.rs`.
- `crates/bumbledb/src/lib.rs` if facade open behavior changes.

Relevant current regions:

- `lib.rs:355-384` for `Environment::write` commit behavior.
- `lib.rs:140-176` for raw open and schema-bound open.
- `storage.rs:354-374` for insert path.
- `storage.rs:377-401` for delete path.
- `storage.rs:409-421` for canonical fact write.
- `storage.rs:731-760` for dictionary intern writes.

## 07. Existing Behavior

`Environment::write` opens an LMDB write transaction.

It wraps it in `WriteTxn`.

It calls the user closure.

If the closure returns `Ok`, it commits.

If the closure returns `Err`, it aborts by dropping the transaction.

The wrapper does not know whether a mutating method returned an ignored error.

The wrapper does not know whether partial writes occurred before that ignored error.

The wrapper does not track poison state.

The wrapper therefore cannot enforce all-or-nothing semantics when errors are ignored.

## 08. Concrete Failure Case

Enable a failpoint after canonical fact put.

Start a write transaction.

Call `txn.insert(&schema, fact)`.

The insert writes canonical fact state.

The failpoint returns an error.

The user closure stores the error in `_` and returns `Ok(())`.

`Environment::write` commits.

The canonical fact can now exist without matching access entries or stats.

The database is internally inconsistent.

This must become impossible.

## 09. Schema Safety Failure Case

Create a database with schema A.

Open it later with raw `Environment::open`.

Build a `StorageSchema` for schema B.

Call schema-dependent read or write methods using schema B.

If no fingerprint check occurs, relation IDs and field layouts can be interpreted incorrectly.

This can produce corrupt reads or writes.

The database already stores a schema fingerprint.

The operation must fail before schema-dependent access.

## 10. Desired Transaction Invariants

Any mutating method error after work has begun poisons the write transaction.

A poisoned write transaction cannot commit.

Poison status is checked by `Environment::write` after the closure returns.

Poison status survives even if the user ignores the original error.

A successful no-op outcome does not poison the transaction.

Exact duplicate insert is a successful no-op.

Absent delete is a successful no-op.

Read-only methods do not poison the transaction.

Commit failpoint behavior remains correct.

LMDB still provides final atomic commit or abort.

## 11. Desired Schema Invariants

Schema-dependent operations must use a verified schema.

Schema fingerprint mismatch must be a hard failure.

Raw environment open must not make schema verification optional for normal database use.

If raw open remains public, schema-dependent methods must verify schema or require a verified schema token.

No compatibility path may reinterpret existing data under a different schema.

No migration path may be added.

No dual reader may be added.

## 12. Research Context

An embedded database cannot assume the caller propagates every error correctly.

Transaction wrappers must protect storage invariants even when a closure mishandles an error.

The set-engine rebase increases the number of internal namespaces.

More namespaces mean more ways to commit a partial logical operation if poison is absent.

Future storage PRDs will change canonical fact ownership and guard layouts.

Those rewrites require a strong transaction safety foundation first.

## 13. Transaction Implementation Plan

Add `poisoned: Option<Error>` or equivalent to `WriteTxn`.

Add a helper `mark_poisoned(error: &Error)` or `poison(error)`.

Every public mutating method should call an internal method that marks poison before returning an error from an in-progress mutation.

Consider wrapping mutating method bodies with a helper that maps errors and sets poison.

Do not poison on validation errors that occur before any write or dictionary mutation.

Do poison on dictionary intern errors after metadata or dictionary writes.

Do poison on canonical write errors after canonical namespace writes.

Do poison on access entry write errors after any access write.

Do poison on stats update errors after any logical write has started.

Do poison on failpoints inside mutation paths.

In `Environment::write`, after closure returns `Ok`, check `write.poisoned` before commit.

If poisoned, drop transaction and return a poison error.

## 14. Error Design

Add a storage error such as `PoisonedWriteTransaction`.

The poison error should preserve or expose the original error category if practical.

Tests may match the poison error kind.

Do not hide the original error in logs.

Do not return success from a poisoned transaction.

Do not attempt to roll back manually inside LMDB.

Dropping the write transaction is the rollback.

## 15. Schema Implementation Plan

Preferred design: introduce a schema-bound database handle.

The schema-bound handle is created only by `open_with_schema` or equivalent.

All normal operations use the schema-bound handle.

Raw `Environment::open` remains internal or clearly low-level for tests and metadata operations.

Alternative design: store verified schema fingerprint in `StorageSchema` and verify before each schema-dependent operation.

If per-operation verification is chosen, make it cheap and impossible to forget.

Do not rely on callers to remember `verify_schema`.

Update facade `bumbledb` crate if public API currently bypasses verification.

## 16. Mutating Methods To Audit

`WriteTxn::bulk_load`.

`WriteTxn::bulk_load_streaming`.

`WriteTxn::insert`.

`WriteTxn::delete`.

Dictionary intern helpers.

Canonical fact helpers.

Unique guard helpers.

Reverse-FK guard helpers.

Access entry helpers.

Stats update helpers.

Metadata write helpers.

Any test-only write helpers.

## 17. Required Transaction Tests

Ignored insert error after dictionary write aborts.

Ignored insert error after canonical fact write aborts.

Ignored insert error after unique guard write aborts.

Ignored insert error after reverse-FK guard write aborts.

Ignored insert error after access entry write aborts.

Ignored insert error before stats update aborts.

Ignored stats failpoint error aborts.

Ignored delete error after access delete aborts.

Duplicate insert outcome does not poison.

Absent delete outcome does not poison.

After a poisoned closure returns `Ok`, no partial state is visible in a later read transaction.

## 18. Required Schema Tests

Open database with schema A.

Attempt operation with schema B.

Assert hard schema mismatch failure.

Assert no data is modified by failed schema mismatch operation.

Assert raw metadata reads still work if raw environment open remains public.

Assert facade API uses schema verification by default.

Assert schema mismatch failure does not change stored fingerprint.

## 19. Required Diagnostics

Log poisoned transaction abort at debug level.

Expose no public partial-commit counter unless there is an existing diagnostics surface.

Failpoint tests should show transaction aborts.

Schema mismatch errors should remain clear.

Do not log sensitive fact values.

Do not log dictionary raw bytes.

## 20. Passing Criteria

No write transaction can commit after a mutating method has poisoned it.

Ignored failpoint errors do not commit partial state.

Duplicate insert remains a successful no-op.

Absent delete remains a successful no-op.

Schema-dependent operations cannot run with an unverified mismatched schema.

Existing failpoint tests remain green.

New poisoned-write tests pass.

The global validation gate passes.

## 21. Failure Modes

Only poisoning some write paths is a failure.

Poisoning duplicate insert no-op is a failure.

Poisoning absent delete no-op is a failure.

Returning `Ok` from a poisoned transaction is a failure.

Trying to manually undo LMDB writes inside the transaction is unnecessary and risky.

Adding schema compatibility readers is a failure.

Allowing raw open to bypass normal safety is a failure unless all schema-dependent APIs guard themselves.

## 22. Non-Goals

Do not change canonical fact layout.

Do not change access key layout.

Do not add migrations.

Do not add compatibility shims.

Do not add multi-writer behavior.

Do not change read transaction semantics.

Do not optimize dictionary storage.

## 23. Completion Notes

Document the chosen poison design in code comments near `Environment::write`.

Document the chosen schema-bound design near `Environment::open` and `open_with_schema`.

Keep ignored-error tests permanent.

This PRD is mandatory before changing storage ownership in PRD 06.
