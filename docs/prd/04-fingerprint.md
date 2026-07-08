# PRD 04 — Fingerprint over statements

**Depends on:** 02, 03.
**Modules:** `crates/bumbledb/src/schema/fingerprint.rs`.
**Authority:** `docs/architecture/10-data-model.md` (§ Schema, fingerprint inputs — exhaustive).

## Goal

The schema fingerprint's canonical serialization covers the new world exactly as
`10-data-model.md` enumerates it, and nothing else.

## Technical direction

1. Bump the encoding-format version label that seeds the hash (a new literal — the
   redesign is a different format even for schemas that would serialize identically,
   which none do).
2. Serialization order, exhaustively (delete the constraint serialization):
   - format label;
   - relations in declaration order: name; fields in declaration order — name,
     structural type description (enum = full ordered variant list; interval = the
     element tag), generation flag;
   - statements in **materialized** order (PRD 02 rule): a form tag
     (Functionality = 0, Containment = 1); for Functionality — relation id,
     projection field ids in statement order; for Containment — each side as
     (relation id, projection field ids in order, selection as (field id, literal
     value canonical encoding) pairs in statement order). Literal values serialize
     through the canonical value encoding from PRD 01/`encoding` (never a Rust
     `Debug` or ad-hoc format).
3. `Resolved` data (target keys, permutations) is **not** hashed — it is a
   deterministic function of the hashed inputs; add a doc comment saying exactly
   that, citing the "constraint ids pinned without being hashed separately"
   precedent in `10-data-model.md`.
4. Update the fingerprint unit tests: the existing style (mutate one input, assert
   the hash changes) extends to statements — reordering two declared statements,
   changing a selection literal, swapping `<=` sides, and changing an interval
   element type must each change the fingerprint; recomputing an identical
   descriptor must not.

## Out of scope

Open-time verification wiring (unchanged, `storage/env.rs`), format version of the
*store* (PRD 06 — a different constant).

## Passing criteria

- `[shape]` No constraint-related serialization code remains in `fingerprint.rs`.
- `[shape]` The serialization function is a single linear pass matching the order
  above, with a doc comment reproducing the input list from `10-data-model.md`.
- `[test]` Sensitivity tests: each of {statement reorder, side swap, selection
  literal change, projection order change, interval element change, enum variant
  addition} changes the hash.
- `[test]` Stability test: two independently constructed identical descriptors
  produce byte-identical fingerprints.
