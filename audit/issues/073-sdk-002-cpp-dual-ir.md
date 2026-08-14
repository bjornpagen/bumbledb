# sdk-002: dual Query IR in C++ — `query_ir` ⊕ `query_value` split one value in half

- **Severity:** high
- **Tree:** sdk (cpp dialect)
- **Status:** OPEN
- **Source:** audit/sdks.md #2
- **Depends on:** co-lands with sdk-001 (same restructure)

## The bug

Engine `Query` is one product. C++ splits it across two structs: interiors and rec ride `query_value` (`cpp/src/query/query.cc:107-112`), while main rules, main head, and the param registry ride `query_ir` (`cpp/src/query/ir.cc:331-345`), whose own comment admits the split ("Named interiors and the optional rec ride `query_value`, not this struct"). Every whole-query walk re-assembles the value by threading four arguments: `detail::append_rule(next.ir, next.interiors, next.has_rec, next.rec, result)` (`query.cc:121`), `derived_tables` (`cpp/src/query/lower.cc:371-387`), `query_view::for_each_wire_rule`.

## Why it's wrong

A table cut in half forces a flowchart to reassemble it at every consumer (Insight 3: the data structure should carry the connections; Insight 9: N sites re-derive one fact). The four-argument thread is the missing struct's field list travelling by hand.

## The fix

Per `audit/CONTRACT.md §C6` (C++): one `query_ir<NI, HasRec, NR>` value (or the sdk-001 phase machine's single aggregate) holding interiors, the conditional rec, and main. `append_rule`, `derived_tables`, and `query_view` take ONE argument. The "variadic pack, not fixed array" concern is solved the way `NI` already solves it — the count is a template parameter of the one struct, not a reason to park rec and main elsewhere.

## Acceptance criteria

- [ ] Gone: `rg -n 'ir, .*interiors, .*has_rec, .*rec' cpp/src` → no four-way thread; `rg -n 'not this struct' cpp/src` → no matches (the confessing comment dies with the split).
- [ ] Unchanged tests: all cpp `ctest` suites green with zero assertion edits; emitted wire bytes identical.
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`.

## Constraints

- Same change as sdk-001 (one fixer owns both). C ABI layout untouched. No Program vocabulary; no new caps.
