# PRD 15: Seal Typed IR And Public Query Surface

## Status

Not started.

## Current State

Execution-boundary validation rejects malformed `TypedQuery` values, but `bumbledb-core/src/query_ir.rs` still exposes public fields on `TypedQuery`, variables, inputs, relation atoms, field bindings, comparisons, operands, and literals. External callers can forge invalid IR and depend on runtime rejection.

`QueryPlan` also remains public and includes physical `FreeJoinPlan` internals. That may be useful for tests today, but the final public API should not expose physical internals unless they are required diagnostics.

## Objective

Make the typed query builder/schema boundary the only normal way to construct executable queries, and reduce public query diagnostics to stable, necessary output.

## Implementation Steps

1. Make typed IR fields private or crate-private where Rust visibility allows.
2. Add read-only accessors needed by `bumbledb-lmdb` normalization without allowing invalid mutation.
3. Keep `QueryBuilder` as the public construction path.
4. Preserve execution-boundary validation as defense in depth.
5. Audit `QueryPlan`, `FreeJoinPlan`, `PlanNode`, and ID exports; remove public physical internals not required by benchmarks/tracing.
6. Update tests that manually forge invalid IR to use dedicated test-only constructors or crate-local helpers.
7. Verify external public API still supports normal query construction and execution.

## Passing Criteria

- External crates cannot construct invalid `TypedQuery` values by directly writing public fields.
- Runtime validation tests still cover malformed IR through test-only or internal constructors.
- Public query output exposes result facts and stable diagnostics, not mutable physical execution internals.
- Full validation passes.

## Failure Modes

- Relying only on execution validation while public fields remain forgeable is failure.
- Making LMDB internals public to normalize private IR is failure.
- Removing validation because construction is sealed is failure.

## Completion

Delete this PRD and commit.
