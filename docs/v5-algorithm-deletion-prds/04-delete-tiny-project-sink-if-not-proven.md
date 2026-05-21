# PRD 04: Delete Tiny Project Sink If Not Proven

## Goal

Hard-delete the tiny project sink specialization if benchmarks do not prove it is worth the extra output-sink complexity.

This is not a join algorithm, but it is a specialized materialization path that competes with the generic encoded project sink.

## Explicit Non-Goals

- No backwards compatibility.
- No permanent sink-selection flags.
- No benchmark-only output shortcuts.
- No changing set semantics or projection dedup behavior.
- No changing public row output semantics.

## Candidate Code Anchors

Inspect and delete if redundant:

```text
TINY_PROJECT_THRESHOLD
TinyProjectSink
OutputSink::TinyProject
is_tiny_project_candidate
tiny-project-specific tests
```

## Required Experiment

Hard-remove tiny project sink so projections use the generic encoded project sink.

Allowed replacement behavior:

- all projections still deduplicate correctly
- output decoding remains final-boundary only

Not allowed:

- adding a permanent sink mode flag
- changing result order assumptions beyond existing unspecified order
- changing set semantics

## Required Tests

- Projection dedup still works.
- Direct materialized projection still does not use prepared result cache.
- Materialized output counters remain correct.
- Add/update tests if removing tiny sink exposes missing coverage.

## Required Benchmarks

Run after deletion attempt:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-no-tiny-project-nonjob.json
```

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-no-tiny-project-job-10k.json
```

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- non-JOB gates pass.
- JOB 10k gates pass.
- active Rust code no longer contains tiny project sink implementation if deletion succeeds.
- if deletion is rejected, RCA explains the exact failing queries and code is restored before commit.

## Completion Criteria

- Tiny project sink is either deleted or explicitly kept with measured evidence.
- No permanent disable switch remains.
- This PRD is deleted and committed after passing.
