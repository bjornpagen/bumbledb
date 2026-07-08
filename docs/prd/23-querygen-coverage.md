# PRD 23 — Query generator coverage

**Depends on:** 11, 12, 21 (the model is the co-executor), 22 (SQLite lane forms).
**Modules:** `crates/bumbledb-bench/src/querygen/`.
**Authority:** `docs/architecture/60-validation.md` (§ differential and property tests — the generator's feature-coverage contract, restated there in full).

## Goal

The randomized query generator emits the new surface, and its **coverage contract
test** (the self-asserting distribution check at n = 1000) is extended to pin it —
the contract in `60-validation.md` is the checklist; implement all of it.

## Technical direction

1. **New shapes to generate** (weights joining the existing shape table):
   negated atoms (0–2 per query, variables drawn from bound positive vars;
   key-covered and non-covered binding mixes; occasional literal/param/set
   bindings inside); param sets (sizes drawn from {0, 1, 2, large-boundary},
   duplicate elements injected, per-type miss elements per the existing miss
   policies); membership bindings (literal, param, and var points — the var case
   requires generating a scalar anchor for the point var first; make the
   generator construct the anchor deliberately, not hope for one);
   `Overlaps`/`Contains` predicates over interval-typed vars (including the
   adjacent-touching boundary in both polarities — generate data AND queries that
   hit `[a,b) [b,c)`); CountDistinct over every type; Arg-restriction with
   constructed ties and tie-free cases, both directions, key-projected variants.
2. **Type-legality matrix:** extend the per-(operator, type) matrix to seven
   types: `Eq`/`Ne` over all seven; order ops over the two integers ONLY —
   the illegal interval cells must be asserted **zero** (the generator must be
   structurally unable to emit them, not filtered after);
   `Overlaps`/`Contains` cells legal only at their typed shapes.
3. **Data generation:** relation fixtures gain interval columns with a dedicated
   interval-value generator: mixes disjoint, adjacent (`end == next.start`),
   nested, and `MAX_END`-sentinel intervals, plus scalar-prefix collision groups
   (several intervals per group — the shapes judgments and joins discriminate).
4. **Coverage contract test:** the existing ±30%-of-weight assertion extends to
   every new shape; the legal/illegal matrix assertion extends per §2; add
   structural assertions: at least one generated query per run contains
   (negation ∧ aggregate), (param set ∧ negation), (membership ∧ Overlaps) — the
   compositions where bugs hide.
5. Duplicate-witness data generation (the D2-skip exerciser) extends to negated
   queries: multiply-witnessed facts on the *negated* side (rejection must not
   depend on witness count).

## Out of scope

Running verify (human). Family definitions (24).

## Passing criteria

- `[shape]` Illegal matrix cells are unrepresentable in the generator's type-
  driven construction (no post-filtering of illegal emissions).
- `[test]` The coverage contract test at n = 1000 passes with every new assertion
  active (this test IS the passing criterion — it is the one place the
  architecture allows a distribution gate).
- `[test]` A seeded determinism test: same seed ⇒ identical query stream across
  two runs (the reproducibility property the oracle protocol depends on).
