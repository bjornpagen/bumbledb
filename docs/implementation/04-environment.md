# PRD 04 — LMDB Environment, Meta, and Storage Keys

Authority: `docs/architecture/40-storage.md` (key layout, widths, open checks,
durability flags), `60-api.md` (open errors).

## Purpose

The LMDB substrate: environment lifecycle, the three databases, `_meta` contents,
open-time verification, and the storage key codec.

## Technical direction

- `storage::env`. `Environment::create(path, &Schema)` and `open(path, &Schema)` over
  heed: three named DBs — `_meta`, `_data`, `_dict`; map size fixed internally
  (envelope: 4 GiB — comfortably ≥ the 1 GB scale axiom); default durability flags
  only (fsync per commit; constructing with NOSYNC/WRITEMAP is not expressible).
- `_meta` keys (single-byte): format version (u32), schema fingerprint (32 bytes),
  storage tx id (u64). `create` writes all three (tx id 0); `open` verifies **format
  version first, then fingerprint**, each mismatch a distinct typed error.
- The workspace error enum starts here: `error` module, one `Error` enum with the
  `60-api.md` taxonomy skeleton (Open/Validation/Runtime/Write categories as variants
  carrying ids, not formatted strings). Local error types from PRDs 01–03 re-home into
  it.
- `storage::keys`: the key codec for `_data`. Namespace tag byte (`F M U R Q S` as
  consts), then big-endian components per `40-storage.md` (relation u32, field u16,
  constraint u16, row_id u64; guard keys embed encoded field bytes). Writers take
  `&mut [u8; MAX_KEY]` scratch (MAX_KEY computed from schema limits, asserted ≤ 511 —
  LMDB's default key ceiling) and return the written length; a fact whose guard key
  would exceed MAX_KEY is a `SchemaError` at declaration time (compute at PRD 02?
  no — key width knowledge lives here; expose a check `Schema`-construction calls;
  wire it in this PRD as a schema-construction hook). **No `[u8; 4096]` zeroing**
  (post-mortem §25); no derived `Ord` on key types.
- Read/write transaction wrappers (thin): `ReadTxn`/`WriteTxn` newtypes over heed
  txns; the reader's **generation** accessor reads the tx id from `_meta` *inside its
  own snapshot* (`40-storage.md` — the race-closing rule, implemented here, used by
  PRD 11).

## Non-goals

The delta/write path (PRDs 06–08). The dictionary DB's contents (PRD 05). The public
`Db` type (PRD 28).

## Passing criteria

- Unit tests: create-then-open round-trips; open with a different schema fails with
  the fingerprint error; a corrupted format version fails with the format error and
  is checked before the fingerprint; key codec round-trips per namespace and orders
  correctly (encoded keys sort by (namespace, components)); oversized-guard-key schema
  is rejected at construction; generation accessor returns 0 on a fresh database.
- `unsafe_code` remains denied in these modules, with one sanctioned exception
  (amended 2026-07-02: heed ≥0.20 marks `EnvOpenOptions::open` unsafe because
  double-opening one environment path in a process is LMDB UB): the single
  `open_env` helper carries `#[allow(unsafe_code)]` and a `// SAFETY:` comment —
  heed's own already-opened registry upholds the invariant. Miri does not apply
  (FFI); the create/open round-trip tests are the module's soundness coverage.
- Global commands green.
