# sdk-013: C++ conditions are leaves only; ABI trees are the back door

- **Severity:** medium
- **Tree:** sdk (cpp dialect)
- **Status:** OPEN
- **Source:** audit/sdks.md #13
- **Depends on:** sdk-011 (condition data reshapes with the variant campaign)

## The bug

Engine `ConditionTree = Leaf | And | Or`. C++ `wire_condition` (`cpp/src/query/ir.cc:272-277`) is one comparison; `condition_of` (`cpp/foreign/query_view.cc:142-157`) hardcodes `BDB_CONDITION_KIND_LEAF`. `and`/`or` trees are UNWRITABLE in the dialect while remaining writable as raw `bdb_condition` graphs (`cpp/foreign/bumbledb_c.h:232-237, 573-578`). Dual constructors: sugar is a strict subset, the ABI is the Program-era escape hatch.

## Why it's wrong

A dialect that cannot spell a legal engine sentence forces users through the untyped boundary (Insight 8: two languages, and the richer one is the unsafe one). TS already lowers condition trees — the C++ builder simply stopped at `.where(leaf)`.

## The fix

Per `audit/CONTRACT.md §C6` (C++): `wire_condition` IS a tree — `std::variant<leaf, and_node, or_node>` (or the sdk-011 per-alternative layout with child index ranges, which is also how the flat `bdb_condition` array already encodes children). Sugar gains `and(...)` / `or(...)` combinators lowering into it; `condition_of` maps the three kinds. No second path.

## Acceptance criteria

- [ ] Gone: `rg -n 'BDB_CONDITION_KIND_LEAF' cpp/foreign/query_view.cc` → not hardcoded (all three kinds mapped).
- [ ] New lock: a cpp runtime test with an `or` condition tree asserting answers match the same query via TS/`query!` (conformance parity), plus a compile-success fixture.
- [ ] Unchanged tests: existing leaf-only tests green with zero assertion edits; wire bytes for leaf conditions identical.
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`; `cd cpp/bridge && PATH="$HOME/.cargo/bin:$PATH" cargo test`.

## Constraints

- ABI layout unchanged (the tree encoding already exists there). No new caps for tree depth/width beyond the existing wire quantities. Semantics identical for existing queries; the new spelling covers already-legal IR only.
