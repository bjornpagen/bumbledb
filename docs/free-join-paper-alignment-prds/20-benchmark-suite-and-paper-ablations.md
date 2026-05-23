# PRD 20: Benchmark Suite And Paper Ablations

## Purpose

Build correctness-first benchmarks that can evaluate paper-compliant Free Join without violating Rosetta. Benchmarks must prove exact set equality before measuring performance and must expose the paper's key ablations.

## Dependencies

- PRD 16.
- PRD 18.
- PRD 19.

## Scope

- `crates/bumbledb-bench` datasets, runners, config, reports, and gates.
- Benchmark scripts.
- SQLite reference checks.
- Paper-shaped synthetic fixtures.

## Required Correctness Rules

- Bumbledb result values must match SQLite `SELECT DISTINCT` projected values before timing matters.
- Count equality alone is never sufficient.
- Benchmark SQL must not use `COUNT`, `GROUP BY`, outer joins, anti-joins, null-sensitive predicates, or bag semantics unless a future Rosetta update adds the feature.
- Open-data ETL must not silently encode nulls as zero or empty string unless documented as a real domain value.
- Decimal/rating parsing must avoid `f64` rounding for persisted exact values.

## Required Benchmark Modes

- Singleton/GJ/LFTJ baseline where retained.
- Binary-derived Free Join.
- Factored Free Join.
- Static cover.
- Dynamic cover.
- Scalar batch size 1.
- Vectorized batch sizes 10, 100, and 1000.
- Materialized output.
- Factorized output.
- COLT source.
- Optional accelerator source when implemented.

## Required Datasets

- Existing ledger.
- Existing sailors.
- Existing joinstress.
- Existing TPC-H subset.
- Paper clover/sand-dollar skew fixture.
- Triangle cyclic fixture.
- Chain acyclic fixture.
- Star acyclic fixture.
- JOB sample clearly labeled as sample.
- Optional JOB full suite if data and query list are available.
- LSQB-compatible subset or a documented Bumbledb substitute for cyclic/acyclic paper coverage.

## Technical Direction

- Add CLI options for plan mode, cover mode, batch size, output mode, and source mode.
- Store run metadata in JSON: scale, dataset, query, plan mode, cover mode, batch size, output mode, source mode, git commit if available, hardware label if supplied, correctness fingerprint, and gate status.
- Add benchmark lints for `SELECT DISTINCT` and forbidden SQL features.
- Keep exploratory scripts able to run without gates, but add a strict script for final validation.
- Do not report performance for a query whose correctness check failed.

## Non-Goals

- Do not add DuckDB as a planner dependency.
- Do not add SQL frontend support.
- Do not implement LSQB features that violate Rosetta, such as nulls, anti-joins, or outer joins.

## Acceptance Criteria

- Benchmark runner can execute all required modes where implemented.
- Benchmark runner refuses or fails correctness when exact values differ.
- JSON and Markdown reports show plan mode, cover mode, batch size, output mode, source mode, and Free Join/COLT counters.
- At least one ablation fixture proves COLT laziness impact through counters.
- At least one ablation fixture proves vectorization mode through batch counters.
- At least one skew fixture shows factored/dynamic plan mechanics differ from naive binary-derived plan mechanics.
- Open-data ETL null and decimal risks are fixed or explicitly rejected.

## Required Tests

- Renderer tests for new fields.
- Benchmark lint tests for forbidden SQL features.
- End-to-end equal-count/different-value failure test.
- CLI parsing tests for modes.
- Small benchmark run for each mode on a tiny dataset.
- Open-data parser tests for null and exact decimal behavior.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-bench --bin bumbledb-bench --all-features
cargo test -p bumbledb-bench --bin bumbledb-bench renderer --all-features
cargo run -p bumbledb-bench -- --preset quick --format json --repeats 1 --warmup 0
```

If CLI names change, the final command must be replaced with the strictest equivalent quick correctness-first run.
