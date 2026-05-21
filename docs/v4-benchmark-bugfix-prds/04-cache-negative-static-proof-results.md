# PRD 04: Cache Negative Static Proof Results

## Goal

Avoid repeating the same failed static proof attempt across warmup and sample executions.

Even after PRD 03 gates proof execution, there will be some cases where proof runs and cannot prove emptiness. That negative result should be cached per snapshot/query/input so repeated prepared executions do not pay the same proof cost again.

## Explicit Non-Goals

- No backwards compatibility.
- No persistent cache across database opens.
- No cache shared across schema fingerprints or transaction IDs.
- No caching unsound partial proof data.
- No masking query result changes after writes.

## Required Cache Key

Key negative proof by:

```text
schema fingerprint
storage tx id
query shape key
encoded inputs
proof kind
```

Proof kinds:

```text
static_literal
static_semijoin
```

If static literal proof is already cheap enough, caching only static semijoin negatives is acceptable.

## Required Cache Value

Cache a compact value:

```rust
enum StaticProofCacheValue {
    ProvenEmpty,
    ProvenNotEmptyOrInconclusive,
}
```

Do not cache candidate sets unless a later PRD explicitly proves this is safe and useful.

## Required Behavior

- If cache says `ProvenEmpty`, return `StaticEmpty`.
- If cache says `Inconclusive`, skip proof and run normal planning/execution.
- Cache entries must be invalidated naturally by tx id changes.
- Cache must not affect different input bindings.

## Required Tests

- First execution of non-empty proof-eligible query records proof attempt.
- Second execution skips proof via negative cache.
- Write transaction changes tx id and causes proof to be considered again.
- Different input binding does not reuse negative result.
- Proven-empty cache still returns static empty.

## Required Benchmarks

Run non-JOB materialized and rows modes where relevant.

Expected:

- warmup/sample loops do not repeat expensive failed proof.
- static proof timings on second execution are zero or near-zero for cached-negative cases.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- non-JOB benchmark proves repeated sample blowups are gone.

## Completion Criteria

- Negative static proof attempts are cached safely.
- Repeated benchmark samples do not redo failed proof work.
- This PRD is deleted and committed after passing.
