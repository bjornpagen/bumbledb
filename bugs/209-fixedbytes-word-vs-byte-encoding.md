# Lean FixedBytes is N words; Rust/docs encode N bytes padded to ⌈N/8⌉ words
- id: 209
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: split
- components: lean/Bumbledb/Values.lean, lean/Bumbledb/Conformance.lean, docs/architecture/10-data-model.md, crates/bumbledb/src/encoding.rs, crates/bumbledb/src/encoding/decode.rs
- status: fixed (2026-08-13)

## Summary
Lean models `bytes<N>` as a length-`N` list of abstract Words and `encodeAt` as that list. Docs and Rust store `N` raw bytes zero-padded to a multiple of 8. For `N ≢ 0 (mod 8)` or `N > 8`, `value_eq_iff_encode_eq` on `fixedBytes` is not byte-identical to on-disk `fact_bytes`. The pad-is-encoding claim in Lean hides a different granularity than the engine's pad-is-trailing-bytes invariant.

## Lean spec
```525:527:lean/Bumbledb/Values.lean
/-- A `bytes<N>` payload: exactly `n` words. The zero-pad to the word
boundary is encoding, not data (constant for a fixed `n`). -/
abbrev FixedBytes (n : Nat) : Type := { l : List Word // l.length = n }
```

`encodeAt .fixedBytes _ bs => bs.val` (`Values.lean:556`). Module doc (`:53-55`, `:78-82`): pad invisible at this level; `n` total over ℕ while Rust is `1..=64`. Conformance JSON maps one byte → one Word (`Conformance.lean` bytes decoder).

## Normative docs
`10-data-model.md:14-15`: `Bytes(N), N ∈ 1..=64` — "the N raw bytes, zero-padded to the word boundary — the pad is encoding, not data (a nonzero pad byte is corruption)." Identity (`:489-490`): `bytes<N>` is its N raw bytes zero-padded.

## Rust implementation
`encoding.rs:45-51`: `MAX_FIXED_BYTES = 64`; `fixed_bytes_words(len) = ⌈len/8⌉`. `FixedBytesValue` holds `N` raw bytes in a 64-byte buffer with trailing zeros (`:72-78`). Decode rejects nonzero pad (`NonzeroFixedBytesPad`, `decode.rs`).

## Why this matters
Anyone treating Lean `encodeAt` as the on-disk layout will mis-size `bytes<9>` (Lean: 9 words; Rust: 16 stored bytes, 9 value bytes). Fact-hash and determinant keys are over the padded byte encoding. Query-level value equality via conformance JSON still aligns; storage-level identity does not refine.

## Verification (2026-08-12)
Re-read `FixedBytes` / `encodeAt`, the data-model encoding table, and `fixed_bytes_words`. **Confirmed.** `wrong-side: split`: Lean models N abstract words with pad invisible; docs and Rust store N raw bytes padded to `⌈N/8⌉×8`. Lean (`Values.lean:48-55`) disclaims byte layouts, so this is a refinement gap, not a silent contradiction in the math — still a real encode-length mismatch for anyone reading `encodeAt` as `fact_bytes`.

**Lean** (`lean/Bumbledb/Values.lean:525-527`, `:551-556`): `FixedBytes n` is `{ l : List Word // l.length = n }`; `encodeAt .fixedBytes _ bs => bs.val`. Conformance JSON maps one byte → one Word (`Conformance.lean:247-249`). `n` is total over ℕ; Rust is `1..=64` (`Values.lean:78-82`).

**Docs** (`docs/architecture/10-data-model.md:14-16`, `:489-490`): `Bytes(N), N ∈ 1..=64` is “the N raw bytes, zero-padded to the word boundary”.

**Rust** (`crates/bumbledb/src/encoding.rs:45-50`, `:97-101`): `MAX_FIXED_BYTES = 64`; `fixed_bytes_words(len) = ⌈len/8⌉`; `FixedBytesValue::padded` returns that many bytes.

## Related
- 219 (hash identity vs canonical-bytes identity)

## Resolution (2026-08-13)
Lean `FixedBytes n` is now `n` raw `Byte`s; `padFixedBytes` zero-pads to `⌈n/8⌉×8` abstract words. Wire format unchanged. `value_eq_iff_encode_eq` uses right-append cancellation.
