# PRD 05: Direct Paths Before Expensive Proofs

## Goal

Run obvious direct execution paths before expensive static proof machinery when doing so is safe.

Direct/index-nested-loop paths are simple and predictable. Static semijoin proof is a speculative optimization. The engine should not spend hundreds of milliseconds trying to prove emptiness before executing a direct path that would finish in microseconds or milliseconds.

## Explicit Non-Goals

- No backwards compatibility.
- No restoring primary/ref/direct-chain legacy APIs.
- No relation-name-specific direct path ordering.
- No broad heuristic optimizer pass before direct kernels.
- No changing query semantics.

## Current Problem

`tag_lookup_join` is an obvious direct/index-nested-loop shape:

```text
PostingTag(tag = input, posting = ?posting)
Posting(id = ?posting, account = ?account)
```

The direct kernel executes in roughly milliseconds, but the benchmark reports hundreds of milliseconds because speculative work happens before direct execution.

## Required Ordering

For `execute_query` and `execute_prepared_query`, use this high-level order:

1. Validate/normalize/encode inputs.
2. Try direct storage project for single-relation direct scans.
3. Try direct index-nested-loop/direct chain kernels that do not need query image proof.
4. Use cheap static literal proof if applicable.
5. Use gated static semijoin proof if applicable.
6. Use direct count/factorized count if applicable.
7. Build full plan and execute LFTJ/hash/mixed.

If a direct path requires a query image, it may build one, but static semijoin must not run before an already-eligible direct path unless the query is a global count/static-empty candidate where proof is explicitly cheaper.

## Required Direct Eligibility Rules

Direct path should be considered before static semijoin when:

- query output is materialized projection
- direct chain/prefix/range kernel is structurally available
- no aggregate count proof is required
- no query image static proof has already been proven cheap by preflight

## Required Tests

- `tag_lookup_join` synthetic equivalent runs direct path before static semijoin.
- `chain4_from_a` synthetic equivalent runs direct path before static semijoin.
- Direct range query runs direct path before static semijoin.
- q24-like query still gets static semijoin proof because no better direct materialized path exists.
- q09-like global count still gets factorized count.

## Required Instrumentation Assertion

For direct path tests, assert:

```text
static_semijoin_proof_micros == 0
runtime_kind == DirectKernel or IndexNestedLoop
```

If timing field names differ, assert the corresponding proof counter is zero.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- non-JOB `tag_lookup_join` and `chain4_from_a` do not pay static semijoin proof cost.

## Completion Criteria

- Direct paths are preferred over speculative proof paths where appropriate.
- Non-JOB direct workloads recover.
- This PRD is deleted and committed after passing.
