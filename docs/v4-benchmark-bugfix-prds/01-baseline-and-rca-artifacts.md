# PRD 01: Baseline And RCA Artifacts

## Goal

Create a durable benchmark RCA baseline for the bugs observed after v3. This PRD should not change query execution behavior. It should make the current failure modes reproducible and explicit before fixes begin.

## Explicit Non-Goals

- No backwards compatibility.
- No migration work.
- No optimizer changes.
- No benchmark threshold relaxation.
- No hiding current regressions.

## Required Artifacts

Add a durable RCA note under a docs path such as:

```text
docs/benchmark-rca/v4-current-bugs.md
```

This file must include:

- The exact artifact paths used.
- The exact benchmark commands used.
- The non-JOB failure table.
- The JOB suspicious-fast table.
- The timing gap analysis.
- The hypothesis that static semijoin proof is over-eager and under-instrumented.
- The hypothesis that JOB samples include prepared result-cache hits.

## Required Benchmark Commands

Document and, if feasible, run:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v4-baseline-nonjob.json
```

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v4-baseline-job-10k.json
```

## Required RCA Content

Include this exact observation pattern for non-JOB:

```text
reported sample wall time is hundreds of milliseconds
engine subphase execute/sink/image/plan totals are only milliseconds
there is a large unaccounted gap inside execute_prepared_query
```

Include examples:

```text
tag_lookup_join: avg ~745ms, execute ~6.5ms
red_boat_sailors: avg ~631ms, execute ~13ms
revenue_by_customer_range: avg ~272ms, execute ~7ms
supplier_nation_orders: avg ~271ms, execute ~9ms
```

Include this exact observation pattern for JOB:

```text
q09/q16/q24 sample times are tiny because repeated samples hit prepared count/static caches
cold correctness execution still includes proof/precompute work
headline sample numbers must be labeled as cache-assisted
```

## Required Tests Or Checks

No new Rust tests are required. This is a baseline/documentation PRD.

Required checks:

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`

## Passing Requirements

- RCA doc exists and contains artifact paths.
- RCA doc explicitly names the three non-JOB gate failures.
- RCA doc explicitly names q09/q16/q24 cache-assisted behavior.
- Worktree is clean after commit.

## Completion Criteria

- The current bugs can be reproduced by a future agent without reading this conversation.
- This PRD is deleted and committed after passing.
