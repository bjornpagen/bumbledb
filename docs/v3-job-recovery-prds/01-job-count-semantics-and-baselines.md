# PRD 01: JOB Count Semantics And Baselines

## Goal

Lock the benchmark semantics and regression baseline before changing optimizer code again.

The v2 cleanup intentionally changed aggregate semantics so global `count` over empty input returns one row containing `0`. SQLite benchmark SQL still had `HAVING COUNT(*) > 0` clauses in JOB count queries, which made SQLite return zero rows for empty global counts. That caused row mismatches in practical JOB runs.

This PRD settles the benchmark contract and records the baseline artifacts for q09/q24 recovery.

## Explicit Non-Goals

- No backwards compatibility with pre-v3 benchmark SQL semantics.
- No compatibility mode for old `HAVING COUNT(*) > 0` count behavior.
- No migration logic for old benchmark artifacts.
- No dual semantic path for empty global counts.
- No v1/v2 query behavior shims.

## Current Artifacts

Known current artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v2-cleanup-nonjob-release-scale10000-r30.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v2-cleanup-job-release-openlimit10000-r30.json
```

Known old comparison artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-results-latest-scale10000-r30.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-results-deep-latest-scale10000-r30.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/final-prd-job.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/final-prd-nonjob.json
```

## Required Code Updates

### Align JOB SQL Count Semantics

In `crates/bumbledb-bench/src/open.rs`, ensure every JOB global count SQL query returns one row even when the count is zero.

Remove `HAVING COUNT(*) > 0` from JOB count SQL queries.

Known current anchors before this PRD may include:

```text
job_broad_cast_keyword_company
job_broad_movie_info_star
job_q01_top_production
job_q09_voice_us_actor
job_q16_character_title_us
job_movie_link_bridge
job_q33_linked_series_companies
```

Do not change materialized projection queries unless they are semantically wrong.

### Document The Semantics

Add comments near JOB query definitions explaining:

```text
Bumbledb v3 follows Codd/set aggregate semantics: global count over empty input returns a single row containing 0. SQLite SQL must not use HAVING COUNT(*) > 0 for benchmark-equivalent global count queries.
```

## Required Baseline Script Or Notes

Add a small helper script or docs note with exact commands for current baseline runs.

Preferred location:

```text
scripts/bench-job-10k.sh
```

If adding a script is too much, add a short `docs` note in this PRD before deletion and copy it to `docs/ROSETTA_STONE.md` or benchmark comments.

Command:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json
```

## Required Baseline Table

Capture the current q09/q24 baseline in a committed doc comment or benchmark note if a durable markdown place is appropriate.

Current numbers from `v2-cleanup-job-release-openlimit10000-r30.json`:

| Query | Rows | BDB avg | SQLite avg | Runtime | Plan |
|---|---:|---:|---:|---|---|
| `job_q09_voice_us_actor` | `1` | `212223us` | `3450us` | `Lftj` | `pure_lftj` |
| `job_q24_voice_keyword_actor` | `0` | `176002us` | `9859us` | `Lftj` | `pure_lftj` |

Old target numbers from `bumbledb-job-results-latest-scale10000-r30.json`:

| Query | Rows | Old BDB avg | Old SQLite avg | Runtime | Plan |
|---|---:|---:|---:|---|---|
| `job_q09_voice_us_actor` | `1` | `903us` | `1960us` | `Lftj` | `aggregate_pushdown` |
| `job_q24_voice_keyword_actor` | `0` | `43us` | `9295us` | `Lftj` | `pure_lftj` with effective empty shortcut |

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- `cargo run -p bumbledb-bench --release -- --preset job --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb --open-limit 10000 --format json`
- No JOB row mismatch caused by empty global count semantics.

## Completion Criteria

- JOB SQL count queries match Bumbledb v3 count semantics.
- JOB 10k benchmark runs through all queries successfully.
- q09/q24 current baseline is recorded for subsequent PRDs.
- This PRD is deleted and committed after passing.
