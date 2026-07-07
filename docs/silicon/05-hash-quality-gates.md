# PRD 05 — Hash quality is false-tag rate: pin it forever

## Purpose

bumblebench exp 02's sharpest negative result: a single-multiply fold hash
is 2× cheaper than our mulxor and passes every probe-length vetting
(mean 1.40, p99 = 5) while collapsing the 7-bit ctrl tag to 19.4% false
compares on strided/sequential keys — 25× the 1/128 design point. Probe
length is blind to tag quality because tags and slots use different hash
bits. bumbledb's key columns are exactly the adversarial case (serials,
enum codes, sequential ids). Nothing in our test suite would catch a
future "optimization" that swaps in a cheaper hash and silently destroys
probe performance on low-entropy keys. This PRD makes that class of
regression impossible.

## Technical direction

`crates/bumbledb/src/exec/wordmap.rs`, `crates/bumbledb/src/exec/colt.rs`.

- **Instrument false compares, test-only.** Behind `#[cfg(test)]` (or the
  existing trace feature if a runtime view is wanted later — but the
  release path must be untouched): count `(tag matched, key mismatched)`
  events per probe sequence in both wordmap and colt. Zero release cost is
  a gate, not a hope — the counter fields and increments must not exist in
  the release binary.
- **Property tests, adversarial key families.** For each of: sequential
  u64s (0..n), strided (0, 8, 16, …), strided ×4096 (page-like), biased
  i64 encodings of small magnitudes, enum-coded low cardinality repeated,
  and splitmix-random (control): fill to the PRD-03 load factor, probe a
  mixed hit/miss stream, assert false-compare rate ≤ 2/128 per probe
  (design 1/128 with headroom; mulxor measures 0.007–0.009).
- **Pin the hash by property, not by constant.** The test must not
  hard-code "mulxor": it gates ANY hash the module ships. Add a doc
  comment on the hash function citing exp 02: cheaper single-multiply
  hashes fail these tests at 19.4% — the two multiplies are load-bearing
  for the tag, independent of probe length.
- **Probe-length stays pinned too.** Keep/extend the existing
  `probe_steps` distribution test — the pair (probe length, false-tag
  rate) is the complete quality contract.

## Passing requirements

1. All adversarial-family property tests green at the shipped hash;
   demonstrably red under a deliberately weakened hash (include the
   red-case as a `#[should_panic]` or a commented reproduction in the
   test, so the gate's teeth are visible in review).
2. Release-cost gate: family ledger within ±2% of post-04 across the
   board (nothing was added to release paths); objdump spot-check shows
   no counter fields in the release probe loop.
3. Verify green; clippy green.

## Out of scope

Changing the hash; per-column hash selection (rejected — one vetted hash,
one contract); any release-mode instrumentation.
