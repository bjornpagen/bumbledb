# 20 — Purge ephemeral: one store kind, kind stops being data

- **Status:** **fixed this pass** — `StoreKind`, `META_STORE_KIND`, and the ephemeral constructors (`Db::ephemeral`, `bdb_db_ephemeral`, marker lifecycle) deleted; hidden `create_nosync`/`open_nosync` remain for the bench; tests: `create_then_open_round_trips`, `parse_meta_reads_five_keys`, `create_open_close`.
- **Severity:** purge — the largest single deletion.
- **Owner ruling:** ephemeral's mutable-no-durability niche is covered by a
  durable store on tmpfs (correctness) and a bench-private NOSYNC lane flag
  (perf). No product consumer exists: TS deliberately has none, and the only
  callers are the bench harness and the C mirror of the harness.

## Principle

REQUIRED-READING step 1: the requirement "a no-durability store kind is
product API with crash-recovery semantics" has no named owner. The marker
lifecycle keeps a promise ("we detect our own crashes") nobody asked for.
The real requirement underneath is a NOSYNC open flag on a throwaway store —
a bench knob, not a store kind.

## Cascade (verified against the tree)

- Public: `Db::ephemeral`, `Db::ephemeral_from_instance`,
  `bdb_db_ephemeral` (+ its admission arm and docs).
- Engine: all of `storage/env/ephemeral.rs` (classifier, probe, wipe,
  marker arm/disarm); the dirty-marker sibling-file law;
  `clear_orphan_marker` at create; the `Drop`-time marker clear;
  `EnvMode`'s ephemeral arm (see 23 — the enum deletes entirely);
  **`StoreKind` deletes as an enum** (a one-arm sum is a unit);
  `META_STORE_KIND` leaves `_meta` (see 23); the NOSYNC lane in
  `open_env.rs` moves behind a bench-only entry (below).
- `publish` loses its kind **parameter** — kind is not data anymore.
- Errors: `StoreKindMismatch` and ephemeral lifecycle kinds leave `Error`;
  their C kinds leave `bdb_error_kind` (the wildcard-free map forces every
  site); the TS `ErrorFamily` rows and `tags.json` entries go.
- Bench: `storemode`'s sum collapses; the windowed-ephemeral lane deletes;
  write lanes re-anchor on a crate-private `NosyncLane` open flag — this
  flag is the planned 10%-back and lives in the bench crate, not the
  engine's public surface.
- Tests: `tests/ephemeral.rs`, marker/crash tests, C ephemeral tests, the
  `ephemeral_wipe` obs event.
- Docs/census: see 40.

## Acceptance

- `grep -rni "ephemeral" crates/bumbledb/src crates/bumbledb-c/src ts/src`
  is empty (bench crate may keep the NOSYNC flag under its own name).
- `StoreKind` does not exist; no store-kind byte is written or read.
- All suites green; the NOSYNC bench flag produces a write-lane baseline
  recorded in 33.

## Adjudication

The engine, C, and TS deleted surfaces landed here. The bench crate
(`storemode`, the windowed-ephemeral lane, `--ephemeral`) is lane D
(issue 33): this pass leaves the hidden Nosync constructors as the
re-anchor and does not edit `crates/bumbledb-bench`. Two historical
"ephemeral p50s" comments remain in `storage/commit/judgment.rs` and
`exec/run.rs` (lane C).
