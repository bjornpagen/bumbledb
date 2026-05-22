# Set-Native Rewrite PRD Roadmap

## Purpose

This directory is the ordered PRD suite for the hard set-native Bumbledb rewrite.

This is intentionally incompatible with the current storage format, schema model, public result API, query aggregate semantics, benchmark contract, and internal execution representation. Backwards compatibility is a non-goal. Migration is a non-goal. Compatibility shims are a non-goal. The acceptable upgrade path is ETL into a new database.

## Product Direction

Bumbledb becomes an embedded typed set engine over LMDB:

- Relations are sets of full tuples.
- The only logical mutations are `insert(tuple)` and `delete(tuple)`.
- Exact duplicate insert is an idempotent no-op.
- Exact absent delete is an idempotent no-op.
- Updates do not exist.
- DB-side ID allocation does not exist.
- SQL bag semantics do not exist.
- Query outputs are result sets, not row bags.
- Aggregate domains are explicit sets.

## Rewrite Thesis

The current engine says relations are sets, but much of the physical system still looks row-bag-shaped:

- Every access path is a full-row covering key.
- Query execution often enumerates witness bindings then deduplicates projected rows later.
- `Count(var)` takes a variable but currently counts emitted bindings.
- Count-only APIs return output row cardinality but are named like query result semantics.
- Benchmarks often compare only row counts, not aggregate values or exact result sets.
- Query images are dense `RowId` column snapshots with full index-entry copies.
- Segment publishing rebuilds full relation snapshots after touched relation writes.

The rewrite must remove these compromises instead of optimizing around them.

## Ordered PRDs

| Order | PRD | Purpose |
|---:|---|---|
| 01 | `01-hard-set-semantics-contract.md` | Freeze the exact logical semantics before storage or executor rewrites. |
| 02 | `02-golden-examples-and-correctness-contract.md` | Promote golden examples to non-regression fixtures with exact expected rows and aggregate values. |
| 03 | `03-schema-and-layout-break.md` | Delete covering-unique as a physical requirement and define set-native access layouts. |
| 04 | `04-storage-namespace-and-data-layout.md` | Replace full-covering row indexes with canonical tuple, unique, reverse-FK, and access namespaces. |
| 05 | `05-insert-delete-write-path.md` | Rebuild write semantics around insert/delete-only set deltas, constraints, and counters. |
| 06 | `06-remove-history-segments-and-rowid.md` | Delete full snapshot segment publishing and dense row-id architecture from current storage. |
| 07 | `07-query-image-access-substrate.md` | Build query images from set-native access streams and trie/cardinality structures. |
| 08 | `08-set-native-query-execution.md` | Stop enumerating hidden witnesses when projection/existence semantics suffice. |
| 09 | `09-explicit-aggregate-domains.md` | Replace ambiguous aggregate semantics with explicit set-domain aggregates. |
| 10 | `10-direct-and-factorized-kernels-rebuild.md` | Rebuild direct/factorized fast paths only where they are domain-correct. |
| 11 | `11-public-api-and-result-set-break.md` | Replace bag-shaped public output/count APIs with typed result sets and cardinality APIs. |
| 12 | `12-benchmark-and-measurement-contract.md` | Make benchmark correctness compare exact sets and aggregate values before timing. |
| 13 | `13-fuzz-crash-and-property-validation.md` | Add operation-sequence, query-equivalence, crash, and cache validation gates. |
| 14 | `14-docs-cleanup-and-final-cutover.md` | Rewrite normative docs and delete obsolete compatibility language after implementation. |

## Global Non-Negotiable Gates

Every PRD that changes behavior must pass these gates before completion:

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- Golden examples compile and run.
- Golden examples compare exact result sets and aggregate values.
- No new compatibility readers or storage migrations are added.

## Golden Example Families

The rewrite must preserve and enrich these examples:

- Ledger: holders, accounts, entries, postings, tags, balances, time ranges.
- Sailors: many-to-many reserve facts, duplicate projected witnesses, ranges, colors.
- Joinstress: chains, triangles, cyclic joins, count-domain traps.
- TPC-H subset: customer/order/lineitem/supplier joins and grouped revenue.
- IMDb/JOB: static proof, count, projection, and high-join workloads.
- Lahman: compound keys and year joins.
- LDBC subset: two-hop existential joins and social edge projections.

## Completion Definition

The rewrite is complete only when:

- The old storage format is rejected by version.
- The old covering-index physical model is deleted.
- The old row-id query-image model is no longer the semantic substrate.
- Ambiguous `Count(var)` semantics are deleted.
- Benchmarks validate values, not just row counts.
- All golden examples are richer than before and permanently gated.
- The new `ROSETTA_STONE.md` describes the implemented system, not the superseded one.
