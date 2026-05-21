# PRD 03: Delete Hash Probe If Redundant

## Goal

Hard-delete hash probe execution if direct kernels, factorized counts, static proof, and pure LFTJ cover the workload acceptably.

This PRD starts only after the mixed-runtime decision is committed.

## Explicit Non-Goals

- No backwards compatibility.
- No permanent disable flags.
- No new join algorithm.
- No relation-name-specific planner behavior.
- No deleting direct kernels, factorized count, static proof, or pure LFTJ.

## Candidate Code Anchors

Inspect and delete if redundant:

```text
NodeImpl::HashProbe
QueryRuntimeKind::HashProbe
PlanFamily::HashProbe
HashTrieIndex use from query execution
HashAtomIndexRequest
HashAtomIndex
execute_hash_probe
HashProbeExecutor
hash-probe optimizer candidate construction
hash-probe-specific counters/tests
```

Do not delete reusable hash trie data structures if still used by direct kernels or other retained code.

## Required Experiment

Hard-remove hash probe as an execution/planner family.

Allowed replacement behavior:

- affected joins fall back to pure LFTJ
- direct kernels remain available
- factorized counts remain available

Not allowed:

- leave hash probe as unreachable dead code
- hide failures behind a switch
- keep tests whose only purpose is preserving deleted runtime behavior

## Required Tests

- Existing correctness tests pass.
- Rewrite any hash-probe runtime tests to assert pure LFTJ fallback correctness if the query shape still matters.
- Add an assertion that planner/runtime no longer reports `HashProbe` if deletion succeeds.

## Required Benchmarks

Run after deletion attempt:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-no-hash-probe-nonjob.json
```

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-no-hash-probe-job-10k.json
```

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- non-JOB gates pass.
- JOB 10k gates pass.
- active Rust code no longer contains hash-probe execution/planner runtime if deletion succeeds.
- if deletion is rejected, RCA explains the exact failing queries and code is restored before commit.

## Completion Criteria

- Hash probe is either deleted or explicitly kept with measured evidence.
- No permanent disable switch remains.
- This PRD is deleted and committed after passing.
