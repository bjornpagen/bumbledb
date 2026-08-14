# sdk-18: compile-fail holes — C++ and TS do not pin the cutover the way `query!` does

Severity: med
Tree: sdk (cpp + ts)
Status: OPEN
Source: audit/sdks.md #18
Blocked-by: sdk-01, sdk-04, sdk-05, sdk-13
Blocks: none

## Bug

Rust `query!` pins the cutover (named_head_without_keyword, phase
order, one rec, nonempty base/rec, bare main). C++ has consteval
traps, not types, and no fixtures for: recursive-after-main (only
`query_interior_after_main.cc` exists), interiors-only/rec-only
`prepare<>`, Measure as a find column (unwritable), condition trees
(unwritable), negation-in-rec. TS: second-recursive,
interior-after-recursive, duplicate/empty interior name are runtime
throws (`lower.ts:1614-1642`; `query.test.ts:1161-1215`) where
after-main is `never`.

## Why it is wrong

The suite chases traps instead of BEING the type (Insight 14). Once
sdk-01/05 land, the fixtures are the lock that keeps the phase
machines from regressing.

## Fix

Cites CONTRACT C6. After the phase machines land, add:

- C++ compile-fail: `query_recursive_after_main.cc`,
  `query_second_recursive.cc`, `query_interior_after_recursive.cc`,
  `query_prepare_interiors_only.cc`, `query_prepare_rec_only.cc`,
  negation-in-rec fixture (consteval or type-level, whichever
  sdk-01 makes true — the fixture asserts the actual mechanism).
- C++ compile-SUCCESS: measure find column (sdk-04), condition tree
  (sdk-13).
- TS expect-error fixtures: second-recursive,
  interior-after-recursive (sdk-05), impostor-Query (sdk-16),
  Count-with-over (sdk-06).

## Acceptance criteria

- [ ] Every fixture above exists, runs in the suites' normal lanes,
      and fails/passes for the STATED reason (each file's header
      names the rule it pins).
- [ ] `cargo test -p bumbledb-query` (existing fixtures UNCHANGED),
      cpp suite, `pnpm test` green; `bash scripts/check.sh` green.

## Constraints

No fixture that pins a runtime throw where the type now refuses —
pin the type. No Program vocabulary.
