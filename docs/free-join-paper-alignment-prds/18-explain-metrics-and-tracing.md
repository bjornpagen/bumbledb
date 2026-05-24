# PRD 18: Explain, Metrics, And Tracing

## Purpose

Make the physical plan and runtime behavior auditable. If Bumbledb claims paper Free Join, explain output and counters must prove subatoms, covers, GHT/COLT use, vectorization, and output mode.

## Dependencies

- PRD 03.
- PRD 11.
- PRD 13.
- PRD 14.
- PRD 17.

## Scope

- `QueryPlan::explain`.
- Query counters and timings.
- Benchmark JSON/Markdown renderers.
- Trace spans.
- Golden explain snippets.

## Required Plan Metrics

- Number of Free Join nodes.
- Number of subatoms.
- Atom partition coverage.
- Cover candidates per node.
- Binary plan source and left-deep decomposition, when applicable.
- Factorization attempts and moves.
- Plan mode: singleton, binary-derived, factored, injected, or other.

## Required Runtime Metrics

- Node entries.
- Cover choices.
- Exact cover key counts and estimates.
- Probe calls, misses, survivors.
- COLT nodes created and forced.
- COLT offset vectors scanned.
- COLT hash maps built.
- Batch size, batch count, input tuples, survivor tuples, failed tuples.
- Projection duplicate witnesses.
- Factorized output logical facts, materialized facts, and expansions saved.
- Scalar fast-path counters only when a formal singleton-plan fast path is actually used.

## Required Explain Output

Explain must include:

- Query execution mode.
- Formal Free Join plan before and after factorization when applicable.
- Node list with subatoms and atom occurrence IDs.
- Available and new variables per node.
- Cover candidates per node and chosen cover policy.
- GHT schema per atom occurrence.
- Source kind: COLT, optional accelerator, formal singleton fast path, or other.
- Vectorized batch size or scalar mode.
- Output mode: materialized set or internal factorized.
- Sink mode: projection result-set sink, internal factorized sink, or other private non-aggregate sink names that exist at that PRD.
- Query-image/base-image cache diagnostics.
- Timings and allocation stats.

## Technical Direction

- Prefer stable machine-readable JSON fields for benchmarks.
- Human explain text may change, but tests should cover key fragments.
- Do not print raw user data values except where existing explain already does so safely.
- Keep dead counters out of explain. Remove or increment counters like stale `trie_intersections`.
- Trace spans must align with real phases: plan validation, binary2fj, factorization, base image build, COLT force, cover choice, vectorized batch probe, sink materialization, benchmark correctness.

## Non-Goals

- Do not make explain output a stable public API unless explicitly documented.
- Do not expose query images, COLT nodes, or fact handles as public data structures.
- Do not claim public aggregation support merely because an internal sink/fold seam exists.

## Acceptance Criteria

- Explain no longer contains misleading singleton `free_join_node bind_vars` output for formal Free Join mode.
- Free Join explain shows subatoms, partitions, covers, and source schemas.
- Any future Generic Join-like mode is labeled as a formal singleton-subatom Free Join plan.
- Benchmark JSON includes plan mode, batch size, cover mode, output mode, and new counters.
- Markdown renderer includes core Free Join/COLT/vectorization counters without stale fields.
- Trace summarization script uses only surviving JSON fields.
- Explain distinguishes public result-set semantics from private sink mechanics and contains no aggregate claims unless a later Rosetta update adds them.

## Required Tests

- Explain golden for singleton mode.
- Explain golden for binary-derived Free Join.
- Explain golden for factored Free Join.
- Explain golden for dynamic cover selection.
- Benchmark renderer tests for JSON and Markdown fields.
- Trace summarizer test or script dry run if practical.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb explain --all-features
cargo test --workspace --all-features
```
