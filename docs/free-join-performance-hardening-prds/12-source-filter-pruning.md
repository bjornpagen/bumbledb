# PRD 12: Source Filter Pruning

## Purpose

Turn literal/input predicates and simple range predicates into early source pruning with measured survivor counts.

## Current Problem

Source predicates are encoded before COLT source construction, but filtering still scans base-image offsets after the relevant columns are loaded. Empty JOB queries therefore often pay broad relation image and scan costs before proving emptiness.

## Required Design

- Represent filters in a typed, encoded form before source construction.
- Load only columns required for filtering and plan subatoms.
- Apply filters before building COLT maps.
- Emit rows-tested and survivors counters per filter and per atom.
- Short-circuit query execution when any atom source has zero survivors.
- Preserve residual predicate evaluation for cross-variable comparisons.

## Required Filter Kinds

- `Eq` for literals.
- `Eq` for runtime inputs.
- `NotEq`, `Lt`, `Lte`, `Gt`, `Gte` for orderable primitive values.
- `False` for missing dictionary entries or impossible inputs.

## Breaking Direction

- Do not preserve old `PredicateMode::ResidualOnly` except as a test-only differential mode if still useful.
- If keeping a residual-only mode, it must be clearly marked test-only and excluded from benchmark claims.

## Passing Criteria

- A test proves a missing string dictionary literal creates a zero-source without scanning unrelated columns.
- A test proves pushed range filters and residual evaluation produce identical results on a fixture.
- Trace for q09 reports survivor counts for `country_code = '[us]'`, `gender = 'm'`, and `role = 'actor'` sources.
- Empty-source short-circuit emits a dedicated span/counter.
- Global acceptance from PRD 00 passes.
