# 40 — Docs and spec lockstep: the store and the value

- **Status:** OPEN (final pass; lands after 20–23).
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

## Acceptance

- `spec-census.sh` green with the grown token list; no doc names a purged
  surface outside history sections; the proposal's gates match the
  implemented laws.
