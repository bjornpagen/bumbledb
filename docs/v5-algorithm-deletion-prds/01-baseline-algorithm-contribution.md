# PRD 01: Baseline Algorithm Contribution

## Goal

Create a data-driven baseline that identifies which execution algorithms are actually contributing value before any deletion.

This PRD must not change engine behavior. It establishes the evidence and deletion criteria for the v5 hard-deletion pass.

## Explicit Non-Goals

- No backwards compatibility.
- No migration work.
- No permanent feature flags.
- No permanent runtime disable switches.
- No algorithm deletion in this PRD.
- No relation-name-specific logic.

## Ordered Deletion Candidates

Evaluate in this order:

1. Mixed hash/LFTJ runtime.
2. Hash probe runtime.
3. Tiny project sink.

Do not start a later deletion before the earlier one has a committed keep/delete decision.

## Required Baseline Artifacts

Run and store:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-baseline-nonjob.json
```

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-baseline-job-10k.json
```

Optional targeted prepared-result q09 artifact:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --query job_q09_voice_us_actor \
  --cache-mode prepared-result \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-baseline-job-q09-prepared-result.json
```

## Required Documentation

Create:

```text
docs/benchmark-rca/v5-algorithm-contribution.md
```

Include:

- exact artifact paths
- exact commands
- current non-JOB wins/losses and gate failures
- current JOB wins/losses and gate failures
- per-query runtime family table
- queries using `Mixed`
- queries using `HashProbe`
- queries using `StaticEmpty`, `DirectKernel`, `IndexNestedLoop`, and `Lftj`
- q09/q16/q24 cache-mode behavior
- deletion decision criteria

## Deletion Decision Criteria

An algorithm may be hard-deleted only if all are true after deletion:

- `cargo test --workspace --all-features` passes.
- full non-JOB benchmark has zero gate failures.
- JOB 10k benchmark has zero gate failures.
- q09/q16/q24 still pass their cache-mode honesty gates.
- performance regressions are documented and accepted by the PRD if they are below existing gates.
- no permanent disable switch remains.

If deletion fails these criteria, revert only that deletion attempt, document the reason, and delete the PRD as a keep decision.

## Required Checks

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`

## Passing Requirements

- Baseline artifacts exist.
- RCA doc exists with runtime-family contribution tables.
- No engine behavior changes are made.
- Worktree is clean after commit.

## Completion Criteria

- Future deletion PRDs can decide from measured evidence instead of vibes.
- This PRD is deleted and committed after passing.
