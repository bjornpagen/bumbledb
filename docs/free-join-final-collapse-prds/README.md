# Free Join Final Collapse PRD Suite

## Status

In progress. PRDs 01 through 09 and obsolete PRD 14 have been completed or removed. The remaining suite starts from the current `ec25f85` state: one Free Join execution path, no public plan-cost optimizer, but still too much explanatory plan surface, eager sorted-trie fallback, query-image/cache bulk, stale counters, public typed IR construction, and oversized modules.

## Purpose

This suite is the final ordered contract for reducing Bumbledb to the smallest useful embedded set database whose only query execution architecture is Free Join.

The previous work removed the obvious direct/hash/static sidecars, aggregate/cardinality/prepared query APIs, fake optimizer/cost surface, and single-variant node implementation indirection. The remaining work is not feature expansion. It is culling: delete explanatory plan structures that do not execute, delete eager atom materialization, delete sorted-trie caches, bound/minimize query images, seal public construction surfaces, and split the large modules only after behavior is smaller.

## Non-Negotiable Rules

- Relations are sets of full facts.
- Query results are sets of result facts.
- The only query execution algorithm is Free Join.
- LFTJ/sorted-leapfrog is an implementation technique inside Free Join, not a separate plan family.
- Lazy access/COLT-style iteration is the target access abstraction.
- No direct kernels, hash trie sidecars, static proof sidecars, alternate runtimes, or compatibility options may return.
- No prepared-query cache, cardinality-only API, aggregate API, or public diagnostics survives unless a PRD explicitly keeps it.
- No public plan-cost optimizer/candidate surface may return without a later feature PRD and a second real executable choice.
- No plan field may exist only for explain output. If runtime does not consume it, delete it.
- No benchmark field may preserve a deleted mechanic. Use actual surviving counters only.
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

After each PRD, Rust source and normative docs must have zero matches for concepts removed by that PRD and all prior completed PRDs. Future PRDs may name their own deletion targets until their PRD file is deleted.

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
SubAtom
free_join_subatom
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

Completed and deleted PRDs:

1. `01-delete-public-facade-crate.md`
2. `02-minimize-public-lmdb-api.md`
3. `03-seal-typed-ir-builder-boundary.md` as execution-boundary validation only; public typed IR sealing is reintroduced below because public fields still allow forged IR construction.
4. `04-delete-aggregate-query-surface.md`
5. `05-delete-cardinality-only-query-api.md`
6. `06-delete-prepared-query-and-plan-cache.md`
7. `07-collapse-query-plan-diagnostics.md`
8. `08-free-join-plan-execution-authority.md` partially; variable order now comes from `FreeJoinPlan`, but explanatory `SubAtom` plan data remains and must be deleted next.
9. `09-delete-node-implementation-indirection.md`
10. `10-delete-explanatory-free-join-plan-surface.md`
11. `11-replace-eager-sorted-trie-fallback.md`

Remaining ordered PRDs:

12. `12-delete-sorted-trie-cache-and-temp-builds.md`
13. `13-delete-stale-diagnostics-and-bench-mechanics.md`
14. `14-minimize-query-image-and-cache.md`
15. `15-seal-typed-ir-and-public-query-surface.md`
16. `16-minimize-storage-api.md`
17. `17-hard-module-split-gate.md`
18. `18-final-collapse-gate.md`

## Final Done Definition

- Workspace has no placeholder public facade.
- Public API exposes only embedded environment, schema, facts, values, set query execution, insert, delete, and required diagnostics.
- Public typed IR cannot bypass builder/schema validation.
- Aggregate query surface is gone unless reintroduced by a later explicit feature PRD.
- Cardinality-only and prepared-query APIs are gone.
- FreeJoinPlan contains only data consumed by execution.
- Lazy access replaces eager sorted-trie atom construction.
- Sorted trie cache and temporary atom relation builds are gone.
- No public plan-cost optimizer/candidate surface remains; variable-order scoring stays private.
- Query image is scoped, bounded, compact, and private where possible.
- Public typed IR cannot be forged outside builder/schema validation.
- Stale zero counters and benchmark columns for removed mechanics are gone.
- Large source files are split below the suite limit.
- Full validation and hygiene gates pass.
