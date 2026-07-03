# PRD 11 — The randomized query generator

Authority: `50-validation.md` (the generator's feature-coverage contract, itself
asserted), `20-query-ir.md` (what a valid query is), the suite ruling that 2-way
agreement (engine vs SQLite) replaces the 3-way reference engine.

## Purpose

Seeded random valid queries over the ledger schema, every one of which the engine
validates and the translator translates — the fuel for `verify`'s randomized half.

## Technical direction

- `querygen::random_query(rng: &mut Rng, schema: &Schema) -> Query` built from a
  shape grammar (weights in a const table, documented):
  - **guard** (10%): one atom, serial id bound to a param, 1–2 vars projected.
  - **star** (20%): Posting joined to 1–3 of {Account, Instrument, Transfer} on its
    FK fields.
  - **chain** (20%): Holder ← Account ← Posting (2–3 hops), projecting ends.
  - **self-join** (10%): two Posting occurrences equated on `transfer`, projecting
    both amounts.
  - **gated** (10%): any of the above plus a zero-binding Tag gate atom.
  - **aggregate** (20%): any join shape re-projected as group-by + one of
    Sum(amount)/Count/Min(at)/Max(amount); group key = 0–2 of the bound vars.
  - **filter dressing** (applied to all with 60% chance, 1–3 predicates): i64
    range ops on amount/at (literal or param); Eq/Ne on memo with, at equal
    weight, an in-vocabulary literal, an out-of-vocabulary literal (the miss
    path), or a param; Eq on enums/bools; same-atom var-vs-var (amount vs at is a
    type conflict — only same-typed pairs: amount vs at are both i64 ✓ allowed).
  - **repeated in-atom var** (5% of atoms where two same-typed fields exist).
- Construction is correct **by construction** (fresh dense VarIds, dense ParamIds,
  typed literals from the schema walk) — the generator never emits an invalid
  query; `validate()` is the assertion, not the filter.
- Coverage contract, asserted: `querygen::coverage(n, seed) -> Coverage` counts
  every construct over n queries; the contract test requires every counter > 0 at
  n = 1000 (self-joins, gates, misses, params, repeated vars, every aggregate op,
  every comparison op, every shape).
- `querygen::params_for(query, rng, corpus_cfg) -> Vec<Vec<Value>>`: 4 param sets
  per query — in-range hits, boundary values, and (for string params) one
  guaranteed miss.

## Non-goals

Queries beyond the ledger schema; negation/recursion; queries the translator
cannot express (the grammar and translator are co-total by test).

## Passing criteria

- Unit tests: 1000 generated queries (seed pinned) ALL pass `validate` (via a
  prepare against an S corpus db is overkill here — call the engine's validate
  through `Db::prepare` on an empty schema-loaded db) AND `translate` returns Ok;
  coverage contract passes at n = 1000 and is itself golden-pinned (counts within
  ±30% of expectation bands so weight regressions surface); determinism (same
  seed ⇒ identical Debug rendering of query #500); params_for produces the
  documented 4 sets with a miss where applicable.
- `scripts/check.sh` green.
