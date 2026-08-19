# 02 — One temporal shape: an async signature means an AsyncTask

- **Status:** OPEN (verified 2026-08-19 17:08 EDT; the tree is hot).
- **Severity:** should-fix; `fromInstance` is the ship-blocking half.
- **Supersedes:** VER-05, and answers the "do we have both a sync and async
  API?" review.

## Principle

SPOV 3 / Insight 12: two coordinate systems for one surface is the special
case factory. The TS API today has two temporal shapes — sync methods and
`async` methods — and the async shape *lies*: of the five `async` entries,
exactly one does off-thread work. An `async` signature that runs sync native
work is a flag (`Promise`) guarding nothing; the event loop blocks exactly as
if the method were sync, but the caller has been told otherwise.

## Evidence (all sync `#[napi]` fns wrapped in `async` TS methods)

| TS method | Native | Work on the JS thread |
| --- | --- | --- |
| `Db.create` | `db_create` (`ts/crate/src/lib.rs:261`) | store birth: LMDB init, publish protocol, fsync chain |
| `Db.open` | `db_open` (`lib.rs:284`) | env open + meta parse (small, fixed) |
| `Db.exhume` | `db_exhume` (`lib.rs:387`) | env open + descriptor decode (small, fixed) |
| `Db.fromInstance` | `db_from_instance` (`lib.rs:336`) | **O(catalog)**: full `_data`/`_dict` copy, commit, fsync chain, atomic rename |
| `builder.admit` | `instance_builder_admit` (`lib.rs:1362`) | **honest**: napi `AsyncTask`, `compute` on the worker pool |

The data plane (`db.read`, `db.write`, instance reads) is sync **by recorded
ruling** (R10/R12: lexical transactions, thenable-refused callbacks) — that
half is correct and stays.

## The law

The temporal shape is data, not decoration:

- **Control plane** (store lifecycle + admission): `async`, and every one is
  a real `AsyncTask`. `AdmitTask` is the in-file template.
- **Data plane** (reads, writes, point reads, execute): sync, per R10/R12.

No third shape. An `async` TS method whose native is sync is unrepresentable
after this change because the wire declaration *is* the AsyncTask return
type.

## The fix

1. `db_from_instance` → `AsyncTask<PublishTask>`: `compute` runs
   `Db::from_instance` off-thread (`OwnedInstance` is `Send + Sync`; hold the
   `Arc<Sealed>` and a lease flag so `dispose` during publish is a typed
   refusal, not a race); `resolve` wraps the `DbHandle`.
2. `db_create` → `AsyncTask<CreateTask>` (it runs the publish protocol and
   admission of the empty candidate — same class as `fromInstance`).
3. `db_open` and `db_exhume` → same treatment for uniformity of the law. The
   work is small, but the rule is worth more than the microseconds: one
   temporal shape, no judgment calls per method.
4. TS signatures are already `async` — no SDK surface change; only the lie
   is removed.

## Acceptance

- Every `async` method on `Db`/`InstanceBuilder` resolves through an
  `AsyncTask` — grep gate: no sync `#[napi]` fn is awaited by the SDK.
- A publish of a large instance does not block a concurrently ticking JS
  timer (test: interval keeps firing during `fromInstance`).
- Dispose-during-publish returns the typed spent-handle refusal.
