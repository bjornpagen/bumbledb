# Free Join Final Collapse PRD Suite

## Status

Not started.

## Purpose

This suite is the final ordered contract for reducing Bumbledb to the smallest useful embedded set database whose only query execution architecture is Free Join.

The previous suites removed the obvious direct/hash/static sidecars and moved the source toward fact-native storage and Free Join execution. This suite finishes the job by deleting optional public surfaces, duplicate query APIs, leftover caches, stale diagnostics, fake optimizer structure, eager trie fallback, and oversized module boundaries.

## Non-Negotiable Rules

- Relations are sets of full facts.
- Query results are sets of result facts.
- The only query execution algorithm is Free Join.
- LFTJ/sorted-leapfrog is an implementation technique inside Free Join, not a separate plan family.
- Lazy access/COLT-style iteration is the target access abstraction.
- No direct kernels, hash trie sidecars, static proof sidecars, alternate runtimes, or compatibility options may return.
- No prepared-query cache, cardinality-only API, aggregate API, or public diagnostics survives unless a PRD explicitly keeps it.
- Benchmarking and tracing stay.
- Storage, schema validation, fact insert/delete, and set scans required by tests/benchmarks stay.
- Completed PRD files are deleted after implementation.

## Validation Gate

Every PRD that changes Rust must pass:

```text
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
```

Every PRD that changes query behavior must additionally pass:

```text
cargo test -p bumbledb-test-support --test golden_examples --all-features
cargo test -p bumbledb-test-support --test property_and_differential --all-features
cargo test -p bumbledb-test-support --test sqlite_comparison --all-features
```

Benchmark renderer changes must also pass focused renderer tests:

```text
cargo test -p bumbledb-bench --bin bumbledb-bench renderer
```

## Global Source Hygiene Gate

After each PRD, Rust source and normative docs must have zero matches for removed concepts:

```text
DirectKernel
DirectChain
IndexNestedLoop
HashTrie
hash_trie
query_access
StaticProof
static_empty
static_semijoin
QueryExecutionOptions
QueryRuntimeKind
PlanFamily
runtime_kind
plan_family
CostKey
PlanCandidate
OptimizerTrace
PlanEstimates
VariableEstimate
NodeFactEstimate
chosen_plan
candidate_plan
free_join_estimates
iterator_ops
build_facts
FreeJoinLftj
pure_lftj
count-cache
count_cache
covering
segment
bag
tuple
row
```

The words `borrow` and Rust lifetime names are not part of this gate.

## Ordered PRDs

1. `01-delete-public-facade-crate.md`
2. `02-minimize-public-lmdb-api.md`
3. `03-seal-typed-ir-builder-boundary.md`
4. `04-delete-aggregate-query-surface.md`
5. `05-delete-cardinality-only-query-api.md`
6. `06-delete-prepared-query-and-plan-cache.md`
7. `07-collapse-query-plan-diagnostics.md`
8. `08-free-join-plan-execution-authority.md`
9. `09-delete-node-implementation-indirection.md`
10. `10-replace-eager-sorted-trie-fallback.md`
11. `11-delete-sorted-trie-cache-and-temp-builds.md`
12. `12-free-join-factoring.md`
13. `13-vectorized-free-join-batches.md`
14. `15-minimize-query-image.md`
15. `16-minimize-storage-api.md`
16. `17-hard-module-split-gate.md`
17. `18-final-collapse-gate.md`

## Final Done Definition

- Workspace has no placeholder public facade.
- Public API exposes only embedded environment, schema, facts, values, set query execution, insert, delete, and required diagnostics.
- Public typed IR cannot bypass builder/schema validation.
- Aggregate query surface is gone unless reintroduced by a later explicit feature PRD.
- Cardinality-only and prepared-query APIs are gone.
- FreeJoinPlan drives execution.
- Lazy access replaces eager sorted-trie atom construction.
- Sorted trie cache and temporary atom relation builds are gone.
- No public plan-cost optimizer/candidate surface remains; variable-order scoring stays private.
- Query image is scoped, compact, and private where possible.
- Large source files are split below the suite limit.
- Full validation and hygiene gates pass.
