# 12 — C handle pairs are unchecked at the bridge

- **Status:** **fixed this pass** — `OwnerToken::{Store,Heap}` on
  `bdb_instance_ref` and `bdb_prepared`; `bdb_instance_execute` compares
  before the engine. Foreign pairing is `ForeignPreparedQuery` (the kind
  hosts already match). Test: `foreign_prepared_is_refused_at_the_bridge`.
- **Severity:** should-fix.
- **Supersedes:** BND-05.

## Principle

The proposal's dual-check law (already implemented on the TS side): the
bridge produces an immediate host-quality refusal for a foreign pairing; the
engine remains the safety net against forged handles. Today
`bdb_snapshot_execute(instance_a, prepared_b)` and
`bdb_db_write_from(db_a, witness_b)` sail through the bridge and are caught
only by the engine's `ForeignPreparedQuery` / `ForeignWitness`.

## Evidence

- `crates/bumbledb-c/src/query.rs`, `answers.rs` — no owner/identity field
  on `bdb_prepared`; no bridge-side compare before entry.
- Precedent in-file: `bdb_tx_ref` already carries its owner pointer for
  `fresh_field` — the shape exists, two handle kinds simply lack it.

## The fix

1. Mint the owner token — the `CatalogIdentity` `Arc` address (stable for
   the life of the owner) — into `bdb_prepared`, `bdb_instance_ref`, and
   `bdb_witness` at creation.
2. Compare at every pairing entry (`execute`, `profile`, `write_from`)
   before touching the engine; mismatch is the typed bridge refusal (with
   the error-origin split, a *bridge* kind, not an engine impersonation).
3. The engine check stays — the bridge check is quality, not safety.

## Acceptance

- Pairing a prepared handle with a foreign instance ref returns the bridge
  refusal without entering the engine (test pins the origin tag).
- The engine-side foreign tests still pass unchanged (safety net intact).
