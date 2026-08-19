# 06 — One instance algebra: the store arm still reads through `storage::read` and `dict::*`

- **Status:** **keep** (prepare/execute generic; scan residue essential) —
  a lending cursor cannot borrow a temporary catalog. Two scan/get/contains
  bodies are the coordinate change, not a dual algebra.
- **Severity:** should-fix.
- **Supersedes:** SPINE-03, SPINE-05, SPINE-06, SPINE-07, PROP-005, PROP-009,
  PROP-017, REP-12/13/14.
- **Adjudication (third pass): keep ACCEPTED for the scan half only; the
  file NARROWS, it does not close.** The lending-cursor/borrow-order
  constraint is real for `scan`. It does not cover the rest of this file:
  the store `CodecRead` still resolving through `dict::*` free functions
  while the heap arm uses `catalog.dict_*` (two dictionary paths — no
  cursor involved) and the two scratch pools. Those remain open. The
  `execute_args` rustdoc ghosts are **fixed this pass** (`execute` is the
  one named entry). If scan unification is ever re-attempted,
  the answer on record is a catalog member built at lease birth (not a
  temporary), so the cursor borrows a value that lives as long as the
  source.

## Principle

Homogeneous coordinates (Insight 12): `FrozenSource` vs `LmdbSource` is the
coordinate change; the algorithms above must be written once. Every place the
store arm bypasses `CatalogRead` into `storage::read`/`dict::*` free
functions is a second instance algebra — a new point-read kind costs two
files, and the codec resolves dictionaries two different ways.

## Evidence

- `crates/bumbledb/src/api/db/read_instance.rs:6` — `use crate::storage::read`
  survives; store `scan` lends through the owned txn instead of
  `CatalogRead::scan_facts` (the second-pass note: "a temporary catalog
  cannot host a lending cursor").
- `plumbing.rs` — store `CodecRead` uses `dict::lookup`/`dict::resolve`
  free functions; the heap arm uses `catalog.dict_*`.
- Two scratch pools: the lease borrows the handle pool, the owned instance
  owns its core pool (SPINE-07).
- Rustdoc ghosts: `execute_args` still named in docs;
  `ReadInstance::execute(BindArgs)` beside `Instance::execute(&[ParamArg])`
  (PROP-009 / REP-12/13).

## The fix

1. **Give `LmdbSource` a real catalog member.** The "temporary catalog
   cannot host a lending cursor" blocker is a construction-order problem,
   not a type problem: `LmdbSource` owns (or borrows) one `LmdbReadCatalog`
   built at lease birth, so `scan_facts`'s lending cursor borrows a value
   that lives as long as the source. Store `scan`/`get`/`contains` then go
   through `CatalogRead` exactly as the heap arm does.
2. `plumbing.rs`'s store `CodecRead` resolves through the source catalog's
   `dict_lookup`/`dict_resolve`; the `dict::*` free functions become thin
   delegates of `LmdbReadCatalog` or delete (PROP-017's sequencing note is
   now satisfied — the spine landed).
3. One `ScratchPool` inside `InstanceCore`; `Db::read` seeds/parks it, the
   lease borrows nothing else from the handle.
4. Docs and inherent methods: delete the `execute_args` ghosts; keep
   `BindArgs` only as a documented conversion into `&[ParamArg]` at the one
   trait entry. Inherent wrappers that merely call the trait are fine
   (REP-14 stands).

## Single way

Every instance point read — heap or store — enters through
`CatalogRead`; every dictionary resolution enters through the source's
catalog; every execute enters through the one trait method.

## Acceptance

- `read_instance.rs` has no `use crate::storage::read` and no `dict::` call.
- `grep -rn "storage::read::" crates/bumbledb/src/api/` is empty.
- One `ScratchPool` type reachable from instances; the handle-field borrow
  is gone.
- No rustdoc names `execute_args`; workspace tests and scenario lanes
  byte-identical.
