# bench-009: "program" still names a Query across the remaining crate

- **Severity:** low
- **Tree:** bench
- **Status:** FIXED(7ef4b2ad)
- **Source:** audit/bench.md F9
- **Depends on:** none (prose; parallel-safe). Skip sites owned by bench-002 (irgen) and bench-007 (closure) if those land first.

## The bug

"Program" still names a Query in remaining bench files (wave-1 oracles excluded):

- `querygen/shapes_rules.rs:1` — "programs of 2–4 rules"
- `querygen/construct.rs:60,83` — "multi-rule programs assemble their own query"
- `querygen/contradict.rs:15` — "the whole program denotes ∅"
- `querygen/tests.rs:127` — "Multi-rule programs"
- `verify/run_algebra.rs:4,26,149,602-603,654,680` — "multi-rule programs," "vanished program"
- `calendar/families.rs:169` — "a three-rule program"
- `conformance.rs:1247` — "union_idempotent at the program level"
- `corpus_gen/irgen.rs:39,49,58` — "valid and invalid programs" (also bench-002)

## Why it's wrong

C7: no `program` as our noun. Coverage reports and algebra row names are what maintainers grep; they still teach that a rule-list is a Program (Insight 1). The denotation did not keep Program. The names did.

## The fix

Per `audit/CONTRACT.md` §C7: "multi-rule query," "vanished query (empty Or)," "union idempotent at the query," "three-rule family." Keep "programmer error" (English). Keep SQL `WITH RECURSIVE` as translator spelling. Keep `Command::new(program)` (OS). Keep scenario "predicate columns" (SQL WHERE indexes) and DNF "predicate trees" (condition trees).

## Acceptance criteria

- [ ] Gone: `rg -inw 'program' crates/bumbledb-bench/src --glob '!naive/query.rs'` naming a Query (allowed leftovers listed in the commit message: programmer-error, OS program, SQL WITH RECURSIVE as translator spelling). The `TooManyCtes`-style absence pins are N/A here.
- [ ] Unchanged tests: algebra row labels may change strings; assertions on verdicts/answers do not.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench`; `./scripts/check.sh`.

## Constraints

- Prose/rename only. Do not retarget `Shape::Rules` (that assembler is a real multi-rule Query). Coordinate with bench-002/007 if those files move in the same wave.
