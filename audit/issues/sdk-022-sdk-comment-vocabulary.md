# sdk-022: stratum/SCC/program vocabulary in SDK comments and messages

- **Severity:** low
- **Tree:** sdk (cpp + ts + rust macros)
- **Status:** OPEN
- **Source:** adversarial pass (not in audit/sdks.md)
- **Depends on:** none (prose/strings; land after sdk-005 to avoid churn on lower.ts)

## The bug

The deleted coordinates survive in SDK prose and user-facing strings:

- `cpp/src/query/rule.cc:190` — comment: "negates no stratum" (Datalog stratification vocabulary in the dialect's negation docs).
- `ts/src/query/lower.ts:1636` — runtime error string: "query: a second recursive is unwritable — this cut admits one rec SCC".
- `crates/bumbledb-query/tests/notation_corpus.rs:14,662,705` — corpus prose calls a case's compact query JSON "the program" ("editing a case's notation, normalized text, or program fails here", "the program compact (the exact `JSON.stringify`").
- `ts/src/query/run.ts:42` — "a literal set folded into the program"; `ts/test/query-closed-literals.test.ts` (6 hits: "the pinned program", "wire program", "one-spelling program"); `ts/test/answers-named-orderable-ban.test.ts:183` — "wire program never moved".
- `cpp/bridge/src/tests.rs` (4 hits) — "The DownAt program" prose and closures named `|program|` binding a query view.

NOT findings (genuine English/compiler senses, keep): "program constant" (`cpp/src/query/ir.cc:41`, `ts/src/query/run.ts:43`, `ts/src/query/lower.ts:1393`, `ts/src/query/atom.ts:53` — a value fixed at build time), "whole-program-optimized". The example RELATION `Program` in `cpp/tests/cookbook/r30_keyed_read.cc` and `crates/bumbledb-query/tests/cookbook.rs` is recipe-30 data owned by docs-023.

## Why it's wrong

Insight 1: comments and error messages are the SDK's teaching surface; each hit trains a user or contributor in the deleted model (stratification for negation walls, SCC for the one rec, program for a query's JSON).

## The fix

Per `audit/CONTRACT.md §C7` vocabulary:

- `rule.cc:190`: state the wall positively ("negation is not part of the rec's arms" or the file's actual rule) without "stratum".
- `lower.ts:1636`: "…— this cut admits one linear rec". The test matching `/second recursive/` (`ts/test/query.test.ts:1161+`) still matches; if any test pins the full string, it updates mechanically in the same change.
- `notation_corpus.rs`: "the query compact" / "a case's notation, normalized text, or query JSON". Test names and assertions untouched.
- ts test prose: "wire program" → "wire IR" / "the pinned query"; `run.ts:42` → "a literal set folded into the query".
- `cpp/bridge/src/tests.rs`: "the DownAt query"; closures rename `|program|` → `|query|`.

## Acceptance criteria

- [ ] Gone: `rg -in 'stratum' cpp/src` → no matches; `rg -in 'scc' ts/src` → no matches; `rg -inw 'program' crates/bumbledb-query/tests/notation_corpus.rs` → no matches; `rg -inw 'program' ts/src ts/test cpp/bridge/src` → only the "program constant" English sense and docs-023's example-relation data remain.
- [ ] Unchanged tests: `cd cpp && ctest --preset dev` (after build), `cd ts && pnpm test`, `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-query` all green; only string-literal edits where a test pinned the exact message.
- [ ] Corpus case files untouched.

## Constraints

- Prose/message strings only; zero behavior change. The recipe-30 `Program` RELATION rename in `crates/bumbledb-query/tests/cookbook.rs` is docs-023's twin and lands THERE, not here.
