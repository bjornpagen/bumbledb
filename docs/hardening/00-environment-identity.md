# PRD 00 — Environment identity: branding, locking, honest create

Findings fixed (docs/audit/): **concurrency-crash CRITICAL** "Cross-environment
PreparedQuery execution aliases the generation clock" (verified by repro);
**api-schema LOW** "Nothing binds a PreparedQuery to the Db it was prepared
against"; **storage LOW** "`create` refuses only bumbledb environments";
**concurrency-crash NOTE** "Multi-process access is unguarded and corrupts";
**concurrency-crash/api-schema LOW/NOTE** "Nested `db.write` self-deadlocks".

## Purpose

One theme, five findings: the engine has no notion of *which environment* a
piece of derived state or a caller belongs to. Give every environment an
identity and make every cross-identity use either unrepresentable or a typed
error — the generation clock finally learns whose clock it is.

## Technical direction

- **A process-unique environment id.** `storage/env.rs`: `Environment` gains
  `instance: u64`, minted from a `static NEXT: AtomicU64` at `create`/`open`
  (fetch_add, starting at 1 — 0 stays "no environment" forever). Per-process
  uniqueness is exactly sufficient: the memo, the cache, and every other piece
  of derived state are process-local, and a wiped-and-recreated store
  necessarily passes through a new `Environment`. No persistence, no
  randomness, no clock.
- **Plumb it.** `ReadTxn` (and therefore `Snapshot`) exposes
  `env_instance() -> u64` (read from the `Environment` reference it already
  holds — no LMDB read). `PreparedQuery` records it at prepare.
- **The check, at the single entry.** `PreparedQuery::execute` (and through
  it `execute_collect`/`explain`/`profile` — verify all four route through the
  one entry; they do today) compares the snapshot's instance against its own
  as the *first* action and returns a new typed error on mismatch:
  `Error::ForeignPreparedQuery` (document: "a prepared query executes only
  against snapshots of the database that prepared it"). One u64 compare on the
  warm path — noise. With the entry guarded, the memo key needs no epoch: the
  aliasing interleaving (audit repro steps 1–4) dies at step 4 with a typed
  error instead of A's data.
  - Why check-at-entry over lifetime branding: a `&'db Db` brand cannot
    distinguish two `Db`s with equal lifetimes ('static schemas make that the
    common case), so the type-level version is invariant-lifetime ceremony for
    less coverage than one integer compare. The audit's "stronger,
    representation-first fix" is achieved where it matters — the *identity* is
    structural (minted with the environment, unforgeable in safe code); only
    the comparison is runtime.
- **Advisory single-process lock.** `Environment::create`/`open` take an
  exclusive advisory `flock` on `<dir>/bumbledb.lock` (create-if-absent,
  `LOCK_EX | LOCK_NB` via `rustix`? No — no new dependencies: use
  `std::fs::File` + `try_lock` from std's file-locking API if the pinned
  toolchain has it, else `libc::flock` is out too; fall back to an O_EXCL
  lockfile-with-pid scheme ONLY if std lacks `File::try_lock`. Check the
  toolchain first: `File::try_lock` stabilized in 1.89 — the pinned toolchain
  is newer; use it). Held for the `Environment`'s lifetime; a second process
  (or second in-process `Environment` on the same path — also now an error,
  strictly better than today's silent double-open) gets a new typed
  `Error::EnvironmentLocked`. This converts the audit's silent-dictionary-
  corruption interleaving into a loud open-time failure and turns
  00-product's "neither supported nor guarded" into "guarded". Amend
  00-product's sentence in this PRD (the decision stays closed; the guard is
  new).
- **Honest `create` refusal.** `Environment::create`'s existing check ("does
  `_meta` exist") extends: after opening, if `_meta` is absent but the
  environment contains *any other* named database or a non-empty unnamed root,
  refuse with `Error::AlreadyInitialized` (it is someone else's LMDB file).
  The half-created-bumbledb recovery case (crash between dir creation and the
  meta commit → empty root, no named DBs) still proceeds, as the audit noted
  it must. Amend 60-api's sentence to match the implemented rule exactly.
- **Nested-write re-entrancy.** `Db::write` records the owning thread id in an
  `AtomicU64` (or `Atomic<Option<ThreadId>>` via `thread::current().id()`
  packed — `ThreadId::as_u64` is unstable; use a monotonic per-thread key from
  a thread-local counter) beside the writer mutex; on entry, if the stored id
  equals the caller's, panic with a named message ("nested Db::write —
  re-entrant write transactions are forbidden") *before* touching the mutex.
  A loud programmer-error panic beats a silent forever-deadlock; document on
  `Db::write`.

## Non-goals

Persisting environment identity (nothing needs it); cross-process readers
(still out of envelope — the lock now says so loudly); relaxing the
same-schema cross-store case (it becomes `ForeignPreparedQuery` like any other
cross-db use — the docs never promised it, and correctness-by-accident is not
a contract).

## Passing criteria

- **The audit's repro, verbatim, as a regression test** (tests/api.rs): two
  `Db::create`s with the same schema, one distinct fact each (both at
  generation 1), prepare on A, execute on A (correct row), execute on B →
  `Error::ForeignPreparedQuery` — never B-as-A's-data. Plus the
  wipe-and-recreate variant: prepare, drop the Db, recreate the store at the
  same path, open a new Db, execute the old prepared query → the typed error.
- Lock tests: a second `Environment::open` on a live path errors
  `EnvironmentLocked`; dropping the first `Db` releases it and open succeeds.
- Create-refusal test: a directory holding a foreign LMDB environment (create
  one with raw heed in the test, no `_meta`) → `AlreadyInitialized`; the
  empty-root recovery case still creates.
- Nested-write test: `db.write(|_| db.write(|_| Ok(())))` panics with the
  named message (via `catch_unwind`); `write` after a *previous* write on the
  same thread still works (the guard clears).
- **The concurrency regression family the audit asked for** (its final NOTE):
  a test driving `PreparedQuery::execute` on N reader threads while a writer
  commits generations, asserting every result is internally consistent with
  exactly one generation (this directly exercises the machinery this PRD
  touches; it is an in-crate family, not e2e).
- 00-product and 60-api amendments landed. `scripts/check.sh` green.
