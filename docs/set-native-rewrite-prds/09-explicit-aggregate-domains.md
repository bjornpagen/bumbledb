# 09 Explicit Aggregate Domains

## Purpose

Implement aggregate semantics as operations over explicit sets. This PRD deletes the old multiplicity-based aggregate path.

## Required Aggregate IR

Aggregate terms must include:

```text
function
domain_vars
measure_var optional depending on function
output_type
```

Examples:

```text
count_domain([posting])
count_distinct(account)
sum(amount).over([posting])
min(t).over([posting])
max(t).over([posting])
```

## Required Semantics

`count_domain(domain_vars)` counts distinct domain tuples per group.

`count_distinct(var)` counts distinct values per group.

`sum(value).over(domain_vars)` sums one value per distinct domain tuple per group. If one domain tuple can bind multiple values, query building must reject the aggregate unless functional dependency proof is available.

`min/max(value).over(domain_vars)` use the values induced by the domain set.

## Required Code Changes

- Replace `AggregateSink` multiplicity updates with domain set state.
- Delete `AggregateState::apply_count_by` from semantic aggregate paths.
- Delete or quarantine `TupleSink::emit_count_range` from result-producing execution.
- Update reference models in test support and query tests.
- Update benchmark query builders to specify domains.
- Update prepared result cache keys to include aggregate domain semantics.

## Acceptance Gates

- No test named or asserting multiplicity remains for semantic aggregates.
- Count over duplicate hidden witnesses counts distinct domain tuples only.
- Sum over two different domain tuples with same value counts both.
- Sum over one domain tuple with duplicate hidden witnesses counts once.
- Ambiguous aggregate measure without domain proof is rejected.
- Golden aggregate values match exact expected rows.

## Tests Required

- Ledger balance by instrument uses posting domain.
- TPC-H revenue by customer uses lineitem domain.
- Triangle count uses explicit triangle domain or explicit projected-domain count.
- Empty global aggregate behavior preserved.
- Grouped empty aggregate behavior preserved.

## Non-Goals

- No SQL bag aggregate compatibility.
- No implicit guesswork for aggregate domains.
