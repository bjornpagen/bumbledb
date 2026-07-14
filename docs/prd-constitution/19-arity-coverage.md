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
mix up to the encoded-width bound (`MAX_DETERMINANT_WIDTH`); the
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

## Results (2026-07-13)

- The mixed scalar cycle is `u64/i64/bool/str/bytes<64>`: arity 29 is
  the last legal point (470 encoded bytes), arity 28 is its under-bound
  neighbor (462 bytes), and arity 30 is the generated 534-byte refusal.
  The seeded tests sweep every legal arity and all selection placements,
  equality at 1/2/3/29, both keyless equality directions, and assert the
  exact `DeterminantKeyTooWide` width. The legacy descriptor and ops
  decisions complete before a fresh `Rng` cursor draws the extension, so
  the corpus stream and its digest remain unchanged while short-input
  fuzz cases retain live late-arm entropy.
- `cargo fuzz run theory -- -runs=50000` completed finding-free in 382 s.
  Arity counts for 1 through 30 were
  `[7754, 7587, 7179, 255, 237, 953, 594, 679, 242, 605, 179, 199,
  307, 1089, 1206, 497, 268, 348, 421, 728, 131, 378, 389, 1049, 142,
  180, 320, 220, 4078, 11786]`; type occurrences
  (`u64/i64/bool/str/bytes`) were
  `[157095, 147401, 138055, 128933, 121799]`; source/target/both
  selections were `[11066, 16174, 22760]`. The session observed 24,267
  equality cases, 34,659 reordered keys, 11,786 width refusals, 15,818
  missing-source-key refusals, and 1,637 missing-target-key refusals.
- `cargo fuzz run ops -- -runs=25000` completed finding-free in 744 s.
  Legal-arity counts for 1 through 29 were
  `[4508, 2376, 1165, 319, 350, 641, 497, 1023, 686, 420, 378, 361,
  360, 490, 839, 576, 388, 1257, 1031, 520, 196, 253, 520, 1132, 499,
  329, 498, 614, 2774]`; type occurrences were
  `[75914, 69286, 64913, 59974, 53542]`; source/target/both selections
  were `[6085, 10090, 8825]`. The session observed 8,254 equality cases
  and 18,116 reordered keys. Each case compared one accepted seed write
  and three typed aborts (key collision, missing source witness, target
  removal), for 100,000 strict engine/naive write-verdict comparisons.
- Engine findings: none. Model findings: none. Engine source changes:
  none.
