# 21 — Purge exhume and the persisted descriptor it was born with

- **Status:** **fixed this pass** — `exhume`/`Exhumed` deleted on Rust, C (none), and TS; open-time descriptor compare gone; `DescriptorMissing`/`DescriptorRoundTrip` gone; tests: `create_then_open_round_trips`, `a_missing_descriptor_is_meta_missing_not_an_adoption`, `create_open_close`.
- **Severity:** purge.
- **Owner ruling:** exhume answers "I have bytes but not code" — a failure
  mode the store's owner essentially cannot have (the schema is source code
  in the same repo as the app). The migration story it theoretically served
  (dyn-scanning a store through napi `Value` marshaling) was never serious;
  a real migration is reads-and-writes with two schemas you both possess.

## Principle

Step 1 again: built in `c79c2b38` ("self-describing stores… exhume reads it
theory-less") for an unowned requirement. The descriptor's **decode** half
has exactly two consumers — the codec itself and `api/db/exhume.rs` — so
the self-describing machinery dies with its only reader.

## Cascade (verified against the tree)

- Engine: `api/db/exhume.rs` + tests, `Exhumed`, the free `exhume()`.
- TS: `Db.exhume`, `exhume.ts`, `db_exhume`, `ExhumeTask` (one of the five
  AsyncTasks), `ExhumeOutcome` and its three refusal variants, the exhume
  rows in `tags.json`.
- C: nothing — it never had exhume (verified: zero hits).
- `_meta`: `META_SCHEMA_DESCRIPTOR` leaves the roster (see 23). The
  descriptor **encode** half stays — the fingerprint is the blake3 of the
  canonical encoding. The decode half of `schema/descriptor_codec.rs`
  deletes; the open-time descriptor check deletes; `verify_store`'s
  descriptor↔fingerprint pass deletes;
  `strip/overwrite_schema_descriptor_for_tests` delete.
- Errors: `DescriptorMissing`; `CorruptionError::DescriptorRoundTrip`;
  their C kinds and TS families.
- `EnvMode::Exhume` arm and the R17 no-advisory-lock open lane (see 23).

## Add-back trigger (recorded)

A real bytes-without-code incident resurrects exhume **from git as a
standalone CLI tool**, never as SDK surface.

## Acceptance

- `grep -rni "exhume\|descriptor_codec::decode" crates ts/src ts/crate` is
  empty outside git history.
- Fingerprints unchanged (encode half untouched): a store created before
  and after this change carries the same fingerprint for the same schema.
- All suites green.

## Adjudication

Owned exhume surfaces landed here: `api/db/exhume.rs`, `storage/env/exhume.rs`,
`Exhumed`, TS `Db.exhume`/`ExhumeTask`/`ExhumeOutcome` and tags, open-time
descriptor compare, `DescriptorMissing`/`DescriptorRoundTrip` and their C
kinds, the R17 `OpenLane::ReadOnly` arm, and
`strip`/`overwrite_schema_descriptor_for_tests`. The descriptor **decode**
half of `schema/descriptor_codec.rs` and the unused `wire.rs` decode helpers
are lane B — deleting them here would collide; they now warn `dead_code`
until B lands. `verify_store`'s descriptor↔fingerprint pass stays until
that file's owner deletes it; `ReadTxn::schema_descriptor` remains as its
only reader. Bench and docs still mention exhume (lanes D/E). `META_SCHEMA_DESCRIPTOR`
leaving the roster is 23.
