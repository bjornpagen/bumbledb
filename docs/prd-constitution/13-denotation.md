# PRD 13 — The denotation: the query contract completed, in answers

**Depends on:** 06, 09, 10 (cites final comparison/measure vocabulary).
**Modules:** `docs/architecture/20-query-ir.md` (the load-bearing one),
`10-data-model.md` (glossary line), `README.md` (the three-equalities
note), and — the language-law cut — the public answer carrier in
`crates/bumbledb/src/api/` (`Rows` and its result vocabulary) + every
"row(s)" saying query-output in docs and identifiers.
**Authority:** deep audit #4: the code is faithful to a SCATTERED
contract. Much is already normative on main (negation range
restriction, empty-global zero-rows, Sum overflow, the DNF story) —
verified. What is still unstated in one place: the atom-matching
equation, the three equality levels, the tuple-level dedup contract
(the sink owns union — exec/sink.rs:6-14 — but no doc SAYS result
identity is canonical head-tuple bytes), and the fact/row/tuple
glossary discipline.
**Representation move:** the answer vocabulary lands in the one public
carrier (a rename, ConditionTree-discipline); every added doc sentence
must cite the mechanism that makes it true.

## Context (decided shape) — the additions to 20-query-ir.md

1. **The matching equation** (one display block): for fact f and atom
   binding (field, term) — Var unbound binds; Var bound demands
   equality (same-fact unification within an atom, join across atoms);
   Param/Literal select; ParamSet selects membership. With the two
   laws already true in code and now stated: repeated variables are
   unification constraints, not post-filters; membership-only
   variables are refused.
2. **Answer identity and union:** "a query denotes the SET of its
   answers — the union of its rules' head projections; answer identity
   is the canonical head-tuple bytes; the sink IS the union (no merge
   node exists) and its seen-set is the semantics' dedup — elided only
   under the distinct-bindings proof (PRD 17's witness)." One
   paragraph, citing exec/sink.rs.
3. **The three equality levels** (also one README sentence):
   dependency `==` (key-backed view correspondence, PRD 12), selection
   `==` (σ equality inside a view), comparison `Eq` (typed
   term-equality predicate) — one concept, equality of denotations, at
   three types; never interchangeable in diagnostics.
4. **The glossary** (in 10-data-model.md), per the language law: fact
   = stored full tuple (identity = canonical bytes); **answer** = an
   output tuple of a query (the Datalog word — "row" is banned for
   query output); ground axiom = a closed relation's sealed element;
   tuple = the mathematical product in notation. A docs sweep for
   violations (fix stragglers, list them in the commit body). "Row"
   survives only as physical stride vocabulary and for SQLite's side
   of the differential.
5. **The answer cutover in code** (the language-law hard cut): the
   public result carrier `Rows` → `Answers` (api surface + every
   consumer: bench compare/verify/naive, cookbook compiled copies,
   tests), result-buffer prose and identifiers saying "row" for query
   output → answer (`out.arity` etc. keep their names; identifiers
   whose NAME says row-as-answer change — enumerate by grep, list the
   survivors that mean physical layout). Assertion VALUES unchanged;
   this is the ConditionTree discipline applied once more.
6. **The measure and PointIn rows** in the comparison/aggregate tables
   updated to the PRD 06/09/10 vocabulary (three-predicate line, typed
   order refusals, surface-Duration/IR-Measure mapping).

## Technical direction

Write against the code, not the spec: every added claim gets its
mechanism citation (file or chapter §) inline, in the house
citation style. Where the spec's §11 contract and current behavior
disagree, current behavior wins and the difference is NOT imported
(the reconciliation already established there are none outstanding
beyond items owned by other PRDs — if writing uncovers one, policy 5).

## Passing criteria

- `[shape]` 20-query-ir.md contains the matching equation block, the
  answer-identity/union paragraph, and the three-equalities note
  (grep for "matching" heading, "canonical head-tuple", "three
  types").
- `[shape]` The glossary present in 10-data-model.md; the straggler
  sweep's fix list in the commit body.
- `[shape]` `grep -rn "\bRows\b" crates/bumbledb/src/api` → zero (the
  carrier is `Answers`); the docs "row"-for-query-output battery zero
  outside the recorded physical/SQLite survivors.
- `[test]` Full workspace suite green with unchanged assertion values
  (mechanical re-anchors only — the Rows→Answers chase).
- `[gate]` Fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

This PRD is its amendments.
