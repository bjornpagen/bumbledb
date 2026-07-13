# PRD 10 — The entropy seam: one Rng, two sources

**Depends on:** 01 (toolchain). Phase D's foundation — 11–14 all build
on this seam.
**Modules:** `crates/bumbledb-bench/src/gen/` (the generator's xorshift
core and every call site that draws randomness: schema descriptors,
data, querygen, opgen), the corpus-digest pin test.
**Authority:** Ned-Williamson-style generative fuzzing (decided
2026-07): the fuzzer must drive the EXISTING generators — the ones the
two-oracle differential already trusts — from libFuzzer's byte stream,
not grow a parallel generation stack. The seam between "seeded
reproducible run" and "fuzzer-driven run" is the entropy source and
nothing else.
**Representation move:** the generator currently reaches into a concrete
xorshift struct. Reify the entropy source as a closed sum so the same
generation code is byte-identically reproducible under seeds AND
steerable by a coverage-guided fuzzer — two constructors, one
generation path, zero behavioral drift for the seeded arm.

## Context (decided shape)

```rust
/// Where generation entropy comes from. Seeded is the bench/differential
/// arm and must remain byte-identical to the pre-seam xorshift stream —
/// the corpus digest pin arbitrates. Bytes is the fuzzer arm: draws
/// consume the fuzzer's data; exhaustion falls back to a fixed
/// deterministic tail (zeros), never a panic — libFuzzer shrinks better
/// when short inputs are legal.
pub enum Rng {
    Seeded(XorShift),          // today's generator, unchanged
    Bytes(ByteSource),         // cursor over fuzzer-provided &[u8]
}
```

- Every generator function takes `&mut Rng` (most already take the
  concrete type — mechanical). The draw primitives (`next_u64`,
  bounded draw, bool, choice) match once on the variant; generation
  logic above them never knows the source.
- `ByteSource` semantics: consume from the slice; on exhaustion return
  zeros deterministically. Bounded draws use the same reduction the
  seeded arm uses (identical modulo/widening discipline) so a corpus
  byte string maps stably onto generation decisions — stability is what
  makes libFuzzer's mutations meaningful.
- `Scale::Tiny` added to the generator's scale ladder: relation counts,
  row counts, string lengths, and op-sequence lengths small enough that
  a single fuzz iteration (build store → run ops → run oracles) is
  milliseconds. Tiny is a first-class scale with the same invariants,
  not a special-cased path.
- **The pin:** the corpus digest test (the generator's
  determinism anchor) must pass UNCHANGED — same digest, zero test
  edits. This is the PRD's policy-8 pin and its most important line:
  the seam provably did not move the seeded arm.

## Technical direction

1. Land `Rng` with `Seeded` delegating verbatim to the existing
   xorshift (same struct, moved inside); chase call sites mechanically.
2. Run the digest pin + the full bench-lib suite before writing
   `Bytes` — the seeded arm's identity is proven first, in its own
   commit.
3. Land `ByteSource` + `Scale::Tiny`; a unit test drives a full
   schema+data+ops generation from a fixed byte string twice and
   asserts identical output (determinism of the fuzzer arm itself).
4. No fuzz targets in this PRD — the seam only. (Targets are 11–14;
   keeping them out keeps this PRD's blast radius auditable.)

## Passing criteria

- `[test]` The corpus digest pin green with ZERO edits to the test.
- `[test]` The `Bytes`-arm determinism test (same bytes → identical
  generated artifacts) and an exhaustion test (short input → completes
  with the zero tail, no panic).
- `[shape]` `grep -rn "XorShift" crates/bumbledb-bench/src` hits only
  the `Rng` module — no generator logic touches the concrete source.
- `[shape]` `Scale::Tiny` exists and is exercised by at least the
  determinism test; its size constants recorded in the scale ladder's
  table.
- `[gate]` bench-lib suite green.

## Doc amendments (rule 5)

The measurement doc's generator section: one paragraph on the entropy
seam (two sources, one generator, digest-pinned) and the Tiny scale's
purpose.
