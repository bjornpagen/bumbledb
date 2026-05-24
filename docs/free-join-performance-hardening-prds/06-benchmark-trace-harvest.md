# PRD 06: Benchmark Trace Harvest

## Purpose

Make JOB benchmark runs produce enough structured evidence to choose safe optimization targets.

## Required CLI

Extend `bumbledb-bench` with options equivalent to:

```text
--trace off|summary|full
--alloc off|on
--trace-output inline|file
--profile-query-label <label>
```

Names may differ, but the capability must exist.

## Required JSON Output

For each report, include:

- existing correctness fields;
- Bumbledb elapsed nanos;
- SQLite elapsed nanos;
- load nanos;
- result rows;
- trace enabled flag;
- allocation enabled flag;
- top spans by elapsed time;
- top spans by allocated bytes;
- aggregate counters;
- optional full span list when requested.

## Required Harvest Commands

The PRD is not complete until these commands run and their outputs are saved under a new ignored or documented location, such as `data/traces/`:

```bash
cargo run --release -p bumbledb-bench -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_q09_voice_us_actor --format json --repeats 3 --warmup 1 --trace full --alloc on
cargo run --release -p bumbledb-bench -- --preset job-sample --job-dir data/job --open-limit 100000 --query job_broad_cast_keyword_company --format json --repeats 3 --warmup 1 --trace full --alloc on
cargo run --release -p bumbledb-bench -- --preset job-sample --job-dir data/job --open-limit 100000 --format json --repeats 1 --warmup 1 --trace summary --alloc on
```

## Required Analysis Artifact

Create `docs/free-join-performance-hardening-prds/TRACE_BASELINE.md` containing:

- top 10 spans by time for q09;
- top 10 spans by allocated bytes for q09;
- top 10 spans by time for broad;
- top 10 spans by allocated bytes for broad;
- base-image rows/columns loaded per query;
- COLT offsets scanned per query;
- tuple materialization counts;
- clone/copy counts;
- a ranked optimization target list.

## Passing Criteria

- All three harvest commands pass exact SQLite comparison.
- `TRACE_BASELINE.md` exists and contains real numbers from the run.
- Benchmark JSON remains valid when `--trace off --alloc off`.
- Full trace output does not exceed reasonable size for one JOB query, or file output is mandatory.
- Global acceptance from PRD 00 passes.
