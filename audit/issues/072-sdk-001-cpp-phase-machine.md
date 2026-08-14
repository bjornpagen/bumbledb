# sdk-001: C++ `query_value` is a Program flag-machine — phase belongs in the template

- **Severity:** high
- **Tree:** sdk (cpp dialect)
- **Status:** FIXED(0c77b514)
- **Source:** audit/sdks.md #1
- **Depends on:** none (foundation for the cpp tree; co-lands with sdk-002, sdk-019; sdk-012/018 build on it)
- **Conflicts with:** sdk-002, sdk-009, sdk-011, sdk-012, sdk-019 (same files; land per INDEX order)

## The bug

`cpp/src/query/query.cc:107-112` — the post-cutover Query is a sum of phases (interiors → optional rec → main), but C++ stores it as independent knobs on one public aggregate:

```cpp
template<class S, std::size_t NI = 0>
struct query_value {
	std::array<interior_ir, NI> interiors{};
	bool has_rec = false;
	rec_ir rec{};
	query_ir ir{};
```

- `has_rec == false` still carries a full `rec_ir`; `has_rec == true` with a default `rec` (empty counts, empty name) is well-typed.
- `.recursive` (`query.cc:198-209`) returns the SAME `query_value<S, NI>`, so a second `.recursive` / `.interior` after rec stay in the overload set; refusal is runtime-in-consteval: `if (has_rec) detail::a_second_recursive_is_refused();` (`query.cc:205-207`), `if (ir.rule_count != 0) detail::interior_or_recursive_after_a_main_rule();` (`query.cc:208-209`, and `:138-140` for interior).
- Main-rule count is `ir.rule_count`, a runtime counter; `Db::prepare<Query>()` (`cpp/src/db/db.cc:279-281`) takes ANY NTTP `query_value` — never-`.rule()`'d, interiors-only, rec-only all prepare and fail at the engine.
- `query_view` projects the flag into the ABI optional (`cpp/foreign/query_view.cc:529-537`): `.rec = Query.has_rec ? &query_rec<Query> : nullptr, .rule_count = Query.ir.rule_count` — 0 representable.

## Why it's wrong

Named derived tables + a recursive flag + an optional output + a constructor that takes whatever you stuff in IS the old Program state space with the name scraped off (Insight 2). The dialect already spends a type-level count on `NI`, proving it knows the move — and declined to spend it on rec and main (Insight 4: bools/counters where a sum belongs; Insight 5: the consteval traps are flowcharts compensating for the missing type).

## The fix

Per `audit/CONTRACT.md §C6` (C++): phase in the template — `query_value<S, NI, HasRec, NR>` (`bool HasRec`, `std::size_t NR` = main-rule count), or equivalently distinct phase types:

- `.interior` exists only when `!HasRec && NR == 0` (via `requires`); returns `query_value<S, NI+1, false, 0>`.
- `.recursive` exists only when `!HasRec && NR == 0`; returns `query_value<S, NI, true, 0>`. The runtime traps `a_second_recursive_is_refused` / `interior_or_recursive_after_a_main_rule` DELETE — the calls are ill-formed.
- `.rule` returns `query_value<S, NI, HasRec, NR+1>`.
- `rec_ir` is a member only when `HasRec` (empty-base / `[[no_unique_address]]` conditional member, or a `std::conditional_t` empty struct); `bool has_rec` dies.
- `Db::prepare` is constrained `requires (NR >= 1)` — empty main is unrepresentable at the SDK, not `EmptyRuleSet` at the engine. (Engine still validates; the DIALECT just can't mint it — CONTRACT §C6 "one refusal authority" stands.)
- `query_view.cc` reads `if constexpr (Query.has_rec)` → `if constexpr (HasRec)` and `NR` instead of `ir.rule_count` where the count is static.

Dialect law backs this: AGENTS.md §27 forbids state machines as independent booleans; §8 blesses sums.

## Acceptance criteria

- [ ] Gone: `rg -n 'bool has_rec' cpp/src` → no matches; `rg -n 'a_second_recursive_is_refused|interior_after_recursive' cpp/src` → no matches (the traps delete with the states).
- [ ] Unrepresentable, compile-checked: new compile-fail fixtures (sdk-018 names them) — `query_recursive_twice.cc`, `query_recursive_after_main.cc`, `query_prepare_without_main.cc` — fail to compile with the phase message.
- [ ] Unchanged tests: every existing runtime/conformance cpp test green with zero assertion edits; wire bytes emitted for existing queries identical.
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`; `cd cpp/bridge && PATH="$HOME/.cargo/bin:$PATH" cargo test`.

## Constraints

- Semantics identical: the engine IR and C ABI shapes DO NOT change here (sdk-008 owns the ABI find-term change). Locked names (`DerivedBudgetExceeded`, `set_derived_budget`, `DEFAULT_DERIVED_TUPLES`, `DEFAULT_REACH_ROUNDS`) untouched. No Program vocabulary; no new caps (sdk-012 deletes SDK caps — coordinate, don't add).
- Co-lands with sdk-002 (one IR struct). Lowering (`derived_tables`, `query_view`) reads `if constexpr (HasRec)` — no runtime bool (absorbs sdks.md #19).
