# PRD 09 — Measure: the IR stops calling the measure a Duration

**Depends on:** 06 (same files; serialize).
**Modules:** `crates/bumbledb/src/ir.rs` (`Term::Duration(VarId)` :80,
`FindTerm::Duration` :163, `FindTerm::AggregateDuration` :169),
`ir/validate/finds.rs`, `ir/render.rs`, `crates/bumbledb-query` (the
macro parses the surface keyword and lowers it), bench mirrors
(querygen/translate/naive), docs.
**Authority:** the census: the IR variant says `Duration` while the
error (`MeasureOfRay`), the sink (`projection/measured.rs`, the
`Measured` sink), the tests (`tests/measure.rs`), and all downstream
prose say measure. One concept, two names, split exactly at the IR
boundary. The concept IS the measure (point-set cardinality, `end −
start`); "duration" is the wall-clock-flavored surface word.
**Representation move:** none — rename to the concept's real name;
the surface keyword stays.

## Context (decided shape)

- `Term::Duration(VarId)` → `Term::Measure(VarId)`.
- `FindTerm::Duration(VarId)` → `FindTerm::Measure(VarId)`.
- `FindTerm::AggregateDuration { op, over }` →
  `FindTerm::AggregateMeasure { op, over }`.
- The `query!` macro's SURFACE keyword remains `Duration(x)` — hosts
  write the word they mean colloquially; the macro lowers it to
  `Measure`. The macro's doc comment carries the one-line mapping
  ("`Duration(iv)` denotes the measure — the point-set cardinality
  `end − start`; a ray has no measure").
- `ir/render.rs` renders the surface keyword `Duration` (round-trip
  law: render(parse(q)) is stable) — goldens therefore unchanged;
  assert zero golden churn.
- Doc comments on the renamed variants state the denotation and cite
  `Error::MeasureOfRay`.
- NOT renamed: `MeasureOfRay`, `measured.rs`, `Measured` sink,
  `tests/measure.rs` — they were already right.

## Technical direction

Compiler-driven; the macro lowering table is the one non-mechanical
site (bumbledb-query keyword → IR variant). Bench querygen/translate/
naive re-anchor mechanically.

## Passing criteria

- `[shape]` `grep -rn "Term::Duration\|FindTerm::Duration\|AggregateDuration" crates fuzz` → zero.
- `[shape]` The macro still accepts `Duration(x)` (compile test) and
  the renderer still emits it (goldens byte-identical — zero churn).
- `[test]` Full suite green, unchanged assertion values; measure tests
  (`tests/measure.rs`, 42 hits) untouched and green.
- `[gate]` Fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

`20-query-ir.md`'s measure section: one sentence pinning the
vocabulary — "surface `Duration`, IR `Measure`, denotation: point-set
cardinality; rays are refused at evaluation (`MeasureOfRay`)."
