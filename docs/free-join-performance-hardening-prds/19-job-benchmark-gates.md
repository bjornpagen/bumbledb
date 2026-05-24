# PRD 19: JOB Benchmark Gates

## Purpose

Make JOB benchmark correctness, trace, allocation, and performance gates strict enough to guide optimization safely.

## Required Queries

The gate must include at least:

- `job_q09_voice_us_actor`;
- `job_broad_cast_keyword_company`;
- all current eight JOB sample queries.

## Required Correctness

- SQLite reference must use exact `SELECT DISTINCT`.
- Compare exact result values, not counts.
- Keep correctness fingerprint.
- Fail benchmark if any result differs.

## Required Metrics

Each JOB report must include:

- Bumbledb elapsed nanos;
- SQLite elapsed nanos;
- load nanos;
- result rows;
- trace summary;
- allocation summary;
- top phase by elapsed time;
- top phase by allocated bytes;
- base-image rows loaded;
- source filter survivors;
- COLT offsets scanned;
- probe calls and misses;
- sink decode count.

## Required Performance Gates

Initial gates should be non-regression gates based on current measured baselines. Tighten only after optimization PRDs land.

Required first gates:

- q09 warm p50 must not exceed the current baseline by more than 25% unless trace overhead is enabled and labeled.
- broad warm p50 must not exceed the current baseline by more than 25% unless trace overhead is enabled and labeled.
- correctness must never be waived.

## Passing Criteria

- A single command can run all JOB gates.
- The command exits non-zero on exact value mismatch.
- The command exits non-zero on performance regression beyond configured budget.
- The command records trace and allocation summaries.
- Global acceptance from PRD 00 passes.
