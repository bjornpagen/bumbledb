# PRD 09: Final V6 Validation And Cleanup

## Goal

Validate the complete v6 mechanical performance pass, document final results, delete this PRD suite, and leave the repository clean.

## Explicit Non-Goals

- No backwards compatibility.
- No migrations.
- No permanent benchmark-only switches.
- No incomplete PRD files left behind.
- No unlabeled cache behavior.

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
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-final-nonjob.json
```

Run:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-final-job-10k.json
```

Run q09 prepared-result:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --query job_q09_voice_us_actor \
  --cache-mode prepared-result \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-final-job-q09-prepared-result.json
```

Run focused hotset if useful:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob \
  --query tag_lookup_join \
  --query red_boat_sailors \
  --query high_rating_red_boats \
  --query revenue_by_customer_range \
  --query supplier_nation_orders \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-final-hot-nonjob.json
```

## Required Final RCA

Create or update:

```text
docs/benchmark-rca/v6-final-mechanical-performance.md
```

Include:

- PRDs completed
- optimizations implemented
- optimizations rejected and why
- non-JOB before/after table
- JOB before/after table
- hotset before/after table
- mechanics counter deltas
- allocation/profile conclusions
- query latency conclusions
- ingest conclusions
- remaining known bottlenecks
- artifact paths
- validation command results
- compatibility statement: no backwards compatibility, no migrations

## Passing Requirements

- all final gates pass
- final benchmark artifacts exist
- final RCA exists
- non-JOB gates pass
- JOB 10k gates pass
- q09/q16/q24 cache behavior remains honest
- no empty PRD directories remain
- `git status --short` is clean after final commit

## Completion Criteria

- The retained database primitives are mechanically faster or clearly measured.
- Future optimization direction is data-driven from normal benchmark counters, not guesses.
- This PRD suite is deleted and committed.
