# 40 — Docs and spec lockstep: the store and the value

- **Status:** **fixed this pass** — proposal two-row table (the store / the
  value); ephemeral, exhume, and sealed-`Instance` sections rewritten as
  purged with add-back triggers; arch docs + cookbook + PUBLISHING
  lockstep; `spec-census.sh` (h) pins the grown deleted-token list;
  Lean untouched (L1–L5 stand). Verify: `bash scripts/spec-census.sh`.
- **Severity:** documentation lockstep (step-15 discipline).

## The changes

1. **`proposals/instance-lifetime.md`** amended: the three-durations table
   becomes two rows — *the store* (mutable, durable, leased reads and
   writes) and *the value* (immutable, proven, owned); the ephemeral
   section, the exhume lines in the format cutover, and the sealed
   `Instance` trait roster are rewritten as purged (with the add-back
   triggers recorded); the gates sections updated (no crash-marker gates;
   `_meta` four-key gate; zero-dyn and alloc-budget gates added).
2. **`docs/architecture/`**: `50-storage` (kind byte, marker law, exhume),
   `70-api` (surface roster), `76-c-abi` (deleted constructor/kinds),
   `61-bench-lanes` (windowed-ephemeral lane deleted; NOSYNC flag lane;
   the three new heap lanes from 39), cookbook recipes, `README` index.
3. **`ts/PUBLISHING.md`**: the 0.15.0 row rewritten for the revised-in-
   place format-8/ABI-3 rule; the rebuild note for pre-publish stores.
4. **Census**: `scripts/spec-census.sh` deleted-token list grows by
   `ephemeral`, `exhume`, `StoreKind`, the persisted-descriptor vocabulary,
   and the `Instance`-trait vocabulary; the zero-dyn census (27) wires in
   here.
5. **Lean: untouched, and said so.** The purge is storage-lifecycle, not
   judgment — L1–L5, the conformance lanes, and the bridge prose need no
   change; this file records that assertion so nobody hunts for phantom
   lockstep work.

## The fix

Present-tense law is one public engine: the store and the proven value.
C ABI stays 3. Format 8 stays 8; the `_meta` roster was revised in place
to four keys (format, fingerprint, generation, dict-next). Kind is not
data. There is no theory-less open.

- Proposal: two-row table in the ruling; sealed-trait roster, format
  cutover, and ephemeral section are **purged** with the recorded
  add-back triggers (tmpfs + hidden NOSYNC flag; exhume from git as a
  CLI, never SDK; a named generic host that cannot hold a concrete type).
  Persistence gate is four keys. Crash-marker gates are gone. Zero-dyn
  and alloc-budget gates added. Lean correspondence states the purge
  does not touch L1–L5.
- Architecture, cookbook recipe 28, and PUBLISHING describe that law
  and do not name purged surfaces outside deleted-vocabulary / add-back
  lines.
- `spec-census.sh` (h) greps the living docs for the deleted tokens and
  allows only `purged` / `add-back` lines.

## Acceptance

- `spec-census.sh` green with the grown token list; no doc names a purged
  surface outside history sections; the proposal's gates match the
  implemented laws (four-key `_meta`, `OpenLane::{Write,Nosync}`, no
  public `Db::ephemeral` / `exhume` / `trait Instance`).
- Lean is untouched: no `lean/` file in this commit.

## Collision

Integration 1 left `exhume` comments in files this lane does not own.
Not edited:

- `crates/bumbledb/src/schema/fingerprint.rs` (test comment "field exhume
  failure")

Related leftover outside this lane: `META_SCHEMA_DESCRIPTOR` key `[5]`
still readable so `verify_store`'s leftover descriptor pass compiles
(`storage/env.rs`, `readtxn.rs`). Bench crate still spells `--ephemeral`
and `Db::ephemeral` (lane D).

## Adjudication

Docs and census only. Engine, C, TS, bench, and Lean were not edited.
