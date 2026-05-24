# Free Join Paper Alignment PRD Suite

## Status

Drafted. This suite is the ordered implementation map for breaking Bumbledb into direct alignment with the Free Join paper while preserving the Rosetta Stone contract.

## Inputs

- Normative product contract: `docs/ROSETTA_STONE.md`.
- Local paper source: `docs/free-join-paper/arXiv-2301.10841v2/`.
- Historical investigator reports were deleted after this suite superseded them.

## Non-Negotiable Constraints

- Bumbledb is a strict Codd-style set engine.
- Relations are sets of full facts.
- Query solutions are sets of variable bindings.
- Projection returns duplicate-free result facts.
- SQL bag semantics are forbidden.
- SQL is allowed only inside benchmark reference oracles using `SELECT DISTINCT`.
- LMDB remains the only durable backend.
- The v5 storage rebuild must use real LMDB through the Rust `heed` binding, matching the pre-purge backend choice unless a later explicit decision replaces it.
- Application-level copy-on-write maps, in-memory shadow stores, or fake transaction layers are forbidden as substitutes for LMDB write transactions and MVCC read snapshots.
- Runtime DDL, server mode, network protocol, async API, nulls, floating-point persistence, non-serial generated IDs, and public aggregation remain out of scope.
- The paper's bag-semantics and DuckDB assumptions must be adapted, not copied.
- A future Logica-like language may lower into typed IR, but it must adapt to Rosetta set semantics rather than importing upstream Logica multiset/null/SQL assumptions.
- Public aggregation remains out of scope for this suite, but the Free Join executor must preserve a private sink/fold boundary so future Rosetta-compatible aggregate consumers can be added without rewriting the executor.
- Breaking storage and Rust API changes are allowed.
- Compatibility readers and in-place migrations remain forbidden. ETL into a new database is the migration path.

## Ordered PRDs

| Order | PRD | Purpose |
| --- | --- | --- |
| 00 | `00-suite-contract.md` | Locks vocabulary, invariants, and done criteria for the whole suite. |
| 01 | `01-paper-adaptation-and-public-language.md` | Removes misleading public language and adapts paper assumptions to Rosetta. |
| 02 | `02-query-normalization-and-atom-occurrences.md` | Defines formal atom occurrences, self-join aliases, field binding validation, and repeated-variable policy. |
| 03 | `03-formal-free-join-ir-and-validator.md` | Adds paper Free Join IR: subatoms, nodes, partitioning, covers, and validator. |
| 04 | `04-legacy-lftj-purge-verification.md` | Verifies the old singleton-variable LFTJ path stays deleted. |
| 05 | `05-binary-plan-ir-and-bushy-decomposition.md` | Adds internal binary plan IR and bushy-to-left-deep decomposition without SQL/DuckDB. |
| 06 | `06-binary2fj-and-factorization.md` | Implements paper `binary2fj` and conservative factoring as pure plan rewrites. |
| 07 | `07-storage-format-v5-columnar-set-layout.md` | Designs the new breaking durable layout for canonical set membership plus columnar base data. |
| 08 | `08-storage-v5-write-read-snapshot-semantics.md` | Implements v5 insert/delete/bulk load/snapshot semantics and atomicity. |
| 09 | `09-plan-scoped-base-images.md` | Replaces pre-plan query-image assumptions with plan-scoped immutable relation base images. |
| 10 | `10-encoded-tuples-and-ght-api.md` | Adds tuple keys and the paper GHT interface. |
| 11 | `11-colt-lazy-trie.md` | Implements execution-local COLT over relation base images. |
| 12 | `12-scalar-free-join-executor.md` | Implements the paper node/cover/probe recursive Free Join executor. |
| 13 | `13-dynamic-cover-selection.md` | Adds runtime cover choice by exact/estimated key count. |
| 14 | `14-vectorized-free-join-execution.md` | Adds `iter_batch`, batched probing, survivor compaction, and vectorized recursion. |
| 15 | `15-predicate-selection-and-range-pushdown.md` | Defines and implements selection adaptation, equality lowering, literals, inputs, and range pushdown. |
| 16 | `16-planner-statistics-and-plan-selection.md` | Builds a real plan selector using set semantics, Free Join shapes, and prefix-aware statistics. |
| 17 | `17-factorized-output-and-materialization.md` | Adds optional internal factorized output while preserving duplicate-free public result sets. |
| 18 | `18-explain-metrics-and-tracing.md` | Makes plan shape, covers, COLT, vectorization, and output mode observable. |
| 19 | `19-correctness-validation-suite.md` | Builds validator, differential, property, fuzz, and golden tests for paper-compliant Free Join. |
| 20 | `20-benchmark-suite-and-paper-ablations.md` | Adds paper-shaped benchmark coverage and correctness-first ablations. |
| 21 | `21-public-api-cutover-and-legacy-deletion.md` | Cuts over public/internal APIs and deletes stale LFTJ-only claims and old layout paths. |
| 22 | `22-final-paper-compliance-gate.md` | Final acceptance gate proving Rosetta and paper compliance. |

## Global Definition Of Done

Each PRD is complete only when all of these are true:

- The implementation preserves Rosetta set semantics.
- Storage/query behavior that claims durability, atomicity, or snapshot isolation is backed by `heed::Env`, `heed::RwTxn`, and `heed::RoTxn`, not by process-local simulation.
- No public API or documentation implies bag semantics, SQL support, or aggregation unless a later Rosetta update explicitly approves it.
- New behavior is covered by focused unit tests and at least one integration or differential test when execution semantics change.
- Public explain/diagnostics do not claim a paper feature unless that feature is represented and tested.
- Free Join execution work must not hardwire tuple-vector materialization as the only internal output path; projection/factorized output must flow through a private sink-like boundary.
- All acceptance commands listed in the PRD pass.
- `cargo fmt --all --check` passes.
- `cargo check --workspace --all-targets --all-features` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `cargo test --workspace --all-features` passes unless the PRD explicitly narrows the gate for an intermediate refactor.
- `cargo check --manifest-path fuzz/Cargo.toml` passes when fuzz or storage/query boundary types change.
- `bash scripts/check-line-counts.sh` passes.

## Completion Discipline

- Do PRDs in order unless a later PRD explicitly says it can run in parallel.
- If a PRD exposes that a prior PRD was incomplete, stop and repair the prior PRD first.
- If the current code cannot pass a PRD without a breaking change, make the breaking change. Do not add compatibility shims unless the PRD requires them.
- Do not add SQL, bag output, aggregation, runtime DDL, server functionality, or alternate storage engines to satisfy a paper benchmark.
- Do not add public aggregation while completing this suite; if an internal hook is needed, keep it private and prove it preserves current `QueryResultSet` semantics.
- Do not retain misleading names. If a type is LFTJ, call it LFTJ. If a type is formal paper Free Join, it must carry subatoms, partitions, and covers.
