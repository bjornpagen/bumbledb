# Benchmark round

This is the working runbook for the current code, not a record of measurements.
No new round has been run as part of the documentation cleanup.

## What to preserve

- Harness: `crates/bumbledb-bench/`.
- Runner: `scripts/bench-night.sh`; measurement lock: `scripts/measure.sh`.
- Reports: a fresh, ignored `bench-out/<round>/` directory for each run.
- Historical charts: `assets/`. They are not current-release evidence.
- Microarchitectural background: the separate `../bumblebench` repository.
  Its machine-specific conclusions are hypotheses to remeasure here.

Do not reuse an old output directory: the runner skips reports already present.

## Before timing

1. Finish and commit the candidate. Record its full revision and any dirty
   changes, Rust toolchain, CPU, OS, memory, page size, and power conditions.
2. Complete correctness checks separately. The runner also executes its
   verifier before timing and stops if setup or verification fails.
3. Wait for builds, tests, indexing, and other CPU-heavy jobs to finish.
   Observe CPU activity repeatedly, not one instantaneous sample. Keep power
   and thermal conditions stable. A measurement lock does not prove idleness.
4. Preview without timing or compiling:

   ```sh
   scripts/bench-night.sh bench-out/next-round --plan
   ```

5. Use a new output directory for the quiet-host run:

   ```sh
   scripts/bench-night.sh bench-out/quiet-round-YYYYMMDD-HHMM
   ```

The runner builds the ordinary release benchmark once. Instrumented builds
belong in separate diagnostic runs, not the timed baseline. Do not use
`--shared` or scheduler boosting to qualify the quiet-host baseline.

## Local runner coverage

| Lane | Question |
|---|---|
| Verification | Do independent answers agree before we measure? |
| Storage, S and M | What do actual files/pages cost against indexed SQLite? |
| Warm | What is the reusable prepared-query steady state? |
| Cold-open and post-write | What does the first useful read cost? |
| Large-result | What happens when output construction and delivery dominate? |
| Tenant churn | How do cache misses, reopening, and retained state behave? |
| Hash probe | What do short keys and large buffers cost? |

This is the default local round, not every exploratory CLI experiment.
Use `target/release/bumbledb-bench help` for focused follow-ups and larger
scales. Run measurements serially. The runner's manifest lists unavailable
external prerequisites separately; local success does not turn those green.

## External and missing evidence

- Real S3/IAM publication, contention, and recovery require their actual backend.
- Graviton and Linux x64 runtime costs require those machines. GitHub ARM
  correctness coverage is not automatically Graviton performance evidence.
- Larger-than-memory qualification requires genuinely populated data and a
  controlled memory envelope—not a large sparse map.
- Hash known-answer coverage requires the probe's `--kat` input. The default
  night command does not supply it; retain its NotRun status.
- Native, bridge, Effect, and whole-application costs must be measured at their
  own boundaries. Do not infer SDK overhead from a Rust-only benchmark.

The machine-readable release inventory remains in
`.config/obligation-inventory.json`. This runbook does not waive its gates.

## Comparisons and reporting

Use the same data, queries, durability, constraints, and index contracts for
both engines; disclose differences rather than burying them in a ratio.
Separate durable from no-sync writes. Separate resident working sets from
page-fault-heavy runs.

Run repeated baseline and candidate trials on the same quiet machine, rotating
their order. Keep baseline/baseline controls, raw samples, tails, failures,
timeouts, and ambient conditions. Never select only the fastest trial.

For storage, report logical data, index overhead, LMDB pages, apparent file
length, and allocated disk blocks separately. Compare equally compacted
databases; do not call a virtual map size disk usage.

Only replace README claims or charts after reviewing a complete round.
A missed target is a result, not permission to change the workload or budget.
