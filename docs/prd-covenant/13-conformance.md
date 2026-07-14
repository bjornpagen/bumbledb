# PRD 13 — Conformance: the model executes as the third oracle

**Depends on:** 04 (`evalList` + soundness), 05 (computable pack/
classify). Independent of 11/12.
**Modules:** `lean/Bumbledb/Conformance.lean` + `lean/Main.lean` (a
`lake exe` driver), `crates/bumbledb-bench/src/conformance.rs` (the
world/query serializer + the Rust-side comparator), a plain `#[test]`
conformance suite, `scripts/lean.sh` (the exe build), CI.
**Authority:** the dual-oracle blind spot, named repeatedly this
campaign: the engine and the naive model were written from the same
docs — a shared misreading passes every differential forever. The
Lean denotation is derived from the mathematics; evaluating it on real
Tiny worlds closes the one class nothing else can see. This is the
covenant's ambitious closer: the spec stops being commentary and
starts REFEREEING.
**Representation move:** the third lane exists — engine vs naive
model vs the formal denotation itself, on generated instances.

## Context (decided shape)

1. **The interchange format** — one JSON document per case (design
   for hand-readability; it will be the debugging surface):
   `{ theory: {relations, ground_axioms}, instance: {rel → [facts]},
   query: <the IR, serialized>, params: [...], answers: [[values]] }`
   — values in a tagged form (`{"i64": -3}`, `{"interval_u64":
   [0, 18446744073709551615]}` for rays); answers CANONICALLY SORTED
   by the serializer so comparison is byte equality.
2. **Rust side** (`bench/conformance.rs`): serialize a Tiny world +
   query + the ENGINE's answers from the existing generator machinery
   behind the `Rng` seam (valid-arm only; the hostile arm is out of
   scope — the model types its inputs). A corpus builder writes N
   seeded cases to `lean/conformance/cases/*.json` (checked in — the
   replay corpus; N ≈ 200 seeded + the exact-partition/aggregate/
   Allen shapes hand-picked from the cookbook schemas).
3. **Lean side**: `Conformance.lean` — JSON decoding into 03/04's
   types (core Lean `Lean.Data.Json` — allowed, it is core, recorded
   under law 4), evaluation via 04's `evalList` (whose soundness
   theorem is exactly why this lane is a DENOTATION check, not a
   third implementation: the executable form is proved equal to the
   spec), canonical sort, emit. `Main.lean`: read case file(s), print
   answers or a mismatch report. `lake exe conformance cases/`.
4. **The comparator test** (Rust, plain `#[test]`): for each corpus
   case, run the engine fresh, run `lake exe conformance`, compare
   all three (engine, naive, Lean) — any disagreement names the case
   file. CI: the lean job grows the exe build + corpus run (measure;
   if wall time demands, the corpus run joins the Miri cron — record
   the number).
5. **Scope fences**: Tiny scale only; the accepted valid fragment
   only; queries the serializer can express (it expresses the whole
   IR — the fragment limit is the generator's, counted and logged,
   never silent: the crucible's no-silent-caps rule). Params
   supported; the latch's unresolved-literal case EXCLUDED and noted
   (the model has no intern dictionary — a recorded, principled
   exclusion).
6. **A disagreement is a trophy** with three possible verdicts, all
   good: engine bug (the jackpot), naive-model bug (the shared-
   misreading class — the lane's whole point), or model bug (the
   spec was wrong — a design finding). Triage discipline per the
   fuzzing charter.

## Technical direction

Build order: serializer + 5 hand cases → Lean decode + eval on those
5 (debug the format at hand scale) → the seeded 200 → the comparator
test → CI. `evalList` performance: Tiny worlds are ≤ ~1k facts; naive
List evaluation is fine — if a case exceeds seconds, shrink the case,
not the model (the model stays pure; `partial` allowed in Main.lean's
IO shell only). The engine's `pub` needs: answers extraction exists
(`Answers`); the serializer may need one accessor — record any
visibility change.

## Passing criteria

- `[shape]` The corpus checked in (count recorded); the format
  documented in `lean/conformance/README.md` with one annotated
  example.
- `[test]` The three-way comparator green over the full corpus (or
  disagreements triaged as trophies with verdicts — report
  prominently; a found engine/naive bug does NOT block the PRD, it
  crowns it).
- `[shape]` The fragment-coverage count logged (cases expressible /
  generated); the two recorded exclusions (hostile arm, unresolved
  literals) in the README.
- `[gate]` `scripts/lean.sh` builds the exe; CI wall-time measured
  and the lane placed accordingly (number in the workflow comment);
  fingerprint + digest pins untouched.

## Doc amendments

`60-validation.md`'s oracle roster gains the third lane (one
paragraph: what it sees that the others cannot, citing
`eval_sound`); the fuzzing charter cross-references it.

## Results

Executed 2026-07-14. The three-way comparison ran and is GREEN: all 217
checked-in cases agree across the engine, the naive model, and the Lean
denotation — no trophies this build (the lane's teeth were verified by
mutation: a single altered answer value is caught and the case file
named with the differing rows).

* **Corpus**: 217 cases checked in (`lean/conformance/cases/`, ~22 MB)
  = 200 seeded (querygen valid arm, `Rng::new(case_seed)` recorded in
  each file's provenance, two Tiny worlds) + 17 hand cases (Pack over
  the exactly-partitioned Mandate groups with rays, empty-global
  Count+Sum, ArgMax ties, composite Allen mask, Allen mask PARAM,
  negation, overlapping-rule union, multi-rule aggregate union fold,
  CountDistinct over strings, measure find/predicate/fold, var+literal
  membership, interval-param equality, param set, closed join).
  Shape census over the files: 65 aggregate cases, 28 Allen, 46
  negation, 26 `point_in`, 19 multi-rule, 14 param-set, 13 measure,
  10 Arg, 5 measure-fold.
* **Fragment coverage**: 217/323 expressible, logged by the builder and
  comparator (`Report::coverage_line`): 31 unresolved-literal (the
  recorded intern-dictionary exclusion), 5 negated-membership,
  0 set-membership, 0 engine-error, 61 slow (naive > 25 ms — the Lean
  evaluator's cost proxy), 9 wide (> 512 answer rows). All recorded in
  `lean/conformance/README.md`; the hostile arm was never drawn.
* **Wall time** (pinned M2 Max, 2026-07-14): `lake exe conformance`
  over the corpus ≈ 1.0 s (948 ms evaluation) — wired into
  `scripts/lean.sh` as battery 4, comfortably per-push. The full
  three-way comparator (`cargo test -p bumbledb-bench
  three_way_conformance -- --ignored`) = 12.4 s engine+naive
  provenance replay/byte-comparison + 1.2 s Lean, debug build; placed
  in the CI lean job (numbers in the workflow comment), the
  byte-replay half (`the_corpus_replays_byte_identical`) runs in the
  plain workspace suite.
* **Determinism note** (a design finding during landing): the builder's
  slow/wide budgets are wall-clock measurements, so re-deriving the
  exclusion stream inside the comparator was flaky under parallel test
  load. The comparator therefore replays each checked-in file FROM ITS
  RECORDED PROVENANCE (hand name, or `case_seed` + draw) and never
  re-measures — its verdict is load-independent; the budgets bind only
  the deliberate `regenerate_the_conformance_corpus` run (quiet-machine
  note on the test).
* **Engine `pub` needs**: none — `differential::engine_query` and the
  public `Answers` surface sufficed, as expected.
* **The membership lowering** (recorded): the serializer performs the
  validator's bivalent resolution — an element-typed term on an
  interval field becomes a fresh interval variable + `PointIn`
  condition (the predicate form the typing rule licenses); the engine
  runs the original query, Lean the lowered one. Two shapes have no
  lowering and are the counted exclusions above: membership inside a
  negated atom, and element-typed param-set membership (the lowered
  set-`PointIn` would break the `WellTyped` premise `eval_sound`
  names).
* **Law 4 record**: core `Lean.Data.Json` adopted for decoding
  (`lean/Bumbledb/Conformance.lean`, module doc) — core Lean, no
  package dependency.
* **Aggregate glue** (recorded in the module doc): head shapes beyond
  plain projection evaluate as compositions of PRD 04/05's proved
  pieces — `evalList` join states, `dedup` distinct bindings, fibered
  grouping, `checkedSum`, `pack`, `classifyRefined`, encoded-word order
  keys, tie-keeping Arg restriction; the multi-rule aggregate head
  folds the union of head-projected binding sets (the rules-IR
  definition).
* **Bridge**: the `Query.eval_sound` row's instrument (pending this
  lane since PRD 10) now names
  `three_way_conformance_over_the_checked_in_corpus` — 68 rows, 188
  tokens, census green.
