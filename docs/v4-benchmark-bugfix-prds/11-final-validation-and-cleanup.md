# PRD 11: Final Validation And Cleanup

## Goal

Run the final validation suite for the v4 benchmark bugfix pass, document final artifacts, delete this PRD suite, and leave the repository clean.

## Explicit Non-Goals

- No backwards compatibility.
- No migration validation.
- No accepting hidden timing gaps.
- No accepting unlabeled prepared result-cache benchmark numbers.
- No leaving completed PRD files behind.

## Required Final Gates

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
```

## Required Final Benchmarks

Run:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v4-final-nonjob.json
```

Run:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v4-final-job-10k.json
```

Run practical JOB default only if feasible. If skipped, state why.

## Required Final Summary

Report:

```text
Non-JOB wins/losses
Non-JOB gate failures, if any
JOB wins/losses
JOB q09/q16/q24 recompute vs cache behavior
Remaining known performance issues
Artifact paths
Validation command results
Compatibility statement: no backwards compatibility, no migrations
```

## Required Directory Cleanup

After all v4 PRDs complete:

- delete completed PRD files
- remove empty `docs/v4-benchmark-bugfix-prds`
- commit

## Passing Requirements

- all final gates pass
- benchmark artifacts exist
- no empty PRD directories remain
- `git status --short` is clean after final commit

## Completion Criteria

- The observed benchmark bugs are fixed or have precise follow-up tickets.
- The benchmark artifacts are honest and self-explanatory.
- The repository is clean and committed.
