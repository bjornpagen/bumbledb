# PRD 05: Binary Plan IR And Bushy Decomposition

## Purpose

Introduce an internal binary join plan model over typed Bumbledb atom occurrences. This provides the required input to the paper's `binary2fj` conversion without adding SQL or DuckDB.

## Dependencies

- PRD 02.
- PRD 03.

## Scope

- New planner module for binary plans.
- Query planner tests.
- Benchmark/test-only hooks for injecting binary plans.

## Required Model

- `BinaryPlan::Leaf(AtomOccurrenceId)`.
- `BinaryPlan::Join { left, right, join_vars, output_vars }` or equivalent.
- A left-deep sequence representation for decomposed plans.
- A materialized-subplan placeholder for bushy right children, even if materialization execution is implemented later.
- A deterministic initial binary planner over typed IR atom order for now.
- A test-only or internal explicit binary plan injection API for paper examples.

## Technical Direction

- Do not depend on DuckDB or SQL.
- Build binary plans from normalized atom occurrences and equality-sharing variables.
- For self-joins, binary leaves must reference occurrence IDs, not relation names.
- Bushy decomposition follows the paper: a join node that is a right child becomes the root of a new left-deep subplan whose result is treated as an input to the parent plan.
- Initial deterministic planner may use atom order and simple relation cardinality, but the IR must not prevent future cost optimization.
- Binary plan validation must ensure every atom occurrence appears exactly once as a leaf.

## Non-Goals

- Do not convert binary plans to Free Join here.
- Do not execute binary plans here.
- Do not add materialized intermediate execution here beyond an IR placeholder.

## Acceptance Criteria

- Every normalized positive query can produce a deterministic binary plan over atom occurrences.
- Binary plan validation rejects missing leaves, duplicate leaves, unknown atom occurrences, and disconnected output variables.
- Left-deep plans are represented as ordered atom occurrence sequences.
- Bushy plans decompose into ordered left-deep subplans with explicit materialization boundaries.
- No production dependency on DuckDB, SQL parser, or SQL strings exists.

## Required Tests

- Two-atom join binary plan.
- Chain query left-deep plan.
- Star query deterministic plan.
- Triangle query deterministic plan.
- Self-join plan with two distinct occurrence leaves.
- Bushy plan decomposition for `(R join S) join (T join U)`.
- Invalid duplicate leaf rejection.
- Invalid missing leaf rejection.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb planner --all-features
cargo test --workspace --all-features
rg "duckdb|DuckDB|sql parser|SQL parser" crates/bumbledb-core crates/bumbledb-lmdb
```

The final `rg` must return no new production dependency or implementation path.
