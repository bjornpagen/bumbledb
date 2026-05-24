# PRD 02: Base Image Filter Pruning

## Purpose

Push source filters into the base-image loading boundary so filtered atoms avoid loading non-filter columns for rows that will never survive.

## Rosetta Alignment

This preserves set semantics. It only removes impossible source rows earlier under the same snapshot and the same typed predicates.

## Paper Alignment

The paper assumes selections are pushed down to base tables. Current Bumbledb applies source filters in `ColtBuild`, after the base image has already loaded all scoped columns. This PRD moves filter pruning to the base-table read boundary.

## Current Problem

`source_build.rs` does this today:

```text
encode source filters
extend base-image field scope with filter fields
load full base image for plan fields plus filter fields
build filtered COLT root offsets
```

That means selective filters still pay broad base-image load cost.

JOB examples:

- `CompanyName.country_code = '[us]'`
- `Name.gender = 'm'`
- `RoleType.role = 'actor'`
- `Keyword.keyword = 'hero'`
- `Title.production_year` ranges
- `Title.episode_nr` ranges

## Required Design

Create a base-image loading path that accepts encoded `SourceFilter`s.

Load filter columns first.

Compute survivor handles under the LMDB snapshot.

If survivors are empty:

- return an empty source image;
- emit `EmptySourceShortCircuit`;
- do not load non-filter plan columns;
- do not construct COLT maps for that atom.

If survivors are non-empty:

- load only requested plan columns for survivor handles;
- preserve row order over survivors;
- do not apply the same filter again inside COLT.

## Required API Direction

Introduce a function shaped like:

```rust
relation_base_image_filtered_with_trace(
    txn,
    schema,
    relation_name,
    plan_field_scope,
    filters,
    trace,
) -> Result<FilteredBaseImage>
```

The exact names may differ, but the responsibility must be explicit.

`source_build.rs` should no longer load a full image and then pass filters to COLT for routine filtering.

COLT may still accept filters for tests or defensive use, but production `build_sources` should pass no-op filters after base-image pruning is applied.

## Required Filter Semantics

Support all current `SourceFilterOp`s:

- `Eq`
- `NotEq`
- `Lt`
- `Lte`
- `Gt`
- `Gte`

Missing dictionary values must produce an impossible source before any base-image row scan.

Multiple filters on the same atom must be ANDed.

Residual predicates must still run at terminal binding. Pushdown is an optimization, not a semantics change.

## Tests Required

- Equality filter loads filter column and skips unrelated plan columns on zero survivors.
- Range filter on `I64` produces the same result as residual-only mode.
- String dictionary miss short-circuits without scanning relation columns.
- Multi-filter AND semantics match residual evaluation.
- Filtered image row order remains stable and column values align with survivor handles.
- Current predicate pushdown/residual equivalence tests still pass.

## Trace Requirements

Move these counters to the physical pruning point:

- `source_filter_rows_tested`
- `source_filter_survivors`
- `source_filter_false_decisions`
- `empty_source_short_circuits`

Add, if practical:

- filter columns loaded
- plan columns skipped due to pruning
- handles eliminated before COLT

## Benchmark Passing Criteria

Run the full traced JOB sample.

Required improvements against the post-PRD-01 baseline:

- `q09`, `q16`, and `q24` must show lower `column_values_loaded` than PRD 01.
- Any query with an impossible dictionary lookup must show zero base-image load for that atom.
- Total `BaseImageLoad` must drop materially beyond PRD 01 on filtered queries.
- Exact SQLite comparisons pass for all 8 JOB sample queries.
- `binding_copies` remains 0.

This PRD is not complete if filters are still first applied in `ColtBuild` for production query execution.
