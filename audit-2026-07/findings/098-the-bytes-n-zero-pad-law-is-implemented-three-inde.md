## The bytes<N> zero-pad law is implemented three independent times

category: unification | severity: low | verdict: CONFIRMED | finder: engine:encoding

### Summary

The data-model spec states the `bytes<N>` law once — "the N raw bytes, zero-padded to the word boundary — the pad is encoding, not data" (`docs/architecture/10-data-model.md:14-15`, restated at line 477) — but the crate implements it three separate times. `FixedBytesValue` is already the canonical owner: a `Copy`, stack-only 64-byte buffer whose pad is zero by construction, with the exact `⌈len/8⌉ × 8` projection exposed as `.padded()`. The other two sites re-derive what it already holds, and the module's own comment narrates the split rather than erasing it. This directly contradicts the codebase's stated one-definition-site doctrine (the doc comments on `encode_literal` and `append_key_field` both invoke it by name).

### Evidence (all verified against the code)

1. **Canonical owner** — `crates/bumbledb/src/encoding.rs:79-90`: `FixedBytesValue::new` copies `raw` into a zeroed `[u8; MAX_FIXED_BYTES]` (lines 84-85), so the pad is zero by construction; `padded()` at lines 107-109 returns exactly the canonical `⌈len/8⌉ × 8` encoding.

2. **Re-implementation #1** — `crates/bumbledb/src/encoding/encode.rs:60-64`: `encode_fixed_bytes` re-derives the pad on a `Vec` via `extend_from_slice(raw)` + `resize(.., 0)`. It has exactly one production caller (`encode_literal`'s `Value::FixedBytes` arm, encode.rs:92) and is already demoted to a `#[cfg(test)]` re-export at `encoding.rs:28-29`. Meanwhile `append_key_field` already takes the canonical road: `out.extend_from_slice(value.padded())` (encode.rs:140-141).

3. **Re-implementation #2** — `crates/bumbledb/src/ir/normalize/lower_literal.rs:53-67`: `fixed_bytes_word_buf` re-derives the pad per 8-byte chunk (`let mut padded = [0u8; 8]; padded[..chunk.len()].copy_from_slice(chunk); *word = u64::from_be_bytes(padded)`), used by the bind warm path (`api/prepared/bind.rs:150, 327`) and `fixed_bytes_const` (lower_literal.rs:41-47).

4. **The split is narrated, not erased** — `crates/bumbledb/src/encoding.rs:23-27`: "The bytes<N> padder's production users live inside this module ... the bind path resolves through `ir::normalize::fixed_bytes_word_buf` instead (no Vec on the warm path)."

5. **Doctrine** — the same file's own comments demand single ownership: `encode_literal` (encode.rs:68-71) — "The one definition site for selection-literal encoding ... so the two can never drift apart"; `append_key_field` (encode.rs:117-121) — "One definition site ... the parity law".

6. **Test asymmetry** — `encode_fixed_bytes` is parity-tested against `padded()` via round-trip through `decode_fixed_bytes` (`encoding/tests.rs:341-357`), and `decode_fixed_bytes` (`encoding/decode.rs:98-110`) corruption-checks the pad law. But a crate-wide grep shows **no test anywhere exercises `fixed_bytes_word_buf` directly** — its agreement with the canonical padded encoding rests on inspection only.

### Failure scenario / impact

No wrong output today. The exposure is drift: three sites own one law, and the one on the warm bind path is the one with no pinning test. A future edit to any single site (a different pad byte, a boundary change, a chunking bug) silently desynchronizes the column-word view from the stored-fact bytes that `decode_fixed_bytes` corruption-checks, and the untested third copy is where it would land. Representationally, this is exactly the "special case a better representation would erase" pattern the project's design doctrine (docs/design/representation-first.md) targets.

### Suggested fix

Fold both re-implementations onto `FixedBytesValue::padded()`:

- `encode_literal`'s `Value::FixedBytes` arm becomes `out.extend_from_slice(FixedBytesValue::new(raw).padded())`, and `encode_fixed_bytes` dies (its test fixtures move to the same expression).
- `fixed_bytes_word_buf` becomes a word-chunking view over `padded()`: `for chunk in value.padded().chunks_exact(8) { words[count] = u64::from_be_bytes(chunk.try_into().unwrap()); count += 1; }` — every chunk is exactly 8 bytes by the padded-length invariant, so the per-chunk pad buffer vanishes.

The zero-alloc contract on the bind warm path is preserved — `FixedBytesValue` is a stack `Copy` type, so no Vec appears anywhere — and the pad law gets one owner, which the existing round-trip test and `decode_fixed_bytes` then pin for every consumer at once. The `encoding.rs:23-27` comment explaining the split can be deleted along with the split itself.
