# sdk-012: C++ sugar caps — `max_query_rules = 4` is the `max_interiors` that survived

- **Severity:** medium
- **Tree:** sdk (cpp dialect)
- **Status:** OPEN
- **Source:** audit/sdks.md #12
- **Depends on:** sdk-001 (NR in the template changes what needs a cap at all)

## The bug

`cpp/src/query/ir.cc:10-16`:

```cpp
/** Builder capacities: SDK bounds only — the engine's own caps are far higher. */
inline constexpr std::size_t max_query_rules = 4;
inline constexpr std::size_t max_query_atoms = 8;
inline constexpr std::size_t max_query_conditions = 8;
inline constexpr std::size_t max_query_finds = 8;
inline constexpr std::size_t max_query_params = 8;
inline constexpr std::size_t max_query_vars = 32;
```

Engine: `MAX_RULES = 16` per list, rec pooled, NO interior-count cap. The comment confesses "SDK bounds only" — a second theory of size. Worse, `rec_ir` (`ir.cc:317-328`) carries TWO arrays of `max_query_rules` each, so the struct admits 8 rec-pool rules while `static_assert(base+rec <= 4)` (`query.cc:203-204`) is a flowchart on top. Interior COUNT is uncapped (`NI + 1`) — they deleted `max_interiors` and left its cousins.

## Why it's wrong

A number invented by the SDK is a fact about nothing (Insight 10: magic sizes are a second, undocumented theory the engine will disagree with); a struct that admits 8 where the law says 4 plus an assert is the illegal state representable with a guard bolted on (Insight 4).

## The fix

Per `audit/CONTRACT.md §C6` (C++): the engine's cap is the ONE number.

- Import/mirror the engine's `MAX_RULES = 16` as the array bound (one constant, cited to the engine's constant by name in the doc comment); delete the invented `4`. The other capacities (`atoms`/`finds`/`params`/`vars`) either align to the engine's corresponding caps where they exist or — where the engine has none — become per-query template-derived sizes (the pack length IS the bound, like `NI`).
- `rec_ir` becomes ONE pooled array of `MAX_RULES` plus `base_count` — the `static_assert(base+rec <= …)` becomes the array's own capacity, not a law on top of a wider struct.

## Acceptance criteria

- [ ] Gone: `rg -n 'max_query_rules = 4|SDK bounds only' cpp/src` → no matches; `rg -n 'std::array<rule_data, max_query_rules>.*\n.*std::array<rule_data, max_query_rules>' cpp/src/query/ir.cc -U` → no dual rec arrays.
- [ ] Unchanged tests: cpp `ctest` green; existing queries (all ≤ old caps) lower identically.
- [ ] New lock: a compile-success fixture with >4 rules in one interior (legal per engine, unwritable before this fix).
- [ ] Green: `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`.

## Constraints

- NO new caps anywhere (`MAX_CTES`/`MAX_INTERIORS` stay dead — do not reintroduce an interior-count bound while touching this). Widening the SDK to engine-legal sizes is not a semantics change: the engine was always the authority.
