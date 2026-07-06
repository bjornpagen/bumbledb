# PRD 06 — Corruption is a typed error, everywhere

Findings fixed (docs/audit/): **storage LOW** "Corrupt short keys panic
instead of returning typed Corruption errors" (commit.rs:162, read.rs:126-129,
plus the delta.rs:107/dict.rs:84 counter asserts); **image LOW** "Image build
trusts the stored row count for slab sizing before verifying it";
**api-schema LOW** "Out-of-range RelationId panics on the dynamic/ETL
surface"; **sink-pipeline LOW** "Result byte-heap offsets panic past 4 GiB
instead of erroring".

## Purpose

40-storage's rule is "corrupt data is a hard error, never a skip" — and the
error taxonomy implements *hard error* as typed `Corruption`, not process
panic. Four seams still panic on inputs that are data (a corrupted store, an
ETL-supplied relation id, a pathological-but-valid result size), not
programmer error. Convert every one to the typed form the docs promise; leave
genuinely-programmer-error panics (documented `# Panics` contracts) alone.

## Technical direction

- **Key-shape checks before slicing.** The R-namespace restrict scan
  (`commit.rs:162`) and the F-namespace row-id slice (`read.rs:126-129`)
  length-check the scanned key against its exact expected shape
  (`prefix_len + 12` for R; `13` for F — derive both from the keys.rs codec
  constants, never re-hardcode) and return
  `Corruption(CorruptionError::MalformedValue("R key length" / "F key
  length"))` on mismatch. The audit's concrete shape — an 8-byte bare-prefix
  R key under a 1-byte guard — must become a typed error end to end (the
  commit aborts cleanly; nothing torn).
- **Counter reads become fallible.** `delta.rs:107` and `dict.rs:84` assert
  on a `u64::MAX` dict counter; a corrupted counter is data. Return the same
  `MalformedValue`-class Corruption. (The *exhaustion* assert on mint —
  id space genuinely running out — is a different statement; keep it a
  documented panic, it is not corruption.)
- **Image build validates the row count first.** `image.rs:192-204`:
  before sizing slabs, bound `row_count` by a sanity ceiling derived from
  what the store could possibly hold (e.g. `data.mdb` size / minimum fact
  width — or simpler and sufficient: checked arithmetic on every slab-size
  computation, mapping overflow to
  `Corruption(MalformedValue("S row count"))`). The existing both-direction
  scan cross-check stays as the exactness guarantee; this PRD only converts
  the pre-check failure mode from panic/OOM-abort to the typed error.
- **`RelationId` bounds at the dynamic surface.** `schema.rs:213-215` panics
  on out-of-range ids reached from public `insert_dyn`/`delete_dyn`
  (via `encode_dyn`), `bulk_load`, and `Snapshot::scan`. Per 60-api ("ETL
  input is data, not code"), add an `UnknownRelation` arm to the FactShape
  error family and bounds-check at those four public boundaries.
  `Schema::relation` itself keeps its indexing contract for internal callers
  (every internal id is plan-derived and dense) — the check lives at the
  boundary, not in the hot path.
- **ResolveMemo offsets error, not panic.** `api/prepared.rs:221-224`: the
  two `u32::try_from(..).expect(..)` sites become a typed error
  (`Error::Overflow`-class, message naming the result-buffer byte heap).
  A >4 GiB distinct-payload result is absurd under the scale axiom but it is
  *valid input*, and finalize already threads `Result`.
- While in error.rs: rename the FK-list duplicate misfire
  (api-schema NOTE — `SchemaError::UniqueDuplicateField` returned for a
  duplicated FK field) to a correctly-named variant
  (`ConstraintDuplicateField` covering both, or a sibling FK variant) —
  cutover freely, update the pinned test.

## Non-goals

An online M↔F↔U↔R consistency sweeper (stays deferred by decision — the
R-delete asymmetry NOTE remains a documented note, revisited in PRD 10's
docs); guarding physically-unreachable widths beyond the envelope (the
kernel u32 position casts stay documented panics per the executor audit).

## Passing criteria

- Corrupt-store tests, one per seam, each planting the audit's exact shape
  via raw LMDB writes in a test (heed is available to tests): the bare-prefix
  R key under a Bool-guard FK → `Corruption` from the delete's commit, store
  reopenable afterward; a 5-byte F key → `Corruption` from `scan`; a
  `u64::MAX` dict counter → `Corruption` from the next intern-bearing write;
  a huge `S` count → `Corruption` from `image::build` (via a prepared
  execution), with no allocation attempt beyond the ceiling.
- `bulk_load(RelationId(999), ..)`, `insert_dyn`, `delete_dyn`, and
  `snap.scan` with an out-of-range id each return the typed error — no panic
  (the audit's exact call shapes).
- The renamed schema-error variant is pinned by the updated
  `rejects_duplicate_fields_in_an_fk_list` test.
- Grep-level criterion: no `expect`/`assert!`/direct slice on
  externally-sourced lengths remains in the four cited regions (list the
  sites in the commit message); documented `# Panics` contracts elsewhere are
  untouched.
- `scripts/check.sh` green.
