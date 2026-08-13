# Fact identity is canonical bytes in Lean, blake3 of those bytes in the store
- id: 219
- severity: medium
- confidence: confirmed
- area: spec-docs-rust
- wrong-side: split
- components: lean/Bumbledb/Values.lean, docs/architecture/10-data-model.md, crates/bumbledb/src/encoding/fact_hash.rs, crates/bumbledb/src/storage/dict.rs
- status: fixed (2026-08-13)

## Summary
Lean `value_eq_iff_encode_eq` is equality of canonical encodings (abstract words). Architecture docs state that storage membership is blake3-256 of `fact_bytes` and that **hash equality is treated as fact equality — collisions are an accepted axiom**, with no byte verification on `M` or dictionary probes. Lean is silent (mechanism fence). A blake3 collision unifies two Lean-distinct facts in the engine.

## Lean spec
`Values.lean:567-572` theorem 7: within one value type, values are equal iff canonical encodings are equal. Cross-type injectivity is deliberately false. No hash. `lean/README.md:114-120`: hashing/LMDB are mechanism Lean does not own.

## Normative docs
`10-data-model.md:480-493`: "Value equality is `fact_bytes` equality (`value_eq_iff_encode_eq`)" then "Storage implements membership as blake3-256 of `fact_bytes`; **hash equality is treated as fact equality — collisions are an accepted axiom** (2⁻¹²⁸-scale event), not verified against." Same axiom on the dictionary content hash (`:491-501`).

## Rust implementation
`fact_hash.rs` blake3 of canonical fact bytes; `M` keys are the 32-byte digest; dictionary forward `blake3(bytes) → id` (`dict.rs`). Probes do not compare `fact_bytes` on hash hit.

## Why this matters
The Lean identity law is encoding equality. The engine's insert/delete/contains path is hash equality. Docs explicitly accept silent unification of distinct facts. That is a specified weakening of the spec identity theorem, not an accident — but `value_eq_iff_encode_eq` is still cited as if it were the store's membership law. A collision is a set-semantics bug the spec cannot see and the store will not detect.

## Verification (2026-08-12)
Re-read theorem 7, the identity section, and `fact_hash`. **Confirmed.** `wrong-side: split`: Lean encoding equality vs documented blake3 axiom. Docs are explicit, not accidental.

**Lean** (`lean/Bumbledb/Values.lean:567-572`): within one value type, values are equal iff canonical encodings are equal. No hash. `lean/README.md:114-120`: hashing/LMDB are mechanism Lean does not own.

**Docs** (`docs/architecture/10-data-model.md:480-493`): “Value equality is `fact_bytes` equality (`value_eq_iff_encode_eq`)” then “Storage implements membership as blake3-256 of `fact_bytes`; **hash equality is treated as fact equality — collisions are an accepted axiom**.” Same on the dictionary (`:491-501`).

**Rust** (`crates/bumbledb/src/encoding/fact_hash.rs:1-10`): blake3 of canonical fact bytes; no byte verification on hash hit. Dictionary forward `blake3(bytes) → id` (`storage/dict.rs:11`, `:30`).

## Related
- 209 (`bytes<N>` encoding granularity compounds what is hashed)

## Resolution (2026-08-13)
Documented the blake3 collision axiom: Lean identity is canonical encoding equality; store membership is hash equality (`Values.lean` theorem 7, `50-storage.md`, `lean/README.md`). Store unchanged.
