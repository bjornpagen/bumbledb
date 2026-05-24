# PRD 08: Selectivity-Aware Free Join Planner

## Purpose

Stop selecting plans that iterate broad fact relations first and then discard tens of thousands of bindings.

## Rosetta Alignment

The planner remains private. It must not import SQL semantics, bag semantics, or external optimizers.

## Paper Alignment

The Free Join paper starts from a cost-based binary plan and then factors it. Bumbledb does not use DuckDB as infrastructure, so it must grow its own Rosetta-compatible cost model over typed IR, storage stats, filters, and accelerators.

## Current Trace Evidence

`q09`, `q16`, and `q24` each yield about 32k tuples and hit about 32k binding conflicts before returning zero rows.

This is not a base-image-only problem. After physical reads improve, the next fire is plan shape and source access order.

## Required Planner Inputs

Use only internal stats:

- durable relation row counts;
- filter survivor estimates or exact accelerator counts;
- unique constraint knowledge;
- foreign-key graph knowledge;
- available accelerator coverage;
- current Free Join validation rules.

Do not call SQLite, DuckDB, or any SQL planner.

## Required Candidate Families

Keep existing families only if they still earn their place.

Generate additional candidates:

- filter-anchor plans starting from the most selective filtered dimensions;
- FK-walk plans from filtered dimensions into fact relations;
- projection-anchor plans when projected variable is selective;
- GJ-like plans for skewed shared variables;
- binary-derived factored plans as a baseline;
- singleton plans only as fallback.

Every generated plan must validate as formal Free Join before scoring.

## Required Cost Model

Score with explicit components:

- estimated rows iterated by each cover;
- estimated probe calls;
- expected binding conflicts;
- source build cost;
- accelerator availability;
- materialization/sink cost;
- plan node count as a tie-breaker only.

Cost output must be traceable. Every candidate must emit enough data to explain why it lost.

## Tests Required

- On a q09-like fixture, planner prefers a selective dimension/filter/FK path over broad `CastInfo` iteration.
- On a clover skew fixture, planner still prefers a factored Free Join shape.
- On a chain fixture, planner does not overfit to GJ-like plans when binary-derived is better.
- Invalid generated candidates are rejected and traced.
- Forced plan-family tests remain available for equivalence.

## Trace Requirements

Add candidate trace fields:

- family;
- node count;
- estimated source build rows;
- estimated cover rows;
- estimated probes;
- estimated conflicts;
- accelerator assumptions;
- total score;
- rejection reason if invalid.

## Benchmark Passing Criteria

Run full traced JOB sample.

Required evidence:

- `q09`, `q16`, and `q24` binding conflicts drop materially from post-PRD-07 baseline.
- Their selected plans are explainable from internal cost data.
- Exact SQLite comparisons pass for all 8 JOB sample queries.
- Planner time remains a minority cost and does not reintroduce base-image scans.
