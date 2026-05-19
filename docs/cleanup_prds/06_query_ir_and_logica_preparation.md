# PRD 06: Query IR And Logica Preparation

## Status

Draft. This PRD prepares for strict Logica but does not implement Logica.

## Problem

The current typed query IR lives inside `crates/bumbledb-core/src/datalog.rs` with parser/typechecker code for the custom Datalog-like language. The runtime consumes `TypedQuery`, not raw text, so the executor is not inherently tied to the surface language.

We need to separate language-neutral query IR from the custom parser before replacing the frontend.

## Goals

- Move reusable typed query IR out of `datalog.rs`.
- Rename Datalog-specific types to language-neutral names.
- Separate schema constraints from query predicates.
- Add typed expression scaffolding needed by future Logica constraints.
- Keep runtime semantics unchanged during the prep phase.
- Make deleting the custom parser straightforward later.

## Non-Goals

- No Logica parser implementation.
- No dual public frontend.
- No recursion.
- No negation.
- No disjunction.
- No arbitrary functions.
- No nullable values despite Logica having `null`.

## Current Code References

- `TypedQuery`, `TypedClause`, `TypedRelationAtom`, `TypedComparison`, and related types in `datalog.rs`.
- `NormalizedQuery`, `NormAtom`, `NormPredicate`, and `NormFindTerm` in `query.rs`.
- `OutputPlan`, `ProjectPlan`, `AggregatePlan`, and `AggregateTerm` in `free_join.rs`.
- `parse_and_typecheck` usage in tests and benchmarks.
- `ReferenceDb` in `crates/bumbledb-test-support/src/reference.rs`.

## Required Module Shape

Create a language-neutral module such as:

```text
crates/bumbledb-core/src/query_ir.rs
```

Move or introduce:

```rust
TypedQuery
TypedVariable
TypedInput
TypedOutputTerm
TypedClause
TypedRelationAtom
TypedFieldBinding
TypedTerm
TypedPredicate
TypedOperand
TypedLiteral
AggregateFunction
ComparisonOperator
```

`datalog.rs` may temporarily import and produce these types. Later it will be deleted.

## Query Predicate Model

Current comparison-only predicates are not enough for future Logica constraint lowering.

Add a forward-compatible model:

```rust
pub enum TypedPredicate {
    Comparison(TypedComparison),
    Boolean(TypedExpr),
}
```

Only `Comparison` needs execution initially. `Boolean` may be validated as unsupported until future work.

## Typed Expression Skeleton

Add a minimal typed expression model:

```rust
pub enum TypedExpr {
    Variable(usize),
    Input(usize),
    Literal(TypedLiteral),
    Unary { op, expr, value_type },
    Binary { op, left, right, value_type },
    Call { function, args, value_type },
}
```

This is prep only. Execution can reject everything except direct comparison lowering.

## Logica Constraints Context

Logica treats comparisons, unifications, boolean calls, and `Constraint(expr)` as query constraints/filters. This is not the same as database schema constraints.

Required naming:

- `SchemaConstraint` for primary/unique/FK/check declarations.
- `QueryPredicate` or `TypedPredicate` for query filters.

## Custom Datalog Boundary

During this PRD:

- Keep `parse_and_typecheck` working.
- Make it return language-neutral IR types.
- Move parser-specific AST and errors behind `datalog.rs` only.
- Ensure `bumbledb-lmdb` imports IR from `query_ir`, not `datalog`.

After this PRD, the executor should not import `bumbledb_core::datalog`.

## Implementation Plan

1. Add `query_ir.rs`.
2. Move shared enums/types from `datalog.rs` into `query_ir.rs`.
3. Update `datalog.rs` to use the moved types.
4. Update `query.rs`, `free_join.rs`, tests, benchmark harness, and reference evaluator imports.
5. Add typed expression scaffolding with explicit unsupported execution errors.
6. Keep current query behavior identical.
7. Update fuzz target naming only if necessary; full parser replacement comes later.

## Strict Passing Criteria

- `bumbledb-lmdb` no longer imports `bumbledb_core::datalog` in production modules.
- Runtime query execution depends only on language-neutral IR.
- The custom Datalog parser is isolated to frontend-only code.
- All existing query tests pass with identical behavior.
- Reference evaluator imports language-neutral IR.
- Benchmarks compile without executor depending on parser internals.
- Unsupported expression variants cannot panic the executor.

## Verification Commands

```sh
cargo test -p bumbledb-core datalog
cargo test -p bumbledb-lmdb query
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
```
