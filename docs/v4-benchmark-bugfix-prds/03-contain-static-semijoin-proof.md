# PRD 03: Contain Static Semijoin Proof

## Goal

Stop static semijoin proof from running broadly on non-empty materialized workloads.

The current static proof is too eager. It helps JOB q16/q24, but it appears to poison non-JOB materialized queries where it does expensive propagation, fails to prove emptiness, and then normal execution still runs.

This PRD must make static semijoin proof deterministic, cheap, and opt-in by query shape and exact preflight conditions.

## Explicit Non-Goals

- No backwards compatibility for old static proof behavior.
- No preserving broad heuristic propagation.
- No relation-name-specific allowlists or blocklists.
- No hiding failed proof costs.
- No disabling the q16/q24 winning proof paths.

## Current Code Anchors

- `static_query_proves_empty`
- `static_literal_atoms_prove_empty`
- `static_semijoin_proves_empty`
- `enumerate_static_atom_candidates`
- `static_semijoin_prefixes`
- `PlanCounters::static_semijoin_*`

## Required Shape Gate

Before running static semijoin proof, run a cheap preflight.

Static semijoin proof may run only when all are true:

- Query has at least one static literal/input/range predicate.
- Query has at least two relation atoms.
- At least one seed atom can produce an exact candidate set using a leading access-path prefix or a bounded range index.
- The exact seed candidate count is under a small threshold.
- The output is either global count or projection with no more than a configured output variable count.

Recommended thresholds:

```rust
STATIC_SEMIJOIN_MAX_SEED_CANDIDATES = 1024
STATIC_SEMIJOIN_MAX_PREFIX_PROBES = 2048
STATIC_SEMIJOIN_MAX_ROUNDS = 4
```

Tune thresholds by correctness and benchmark evidence, not guesswork.

## Required No-Broad-Scan Rule

Static semijoin proof must not full-scan large relations as a seed.

Allowed:

- exact leading prefix lookup
- exact bounded range lookup where an access path exists
- full scan only for tiny relations below a low threshold, e.g. `<= 256 rows`

Not allowed:

- scanning 10k or 100k rows to maybe build a candidate set
- multiplying candidate prefixes into tens of thousands of probes
- doing repeated failed propagation every benchmark sample

## Required Fallback Behavior

If preflight fails, skip static semijoin proof immediately and record:

```text
static_semijoin_skipped_reason
```

This can be a string in explain output or an enum/counter. It must be visible enough for tests.

## Required Tests

Add tests for:

- q24-like small exact seed still runs and proves empty.
- q16-like range seed still runs and proves empty.
- non-empty red-boat-like query with broad color/static candidate skips semijoin proof.
- tag-lookup-like direct chain query skips semijoin proof.
- tpch-like non-empty materialized query skips semijoin proof.
- skipped proof does not change query result.

## Required Benchmarks

Run non-JOB after this PRD.

Expected:

- `tag_lookup_join` returns near pre-proof times, not hundreds of ms.
- `red_boat_sailors` no longer pays hundreds of ms in static proof.
- `supplier_nation_orders` no longer pays hundreds of ms in static proof.
- JOB q16/q24 remain `StaticEmpty` and under gate thresholds.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- JOB 10k q16/q24 still pass gates.
- non-JOB no longer has static semijoin proof time dominating failed queries.

## Completion Criteria

- Static semijoin proof is a small deterministic rule, not a broad heuristic engine.
- Failed proof paths are cheap and visible.
- This PRD is deleted and committed after passing.
