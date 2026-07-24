## Rng::u64() seeded arm emits only 31 bits — 'random 32-byte payload' corpora are half zeros, arms diverge, range(n>2^31) is a loaded trap

category: bug | severity: medium | verdict: CONFIRMED | finder: bench:honesty

### Summary

The bench entropy seam (`crates/bumbledb-bench/src/corpus_gen/rng.rs`) promises one raw draw with two interchangeable sources. In reality the seeded arm's `u64()` returns `state >> 33` from an MMIX LCG — a value in `[0, 2^31)` — while the fuzzer `ByteSource` arm returns full 64-bit little-endian words. Three consequences, all verified in code:

1. Every "random 32-byte payload digest" built from four raw draws has bytes 4-7 of each 8-byte chunk permanently zero — 16 of 32 bytes constant — so the crud and points corpora exercise FixedBytes compare/store paths on half-zero blobs while documenting them as random digests.
2. Raw-u64 consumers see disjoint value spaces per arm, against the seam's own contract; a fuzzer byte string can steer payloads/nested seeds into states no seed can ever produce.
3. `range(n)` is `u64() % n`: for any future `n > 2^31` the seeded arm silently samples only the lower half of the range. All current bounds max out at `1 << 30`, so this is a loaded trap, not an active miscount.

The type is also misnamed: `XorShift` is an LCG, as its own doc comment says.

### Evidence (all verified against the working tree)

- `crates/bumbledb-bench/src/corpus_gen/rng.rs:37-43` — seeded arm: `self.state = self.state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407); self.state >> 33`. 64 − 33 = 31 output bits.
- `crates/bumbledb-bench/src/corpus_gen/rng.rs:67-74` — fuzzer arm: `u64::from_le_bytes(word)`, full 64 bits.
- `crates/bumbledb-bench/src/corpus_gen/rng.rs:20-24` — doc: "The house LCG"; type name: `XorShift`.
- `crates/bumbledb-bench/src/corpus_gen/rng.rs:90-92` — seam contract: "Every bounded draw reduces this word identically across arms, so the two sources map onto generation decisions the same way." True mechanically for `range()`, but raw-word consumers (below) get structurally different streams per arm.
- `crates/bumbledb-bench/src/corpus_gen/rng.rs:101-104` — `range(n)` is `self.u64() % n`; no width guard.
- Payload sites drawing raw words: `crates/bumbledb-bench/src/crud/corpus.rs:22-25` (`doc_row`), `crates/bumbledb-bench/src/crud/ops.rs:251-258` (`fresh_payload`), `crates/bumbledb-bench/src/scenarios/points.rs:88-91` (`doc_row_sized`, with the comment at `points.rs:97`: "Identity-shaped: a random 32-byte payload digest, inline"). Since each seeded draw < 2^31, `to_le_bytes()` yields `[b0, b1, b2, b3<0x80, 0, 0, 0, 0]` per chunk.
- Other raw-word consumers that diverge across arms: nested generator seeds at `corpus_gen/opgen.rs:86` and `:122`, miss-key strings at `querygen/oracle.rs:219` and `querygen/dress.rs:128`.
- `range()` bound survey (grep over the crate): current maximum is `1 << 30` at `crates/bumbledb-bench/src/writebench.rs:46`; everything else is in the millions or smaller. The `n > 2^31` hazard is latent.
- House-pattern context: the same `wrapping_mul(6_364_136_223_846_793_005) … >> 33` LCG appears in ~15 engine test files (e.g. `crates/bumbledb/src/exec/wordmap/tests/behavior.rs:123-125`, `crates/bumbledb/src/interval/sweep.rs:125-127`), so the truncation is a deliberate inherited convention — taking the high bits is the statistically correct way to use an MMIX LCG — but the bench seam's docs and payload claims don't survive it.
- The arbitration mechanism the fix relies on exists: the corpus digest pin test at `crates/bumbledb-bench/src/corpus_gen/tests.rs:12` (`the_corpus_digest_is_deterministic_and_pinned`), and an avalanche-style mixer already lives at `crates/bumbledb-bench/src/scenarios/mix.rs:4-11`.

### Bench impact / Failure scenario

- Today (honesty): the crud and points corpora's FixedBytes payloads are half-zero, contradicting "random 32-byte payload digest." Differential fairness is intact — both engines load identical rows — so no cross-engine skew; the dishonesty is between the documented corpus shape and the actual bytes the compare/store paths see.
- Cross-arm structure: a fuzzer byte string can produce payloads, nested seeds, and miss-keys with high bits set that no seed can ever reach, so the fuzz corpus and the bench corpus occupy structurally different regions of value space. (Fuzz cases still reproduce from their byte strings, so this is a corpus-divergence issue, not a reproducibility break.)
- Latent trap: any future `range(n)` with `n > 2^31` — a scale bump, a full-width key domain, a miss-draw over the whole id space — silently samples only `[0, 2^31)` on the seeded arm while the fuzzer arm samples all of `[0, n)`, with no assert to catch it.

### Suggested fix

Make the seeded arm emit a genuine 64-bit word: keep the LCG step but finish with an avalanche (the splitmix64-style finisher already used in `scenarios/mix.rs`), or switch to xorshift*/splitmix64 outright — do not return raw full-width LCG state (its low bits are weak; the current `>> 33` exists precisely to discard them). Rename `XorShift` to match the algorithm. The corpus digest pin (`corpus_gen/tests.rs:12`) is the designed arbitration for the deliberate stream change: stamps invalidate, corpora regenerate. Once `u64()` is full-width, `range()` is fine as-is; optionally `debug_assert` the bound against the source width in the interim if the fix is deferred.
