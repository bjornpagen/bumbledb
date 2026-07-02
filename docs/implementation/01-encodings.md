# PRD 01 — Canonical Encodings and the Fact Codec

Authority: `docs/architecture/10-data-model.md` (type layer; canonical fact encoding =
identity; dense, no padding), `00-product.md` (machine model).

## Purpose

The byte-level truth of the whole system: per-type canonical encodings and the
fact_bytes codec. Everything above stores, hashes, and compares these bytes.

## Technical direction

- `encoding` module. Value widths: Bool 1 (strictly 0x00/0x01), Enum 1 (ordinal),
  U64 8 (big-endian), I64 8 (sign-bit-flipped big-endian), String/Bytes 8 (intern id,
  big-endian u64). Encode/decode functions per type; decode of an invalid Bool byte or
  out-of-range enum ordinal returns `CorruptionError` (a local error type re-homed in
  PRD 04) — **never a skip, never a default**.
- Order preservation is a tested contract: for U64 and I64, `a < b ⇔ encode(a) <
  encode(b)` lexicographically.
- `FactLayout`: computed from an ordered slice of type descriptions — per-field byte
  offset and width, total fact width. Dense: no padding anywhere (unaligned loads are
  near-free on the target; document this in a comment citing the doc).
- `encode_fact(&[ValueRef], &FactLayout, &mut Vec<u8>)` writing into a caller buffer;
  `field_bytes(fact_bytes, layout, field_idx) -> &[u8]` as O(1) slicing;
  `decode_field(...) -> ValueRef`. `ValueRef<'a>` is a borrowed six-variant enum
  (String/Bytes carry intern ids at this layer; raw-bytes decode is the dictionary's
  job, PRD 05).
- Fact identity: `fact_hash(fact_bytes) -> [u8; 32]` = blake3. Full 32 bytes, never
  truncated (post-mortem: v5 truncated to 16).

## Non-goals

Interning (PRD 05). Schema types (PRD 02) — this module works on plain type
descriptions passed in.

## Passing criteria

- Unit tests: round-trip per type incl. extremes (0, MAX, MIN, i64::MIN, sign
  boundaries); order-preservation property over sorted samples for U64/I64; Bool
  strictness (0x02 → corruption error); enum ordinal range check; layout offsets for a
  mixed 1/8-byte relation are exactly cumulative widths with no padding; `field_bytes`
  slices equal independently-encoded fields.
- No heap allocation in `encode_fact`/`field_bytes`/`decode_field` beyond the caller's
  buffer (assert via the fact the signatures take buffers; allocator counting arrives
  in PRD 26).
- Global commands green.
