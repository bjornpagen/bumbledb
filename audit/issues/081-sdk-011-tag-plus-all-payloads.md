# sdk-011: discriminator-plus-all-payloads across the C++ builder IR

- **Severity:** medium
- **Tree:** sdk (cpp dialect)
- **Status:** FIXED(57a755a0)
- **Source:** audit/sdks.md #11
- **Depends on:** sdk-001/002 (the IR restructure); sdk-009 lands inside this reshape

## The bug

Every "sum" in `cpp/src/query/ir.cc` is a struct carrying a tag plus EVERY alternative's fields simultaneously:

- `query_literal` (`ir.cc:26-35`): `kind` plus bool, u64, i64, and two interval pairs at once.
- `term_data` (`ir.cc:59-66`): `form` plus variable coord, param name, literal, membership array.
- `body_item` (`ir.cc:146-151`): atom, interior, AND condition payloads side by side.
- `find_data` (`ir.cc:177-187`): `has_over` / `classed` as existence flags.
- `param_data` (`ir.cc:204-212`): `point` / `membership` bools — eight states from three bools, few valid.

`cpp/AGENTS.md` §8 makes `std::variant` the blessed closed sum and forbids "manual discriminator + payload structs" verbatim; it is unused here. NTTP/consteval wanting trivial types is a constraint, not a representation.

## Why it's wrong

This is Minsky's three-boolean problem as a data layout (Insight 4): the state space is the product of all payloads times the tag, and only a sliver is meaningful; every consumer switch reads the tag and trusts the right payload was the last one written. The dialect's own law names the fix.

## The fix

Per `audit/CONTRACT.md §C6` (C++), in preference order:

1. `std::variant` for literals, terms, body items, finds (four FindTerm cases per sdk-004, Count without `has_over`) — IF variant is NTTP-usable on the pinned toolchain (verify with a `static_assert(std::is_structural_v<...>)` probe first; record the result in the commit).
2. If variant fails NTTP: per-alternative ARRAYS — the engine's own `atoms` / `negated` / `conditions` bucketing (rules already bucket at lower time, so recording a mixed `body_item` then re-bucketing is a detour to delete).

Either way: no struct in the recorded IR carries a tag plus more than one alternative's payload; `find_data.has_over`/`classed` and `param_data.point`/`membership` become cases.

## Acceptance criteria

- [ ] Gone: `rg -n 'bool point;|bool membership;|bool classed;' cpp/src/query/ir.cc` → no matches; no recorded-IR struct has a `form`/`kind` field beside multiple payload alternatives (review gate; grep aid: `rg -n 'query_term_form form' cpp/src/query/ir.cc` → the multi-payload `term_data` shape gone).
- [ ] Unchanged tests: cpp `ctest` green with zero assertion edits; wire bytes identical.
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`.

## Constraints

- Consteval/NTTP viability is the real constraint — probe FIRST, choose the arm, and say which in the commit. Semantics identical. Coordinate with sdk-001/002/009 (same files, one campaign).
