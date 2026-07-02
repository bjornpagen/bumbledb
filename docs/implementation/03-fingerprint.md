# PRD 03 — Canonical Serialization and Schema Fingerprint

Authority: `docs/architecture/10-data-model.md` (fingerprint inputs, exhaustively
enumerated there — that list is the contract).

## Purpose

Deterministic schema identity: canonical bytes → blake3 fingerprint, stored at database
creation and compared on open.

## Technical direction

- `schema::fingerprint`. `canonical_bytes(&Schema, &mut Vec<u8>)` serializing exactly
  the doc's input list: format-version label first, then relations in declaration order
  (name; fields in order: name, structural type description including the full ordered
  enum variant list, generation flag; constraints in order: name, ordered field ids,
  FK target relation + constraint names). Length-prefix every string and every list
  (u32 LE) so no two schemas can alias to one byte stream; one-byte tags per ValueType
  variant and per Generation.
- `SchemaFingerprint([u8; 32])` = blake3 of canonical bytes. Plain newtype, no Display
  ceremony (post-mortem §11: v5's formatters had zero callers).
- Auto-materialized serial uniques ARE serialized (they're ordinary constraints in the
  descriptor by PRD 02 — no special case here, which is the point).

## Non-goals

Storage of the fingerprint (PRD 04). Any compatibility labeling of old versions.

## Passing criteria

- Unit tests: identical declarations → identical fingerprints; each of these changes
  the fingerprint — reordering two fields, renaming a field, adding an enum variant,
  reordering enum variants, changing a constraint's field order, changing an FK
  target, toggling Serial generation; a golden-bytes test pins the canonical
  serialization of one small fixture schema (the anti-drift anchor the old repo never
  had — post-mortem §17); an aliasing test: two schemas whose concatenated names could
  collide without length prefixes produce different bytes.
- Global commands green.
