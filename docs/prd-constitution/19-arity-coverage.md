# PRD 19 — Arity coverage: the generators sweep the whole accepted product space

**Depends on:** 03 (FieldSet/Projection carriers), 12 (the equality
locks exist — this PRD generalizes them by generation).
**Modules:** `crates/bumbledb-bench/src/corpus_gen/theorygen.rs` (the
random-descriptor arm), the valid-schema generator beside it, the
differential/fuzz theory + ops lanes that consume them; NO engine
source changes expected (any engine limit the sweep exposes is a
policy-5 finding).
**Authority:** brief B1, approved: the theory says containment and
keyed equality work uniformly at every projection arity and scalar-type
mix up to the encoded-width bound (`MAX_KEY_IMAGE_WIDTH`); the
generators currently exercise a narrow habitual band, so uniformity is
a claim with a coverage hole. This is completion of an existing general
mechanism's VERIFICATION, not new semantics.
**Representation move:** none in the engine. The generator's schema
vocabulary grows until the accepted fragment's product space is
actually sampled.

## Context (decided shape)

1. **Generator arms** (behind the existing `Rng` seam, both the valid
   arm and theorygen's hostile arm):
   - composite keys and containment projections at arities 1 through
     the width bound (compute the max legal arity from the field-type
     mix — wide FixedBytes reach the bound fastest; generate at, one
     under, and one OVER the bound — the over case must reject with
     the width diagnostic);
   - mixed scalar types in one projection (u64/i64/bool/str/bytes<N>);
   - reordered target-key declaration order vs statement order
     (exercising `key_permutation` at arity ≥3);
   - selections on source, target, and both sides at high arity;
   - `==` pairs at arity 1, 2, 3, and max-legal, valid and
     deliberately key-less on either side (the reverse-rejection path
     under generation, not just the PRD-12 hand locks).
2. **Determinism discipline:** all new draws ride `Rng`; the corpus
   digest pin must NOT move (new arms extend the descriptor space but
   the seeded stream's existing draws are untouched — new arms draw
   AFTER existing decision points; if the pin moves, the arm was wired
   into the existing stream: stop and rewire).
3. **Oracles:** the theory target's three oracles judge the schemas;
   the ops lane's verdict-parity oracle judges enforcement at the new
   arities (the naive model already handles arbitrary arity — verify,
   don't assume; a model limit is a policy-5 finding).
4. A short seeded sweep (a few hundred descriptors per arm) runs as a
   plain `#[test]` so CI exercises the space without the fuzzer.

## Technical direction

Extend `theorygen`'s vocabulary tables, not its control flow. Record
in this file's Results: the achieved arity/type coverage histogram
from one 50k-run theory session and one 25k ops session, plus any
engine or model finding (trophy discipline applies).

## Passing criteria

- `[shape]` The generator arms exist behind `Rng`; digest pin
  byte-unchanged.
- `[test]` The seeded sweep test green; `cargo fuzz run theory --
  -runs=50000` and `ops -- -runs=25000` finding-free (or trophied);
  the over-width rejection observed under generation (assert the
  diagnostic appears in the sweep).
- `[shape]` The coverage histogram recorded in Results; zero engine
  source changes (`git diff --stat` confined to bench + fuzz + this
  file) unless a policy-5 finding forced one (then: its own commit,
  recorded).
- `[gate]` Fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

`60-validation.md`'s generator section: one sentence — the descriptor
space now sweeps projection arity and type mix to the width bound.
