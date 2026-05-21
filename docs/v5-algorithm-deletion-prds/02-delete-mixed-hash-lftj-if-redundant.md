# PRD 02: Delete Mixed Hash/LFTJ If Redundant

## Goal

Hard-delete the mixed hash/LFTJ runtime if full validation proves it is redundant.

This is not a disable flag. The code should either be removed and committed, or explicitly kept with documented evidence.

## Explicit Non-Goals

- No backwards compatibility.
- No permanent disable flags.
- No environment-variable switches.
- No relation-name-specific planner behavior.
- No adding replacement algorithms.
- No changing storage semantics.

## Candidate Code Anchors

Inspect and delete if redundant:

```text
NodeImpl::Hybrid
QueryRuntimeKind::Mixed
PlanFamily::Mixed
execute_mixed_free_join
mixed_lftj_node_is_safe
MixedExecutor
hybrid optimizer candidate construction
mixed/hybrid tests that only preserve dead behavior
```

Exact names may differ after prior cleanup.

## Required Experiment

Hard-remove mixed runtime support in a working-tree patch.

Allowed replacement behavior:

- queries previously using mixed should fall back to pure LFTJ or hash probe if those remain valid
- planner traces should no longer select mixed/hybrid
- correctness must not change

Not allowed:

- leave a dormant mixed branch behind
- add a permanent feature flag
- silently skip queries

## Required Tests

- Existing tests pass, or tests specifically asserting mixed runtime are deleted/rewritten to assert pure LFTJ/hash fallback correctness.
- Add a regression test if a formerly mixed query shape must still return correct rows.
- Assert active explain/planner output no longer contains mixed/hybrid runtime selection if code is deleted.

## Required Benchmarks

Run after deletion attempt:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-no-mixed-nonjob.json
```

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-no-mixed-job-10k.json
```

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- non-JOB gates pass.
- JOB 10k gates pass.
- active Rust code no longer contains mixed runtime implementation if deletion succeeds.
- if deletion is rejected, RCA explains the exact failing queries and the code is restored before commit.

## Completion Criteria

- Mixed hash/LFTJ is either deleted or explicitly kept with measured evidence.
- No permanent disable switch remains.
- This PRD is deleted and committed after passing.
