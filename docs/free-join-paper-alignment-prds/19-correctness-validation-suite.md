# PRD 19: Correctness Validation Suite

## Purpose

Build the comprehensive correctness suite required to trust the new paper-compliant engine. This PRD turns plan, storage, COLT, executor, vectorization, and set semantics into permanent regression fixtures.

## Dependencies

- PRD 12.
- PRD 14.
- PRD 15.
- PRD 17.
- PRD 18.

## Scope

- Recreate `crates/bumbledb-test-support`.
- Unit tests.
- Golden tests.
- Property tests.
- Differential tests.
- Fuzz targets.
- Validation scripts.

## Required Test Families

- Formal Free Join validator tests.
- Binary2FJ and factorization golden tests.
- GHT and COLT unit tests.
- Scalar versus vectorized execution equivalence.
- Static versus dynamic cover equivalence.
- Materialized versus factorized output equivalence.
- Scalar/vectorized/factorized sink-boundary equivalence for the current projection result-set sink.
- Reference evaluator differential tests.
- SQLite `SELECT DISTINCT` exact-value tests.
- Storage operation sequence tests under v5.
- Failpoint atomicity tests across v5 namespaces.
- Real LMDB transaction, reopen, and MVCC snapshot tests using temporary directories.
- Query validation tests for malformed IR.
- Fuzz tests for plans, queries, COLT, and vectorized/scalar equivalence.

## Required Differential Coverage

- Random small schemas with bool, integer, enum, serial, string, and bytes fields.
- Random set-valued facts.
- Positive conjunctive queries with projections.
- Self-joins through atom occurrences.
- Literals and runtime inputs.
- Omitted fields and wildcards.
- Comparisons and range predicates on orderable fields.
- Queries with no useful optional accelerator.
- Duplicate witnesses.
- Empty result sets.
- Fixtures where a test-only full-binding sink observes more bindings than the public projection cardinality, proving the executor seam preserves information needed by future aggregation while public output remains set-projected.

## Technical Direction

- Extend `ReferenceDb` to understand the same normalized query semantics chosen in PRD 15.
- Reference databases are test oracles only. They must never replace LMDB as the Bumbledb storage engine under test.
- Generate SQLite only for small schemas where exact `SELECT DISTINCT` SQL can be generated safely.
- Add execution mode matrix tests: scalar/vectorized, static/dynamic cover, materialized/factorized, singleton/binary/factored plan where applicable.
- Keep slow exhaustive tests separated from fast unit tests if needed, but ensure CI or standard validation can run a representative subset.
- Fuzz targets should abort on mismatches against a reference, never silently return unless the input is malformed and rejected by both paths.

## Non-Goals

- Do not benchmark performance here except to ensure tests terminate.
- Do not add SQL as a product API.

## Acceptance Criteria

- Every new formal plan invariant has a positive and negative test.
- Every execution mode returns identical result sets for shared query support.
- Differential tests cover self-joins, duplicate witnesses, and empty sets.
- Internal sink tests prove projection deduplication is a sink behavior, not a limitation of Free Join traversal.
- Fuzz crate includes at least one query/executor differential fuzz target.
- Failpoint tests cover v5 canonical, live row, column, guard, stats, and commit stages.
- Storage tests prove behavior survives process-local cache loss by reopening the LMDB environment from disk.
- Existing golden families still pass.
- New paper examples are permanent fixtures.

## Required Tests

- Clover paper fixture.
- Sand-dollar or clover-skew fixture.
- Triangle fixture.
- Chain fixture.
- Star fixture.
- Self-join fixture.
- Empty-result fixture.
- No-useful-index fixture.
- Duplicate-witness projection fixture.
- Full-binding sink versus projection-sink fixture.

## Validation Commands

```text
cargo fmt --all --check
cargo test --workspace --all-features
cargo test -p bumbledb-test-support --test golden_examples --all-features
cargo test -p bumbledb-test-support --test property_and_differential --all-features
cargo test -p bumbledb-test-support --test sqlite_comparison --all-features
cargo check --manifest-path fuzz/Cargo.toml
```
