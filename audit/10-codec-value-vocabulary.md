# 10 — Typed decode still round-trips the wide `ValueRef` sum; `FixedBytes` still rides the value

- **Status:** OPEN (verified 2026-08-19 17:10 EDT — four
  `unreachable!("schema-typed")` sites in `api/db.rs`; the tree is hot).
- **Severity:** should-fix.
- **Supersedes:** VER-02, PROP-014; carries PROP-015/REP-05 as its tail.

## Principle

Insight 6: `decode_field` *knows* each field's arm from the layout, produces
it, then discards that knowledge into the nine-arm `ValueRef` sum — so the
typed entry points re-narrow behind `unreachable!`. The proof should never
leave the layout. Same disease on the write side: fixedness lives in the
layout **and** on the value, so a mismatched pair is spellable and writes 16
bytes into an 8-byte slot.

## Evidence

- `crates/bumbledb/src/api/db.rs` — 4 × `unreachable!("schema-typed")`
  inside `CodecRead::{decode_bool_field, decode_fixed_bytes_field,
  decode_interval_*_field}` defaults. (The macro emission is gone — these
  three shared defaults are the residue; `decode_u64/i64/str_field` already
  bypass the sum via `field_word_bytes`.)
- `crates/bumbledb/src/encoding.rs` — `ValueRef::FixedBytes` still an arm;
  fixedness duplicated between layout and value (PROP-014).

## The fix

1. Typed entries in `encoding` that never construct the sum:
   `decode_bool(view, idx) -> Result<bool>`,
   `decode_fixed_bytes(view, idx) -> Result<&[u8]>`,
   `decode_interval_u64/i64(view, idx) -> Result<Interval<_>>` — each reads
   the layout arm directly, exactly as `field_word_bytes` does for words.
   The four `unreachable!`s delete; `CodecRead` defaults become one-liners.
2. Write side: field writers take the layout's `ValueType`; the
   `ValueRef::FixedBytes` arm deletes and the width has one home. The
   silent 16-into-8 corruption becomes unconstructible.
3. **Tail (later, same vocabulary):** `IntervalTail` is the fifth
   interval-shape spelling; after this lands it becomes the
   interval-restricted `ValueType` and `bytes()/words()` collapse into the
   one width owner (PROP-015/REP-05).

## Acceptance

- `grep 'unreachable!("schema-typed")' crates/` is empty.
- `ValueRef` has no `Fixed*` arms; `encode_fact` cannot be handed a
  value/layout width disagreement (type-level: the writer takes the layout
  arm).
- Byte-identical stores and goldens — this is representation only.
