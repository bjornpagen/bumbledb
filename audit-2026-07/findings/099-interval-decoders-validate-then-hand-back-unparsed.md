## Interval decoders validate then hand back unparsed tuples, forcing re-parse-with-expect

category: inelegance | severity: low | verdict: CONFIRMED | finder: engine:encoding
outcome: fixed 7406bd74

### Summary

`decode_interval_u64` and `decode_interval_i64` establish the interval invariant `start < end` and then throw the proof away by returning a raw `(start, end)` tuple. Their only non-test consumer, `decode_field`, immediately re-runs the checked constructor and `.expect()`s away the invariant just proven — four such proof-discarding sites. The fixed-i64 arm additionally re-inlines the i64 sign-flip law as a local closure, creating a third definition site for a two-line law already defined once on each side of the codec. This is the codebase's own doctrine violated on the decode side: construction is supposed to be the validation boundary ("parse, don't validate"), but here the boundary validates without parsing.

### Evidence (all verified against the working tree)

- `crates/bumbledb/src/encoding/decode.rs:41-49` — `decode_interval_u64` returns `Result<(u64, u64), CorruptionError>` after checking `start < end`; `decode.rs:57-65` — `decode_interval_i64` likewise.
- `crates/bumbledb/src/encoding/decode.rs:187-198` — `decode_field` re-parses: `Interval::<u64>::new(start, end).expect("decode_interval_u64 accepted these bounds")` and the i64 twin. Two more expect sites on the fixed-width arm at `decode.rs:212-213` and `decode.rs:218-219` (`.expect("the Q2 bound implies start < end")`).
- Grep over the crate: `decode_field` is the **only** non-test caller of `decode_interval_u64`/`decode_interval_i64` (they are not even re-exported in `encoding.rs:16`'s pub list), so the tuple-returning signature serves no consumer that needs raw bounds.
- `crates/bumbledb/src/encoding/decode.rs:216` — `let decode = |word: u64| (word ^ I64_SIGN_BIT).cast_signed();` is the third spelling of the sign-flip law; grep for `I64_SIGN_BIT` finds exactly three use sites: `encode.rs:22`, `decode.rs:30` (`decode_i64`), and this closure. `schema/descriptor_codec.rs:389-391` already shows the correct pattern (`decode_i64(start_word.to_be_bytes())`) for the identical situation.
- Doctrine: `crates/bumbledb-theory/src/interval.rs:3-7` — "Construction is the validation boundary (parse, don't validate): ... a held `Interval` always satisfies `start < end` and the encoder never re-checks it." And `docs/design/representation-first.md:112-114` — "Validation discards proof; parsing keeps it (King, 'Parse, Don't [Validate]') ... a parser returns a refined type that carries the proof." The decode side does the opposite of both.

### Corrections to the original finding

- The claim that the `decode.rs:216` closure "can desynchronize without any test noticing" is **wrong**: `encoding/tests.rs::fixed_interval_round_trips_one_word` (second loop) encodes fixed i64 intervals with starts `i64::MIN` and `-1` and asserts `decode_field` returns the exact host-constructed `Interval`, so any decode-side drift in the sign flip fails that assert. The redundancy is a definition-site smell, not a latent-bug vector.
- `decode_fixed_interval_start` returning a word-domain `(start_word, end_word)` tuple is **justified**, not the same flaw: it deliberately operates in the order-preserving word domain shared by both element types, and has genuine word-domain consumers — `schema.rs:225` (`IntervalTail::words`, key-tail comparison) and `exec/dispatch/fact_word.rs:70` (`FactOperand::Pair`). Its signature should stay; only `decode_field`'s consumption of it carries the expect residue.

### Failure scenario / impact

No wrong output — all four expects are provably unreachable. The cost is representational: the decode boundary validates and then forces every consumer to re-derive what it just proved, and the sign-flip law carries a third definition site that exists only because the closure avoids one `to_be_bytes()` round-trip already accepted elsewhere (`descriptor_codec.rs:389`).

### Suggested fix

1. Change `decode_interval_u64`/`decode_interval_i64` to return `Result<Interval<u64>, CorruptionError>` / `Result<Interval<i64>, CorruptionError>`, constructing the checked type at the one point the check runs. `Interval::new` is `(start < end).then_some(...)` (interval.rs:34-36, 89-91); if the decoders' `const`ness is worth keeping, rewrite `new` as a const `if` — nothing currently calls the decoders in const context, so dropping `const` is also fine.
2. Replace the `decode.rs:216` closure with `decode_i64(word.to_be_bytes())` (the exact pattern at `descriptor_codec.rs:389-391`), restoring the sign-flip law to exactly two definition sites: one encode, one decode.
3. The two fixed-arm expects (`decode.rs:212-219`) can only vanish if `decode_fixed_interval_start` grows checked-typed siblings; given its legitimate word-domain consumers, either leave them (with the Q2-bound comment they carry) or add a thin per-element wrapper used only by `decode_field`.
