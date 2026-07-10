# PRD 06 — Oracles over axioms

**Depends on:** 05 (the final shapes it must judge).
**Modules:** `crates/bumbledb-bench/src/` — the naive model, the IR→SQL
translator + mirror schema, the query generator, the differential runner.
**Authority:** `60-validation.md` (the two-oracle construction is normative;
the naive model must share types, never algorithms).
**Representation move:** none — this is the tax the axioms charge. A
representation the oracles cannot judge does not exist; closed relations,
compiled subsets, and the six-type roster enter the oracle surface here,
before any of Phase C's folds are allowed to claim correctness.

## Context (decided shape)

- **Naive model**: closed relations are BTreeSets seeded from the descriptor
  extension at model construction (the model may "seed" — it is a model, not
  the engine); writes to them rejected with the same typed verdict; the
  membership judgment for closed-target containments implemented from the
  σ-over-extension *definition*, never from bitsets (independence law — the
  model must not share the engine's compiled representation).
- **SQLite mirror**: a closed relation becomes an ordinary mirrored table
  INSERTed from the extension at mirror-build time; references become
  INTEGER columns + the mirror's usual FK-equivalent checks where
  expressible; ψ-subsets are judged only by the naive model (SQLite cannot
  express commit-time CINDs — already the recorded division of labor).
- **Query generator**: closed relations join the drawable atom pool —
  specifically generating (a) joins against closed relations with and
  without payload-column selections, (b) handle literals and handle param
  sets on referencing fields, (c) the fold-shaped pattern PRD 07 targets
  (closed atom whose only escaping variable is the join id) so the fold, when
  it lands, is *already* under adversarial differential coverage, (d)
  out-of-range id writes for the judgment scenarios.
- **Differential write streams**: op streams that attempt closed-relation
  writes (verdict parity on `ClosedRelationWrite`), subset-violating inserts,
  and domain-quantification deletes — verdict parity including the typed
  identity and statement id (the direction-divergence lesson, applied at
  birth).
- **Verdict-parity on errors** extends to the new validation roster: the
  generator's schema-fuzzer (if present in baseline; else the fixture
  battery) covers PRD 01's roster errors on both sides where the model
  validates schemas.

## Technical direction

1. Naive model: `naive.rs` gains the extension-seeded relations and the
   definitional membership check (`σ` applied to extension rows by value
   comparison — reuse the model's existing literal comparison, not the
   engine's encodings).
2. Translator: mirror DDL for closed relations (CREATE TABLE + INSERTs from
   the descriptor); the translator refuses nothing new — closed atoms are
   ordinary tables on the SQLite side, which is exactly what makes the
   differential meaningful for the folds.
3. Generator: extend the theory-drawing and query-drawing arms; the
   fold-shaped pattern gets its own family knob so PRD 07 can point at it.
4. Runner: the three write-stream scenario classes above with
   verdict+payload equality assertions.

## Passing criteria

- `[test]` Family + randomized differential green over a closed-relation
  fixture theory: engine results == SQLite results == naive results for
  reads; engine verdicts == naive verdicts (typed, with statement ids) for
  the three write classes.
- `[test]` The naive membership check is definitional: a unit test feeds it
  a ψ-subset and asserts against hand-computed rows (no engine types beyond
  the shared `Value`).
- `[shape]` The model contains no bitsets and imports nothing from
  `schema::Resolved` (the independence grep).
- `[shape]` The generator emits all four query/write pattern classes
  (counted in a generator self-test, the PRD-21-cookbook counting pattern).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`60-validation.md`: closed relations join the oracle surface table; the
division of labor row (ψ-subsets: naive-only).
