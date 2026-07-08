# PRD 07 — Commit: functionality enforcement

**Depends on:** 03, 06.
**Modules:** `crates/bumbledb/src/storage/commit.rs` and submodules, `crates/bumbledb/src/error.rs`.
**Authority:** `docs/architecture/50-storage.md` (§ commit steps 1–2), `30-dependencies.md` (§ pointwise lifting).

## Goal

Commit phases 1–2 maintain the new `U` namespace and enforce every `Functionality`
statement — scalar keys by put-conflict (as today), pointwise keys by the
ordered-neighbor probe.

## Technical direction

1. **Delete phase (step 1):** per deleted fact, derive and delete its `U` entries
   for every key statement of its relation (guard slicing per PRD 06). Record each
   deleted guard as `(StatementId, guard_bytes)` into the `deleted_guards` set —
   the shape already exists; re-key it by `StatementId` (schema-global) instead of
   `(RelationId, ConstraintId)`.
2. **Insert phase (step 2), scalar keys** (`Resolved::Functionality { interval_position: None }`):
   unchanged mechanics — a `U` put that finds an existing entry is a violation.
   Error: `Error::FunctionalityViolation { statement: StatementId, fact: Box<[u8]> }`
   (rename/replace `UniqueViolation`; carry the offending fact's canonical bytes,
   never a row id — `10-data-model.md`).
3. **Insert phase, pointwise keys** (`interval_position: Some(last)`): the exact
   put cannot detect overlap, so after the put, run the neighbor probe:
   - Let `prefix` = `U | rel | stmt | scalar_prefix_bytes` (all guard bytes before
     the 16-byte interval) and `(s, e)` = the inserted interval's encoded halves.
   - **Predecessor:** cursor-seek to the inserted key, step back one; if the key
     shares `prefix`, parse its trailing 16 bytes as `(ps, pe)`; `pe > s` (byte
     compare on the 8-byte encoded halves — order-preserving encodings make byte
     compare correct) ⇒ violation.
   - **Successor:** step forward from the inserted key; if it shares `prefix`,
     parse `(ns, _)`; `ns < e` ⇒ violation.
   - Two probes, O(log n), same write txn (LMDB write txns read their own writes,
     so intra-delta overlaps are caught identically). Violation error carries the
     statement id and *both* facts' bytes (the incumbent is fetched via its `U`
     value's row_id → `F` get; this is the cold aborting path — one extra get is
     sanctioned, same rationale as the existing referrer fetch).
4. Adjacency is legal by construction: `pe == s` and `ns == e` pass (strict
   comparisons above). Write the comparison directions once, with a comment
   deriving them from half-open semantics; do not "defensively" widen them.
5. Membership-desync hard errors (missing `F`/`M`/`U` on delete) carry over
   unchanged; extend the guard re-derivation to 16-byte fields.

## Out of scope

`R` maintenance and containment checks (PRDs 08–09). The old `restrict.rs` is
untouched here (deleted in PRD 09).

## Passing criteria

- `[shape]` `UniqueViolation` no longer exists; `FunctionalityViolation` carries
  `StatementId` + fact bytes (+ incumbent bytes for the pointwise arm).
- `[shape]` The neighbor probe is one function with the half-open comparison
  derivation comment; no epsilon adjustments, no inclusive comparisons.
- `[test]` Scalar key conflict inside one delta and across deltas both abort with
  the right statement id (port the existing unique-conflict tests).
- `[test]` Pointwise matrix, one test per cell: overlap-left, overlap-right,
  containment, exact-duplicate-interval, adjacent-left (passes), adjacent-right
  (passes), disjoint (passes), same interval different scalar prefix (passes) —
  each in-delta and cross-delta; plus delete-then-reinsert-overlapping-in-one-delta
  (judged against final state: the delete frees the window, passes).
- `[test]` `MAX_END`-sentinel intervals participate correctly: `[5, MAX)` then
  `[9, MAX)` in the same group aborts; `[5, 9)` then `[9, MAX)` passes.
