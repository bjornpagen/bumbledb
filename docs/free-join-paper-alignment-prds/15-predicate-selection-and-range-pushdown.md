# PRD 15: Predicate, Selection, And Range Pushdown

## Purpose

Adapt the paper's pushed-selection assumption to Bumbledb's typed IR. Literals, inputs, same-atom equality, comparisons, and range predicates must be represented deliberately and pushed into sources when safe.

## Dependencies

- PRD 02.
- PRD 12.

## Scope

- Query normalization for selections.
- Same-atom repeated variable policy if not fully implemented in PRD 02.
- Literal/input equality filters.
- Residual comparisons.
- Range predicate pushdown into COLT/base image or optional accelerators.

## Required Semantics

- Equality literals and inputs attached to atom fields are source filters.
- Same-atom repeated variables are either already rejected or lowered into equality predicates over fields of the same fact.
- Cross-atom comparisons remain residual until all operands are bound.
- Range comparisons may push into source scans when the compared field and bound/literal/input value make it safe.
- String/bytes ordering remains unsupported unless Rosetta/persistent types explicitly define it. Equality remains supported through intern IDs.
- Pushdown must never change set output.

## Technical Direction

- Separate source filters from residual predicates in normalized query model.
- Add a `SourcePredicate` or equivalent for atom-local filters.
- For COLT, source filters can reduce initial offset vectors or filter during `force()`.
- For optional physical accelerators, range predicates may choose accelerator-backed offset vectors only when correctness does not depend on the accelerator.
- Preserve current encoded-comparison fast path where correct.
- Do not let range pushdown reintroduce SQL null or three-valued logic.

## Non-Goals

- Do not add aggregation.
- Do not add arbitrary expressions.
- Do not add SQL WHERE parsing.

## Acceptance Criteria

- Literal equality filters are enforced before or during source iteration/probing.
- Input equality filters are enforced before or during source iteration/probing.
- Same-atom repeated variables follow a documented and tested policy.
- Range filters on orderable fields produce the same results as residual evaluation.
- Non-orderable range comparisons are rejected as invalid queries.
- Pushdown and no-pushdown execution modes produce identical sets for test queries.

## Required Tests

- Literal equality filter.
- Runtime input equality filter.
- Same-atom repeated variable accepted-as-equality or rejected, per policy.
- Cross-atom comparison residual.
- Range `<`, `<=`, `>`, `>=` over `U64`, `I64`, and serial.
- Rejection for string/bytes range comparisons.
- Pushdown versus residual equivalence.
- Empty result after pushed selection.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-core query_builder --all-features
cargo test -p bumbledb-lmdb predicate --all-features
cargo test --workspace --all-features
```
