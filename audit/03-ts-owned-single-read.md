# 03 — Two spellings of every owned read; the lease is a store coordinate on the heap arm

- **Status:** OPEN (verified 2026-08-19 ~17:00 EDT; the tree is hot).
- **Severity:** should-fix.
- **Supersedes:** VER-06.

## Principle

Insight 12 (coordinate artifacts) and "a single way to do it." The lease
(`read(fn)` scoping a capability) exists to bound an LMDB read transaction —
a *store* coordinate. A frozen catalog has no transaction and never changes:
there is nothing to scope. `OwnedInstance` nonetheless offers **two spellings
of every read** — `instance.read(fn)` and per-op convenience methods — and
the per-op methods are implemented *as* degenerate one-op leases, so the
store coordinate was imported into the heap arm twice.

## Evidence

- `ts/src/db.ts:1734-1752` — `withInstance` routes `prepare`, `execute`,
  `scan`, `get`, `contains` through `native.ownedRead(handle, cb)`: one
  native→JS-callback→native round trip, one `createReadInstance` wrapper,
  one `instanceStates` WeakMap entry, and one thenable probe **per
  operation**, ending in the `captured ?? result` double-return hack
  (`db.ts:1752`).
- `ts/crate/src/lib.rs:1382-1392` — `owned_read` mints an
  `InstanceHandle::heap` (Arc clone + `External::new` + alive flag flip) per
  call.
- `ownedReclaimer` / `builderReclaimer` (`db.ts:1708-1724`) never
  `unregister` on dispose — the double-close is swallowed, but the registry
  holds dead entries.

## The fix — one way to read an owned instance

1. Direct native entries on `OwnedHandle`: `ownedScan`, `ownedContains`,
   `ownedGet`, `ownedExecute`, `ownedPrepare` — plain calls on a
   `Send + Sync` value with the existing `live()` check. No lease, no
   callback, no per-op allocation.
2. **Delete `OwnedInstance.read(fn)`.** The lexical form buys consistency
   scoping on a store; on an immutable catalog every op sees the same state
   forever, so the grouping form is pure API symmetry — a second way with no
   semantic content. Hosts that write generic read code type against the
   shared method surface (`scan`/`get`/`contains`/`execute`), which both
   `ReadInstance` and `OwnedInstance` structurally satisfy.
3. The `captured ?? result` hack, the per-op `instanceStates` bookkeeping,
   and `owned_read` itself delete with the lease.
4. While there: `FinalizationRegistry.unregister` on `Symbol.dispose` for
   both reclaimers.

## Single way

A store read is a lease (`db.read(fn)` — transaction to scope). A heap read
is a method call (nothing to scope). The shape difference *is* the semantic
difference; giving both arms both shapes hid it.

## Acceptance

- `owned_read` and `OwnedInstance.read` are gone; the five direct entries
  exist; `pnpm test` green.
- A hot `get` loop over an owned instance allocates no per-call handle
  (heap-growth assertion or allocation-count probe).
- A generic host function typed against the common read surface compiles
  against both `ReadInstance` and `OwnedInstance` (type-level test).
