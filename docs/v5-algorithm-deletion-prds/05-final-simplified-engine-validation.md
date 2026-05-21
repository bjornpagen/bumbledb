# PRD 05: Final Simplified Engine Validation

## Goal

Run final validation after all v5 deletion decisions, document final artifacts, delete the PRD suite, and leave the repository clean.

## Explicit Non-Goals

- No backwards compatibility.
- No migrations.
- No permanent algorithm disable flags.
- No keeping completed PRDs.
- No unlabeled benchmark cache behavior.

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
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-final-nonjob.json
```

Run:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-final-job-10k.json
```

Run targeted prepared-result q09 if feasible:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --query job_q09_voice_us_actor \
  --cache-mode prepared-result \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-final-job-q09-prepared-result.json
```

## Required Final Summary

Report:

```text
Which candidates were deleted
Which candidates were kept and why
Non-JOB wins/losses and gate failures
JOB wins/losses and gate failures
q09/q16/q24 cache-mode behavior
Artifact paths
Validation command results
Compatibility statement: no backwards compatibility, no migrations, no permanent disable flags
```

## Required Cleanup

- delete completed PRD files
- remove empty `docs/v5-algorithm-deletion-prds`
- commit

## Passing Requirements

- all final gates pass
- final benchmark artifacts exist
- no empty PRD directory remains
- `git status --short` is clean after final commit

## Completion Criteria

- Engine algorithm surface is simpler or defended by measured evidence.
- All retained algorithms have a clear reason to exist.
- Repository is clean and committed.
