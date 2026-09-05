# Architecture — the shipped product shape

Status: permanent-doc skeleton. Semantics remain normative in
`final-solution/` until proposal retirement (see [README](README.md));
this page records the STRUCTURAL facts that are already true in the
source tree, so consumers and reviewers have one stable orientation page.

## Product

Bumbledb is a set-semantic relational application database: one database
per user/student/tenant, LMDB underneath, warm Free Join and selective
probes first, disk-backed bounded fallback when working sets grow. Typed
schema/query values instead of SQL text; canonical full bytes decide
equality; first-class exact F64 with deterministic sum/mean; grouped
exact measures; application-owned `Id128` identity (no allocator, no
FreshRef). It is not an analytics warehouse, fleet platform or generic
framework.

## Public surfaces

| Surface | Package / crate | Notes |
| --- | --- | --- |
| Rust core | `crates/bumbledb` (`bumbledb`) | Blocking/RAII: `Db::create/open`, `db.read(...)`/`db.write(...)`, `schema!`/`query!`, prepared execution. `publish = false` until crate publication is separately authorized. Log/AWS-free dependency graph — checked by `ts/scripts/absence-gate.ts`. |
| TypeScript core | `ts/` → `@bjornpagen/bumbledb` | Effect-only work (exact Effect `4.0.0-rc.112` peer+dev): `NativeRuntime` service/layer, scoped `Db`/`Snapshot`/`ChangeSet`/`CompleteResult`, `QueryReader`, one-shot page Streams, `Schema.TaggedError` `DbError`. Pure schema/query/scalar construction is synchronous metadata. No Promise/sync/disposal twin. |
| TypeScript log | `ts-log/` → `@bjornpagen/bumbledb-log` | `LocalHistory` (one LMDB transaction), `HostedHistory` (one S3 HEAD over immutable decisions), sealed `Command`s with retained refs, `PublishedSnapshot extends QueryReader`, one native `TenantCache`, explicit admin/migration operations. Imports the core's actual values — no duplicate DSL, codec or cache. |
| Internal Rust log | `crates/bumbledb-log` | The one durable protocol implementation (history, receipts, S3 authority, backup/restore/migration execution). `publish = false`, `#[doc(hidden)]` modules: **not a public Rust log SDK**. |

There is **no C product**: no C crate, headers, exports, examples,
workflows or artifacts anywhere in the release tree (PKG-06; enforced by
the absence gate). Node's internal N-API linkage (`ts/crate`) is an
implementation detail, not a reusable C database SDK.

## One native runtime

Both TypeScript packages load ONE exact-version native artifact per
platform through the core loader (`@bjornpagen/bumbledb-<platform>`
optional dependencies; roster: `darwin-arm64`, `linux-arm64`,
`linux-x64`). The log package re-types the same loaded binding — no
second addon, no cross-addon pointers; duplicate/foreign runtime handles
refuse before use. Importing the core starts no transport, credential
resolution or log maintenance work; the binary contains the internal log
implementation (a measured size cost, not a hidden runtime cost).

## Boundaries that hold everywhere

- Checkpoint/backup/restore/migration exist only in the log product.
- Ordinary log reads are published snapshots; no writable core `Db`
  escapes through a replicated read capability.
- Submit certainty is data: `decided | not-submitted | outcome-unknown`
  in the success channel; interruption stays in `Cause`.
- Errors are direct tagged classes (`DbError`, `ProtocolError`); no
  wrapper error family exists in maintained code.
- Generated migration plans are inert canonical data executed natively;
  users author schemas plus declarative intent, never migration code.
