# 1.2.1 full local benchmark plan

Status: prepared, not measured. Do not copy 1.1.0 results into this run or
relabel earlier binaries as 1.2.1. The final candidate must be committed,
clean, and pass all three supported-platform CI build/test jobs first.

The full runner covers 13 timed lanes: curves (S/M/L and warmth), reads,
scenarios, writes, storage (S/M), warm application performance, cold-open
and post-write application performance, large results, tenant churn,
hash probes, CRUD, lawful operations, and heap. Its setup also runs the
scorecard plan and verification; charts follow successful measurements.

Preview the roster without starting measurements, from the repository root:

```sh
scripts/bench-night.sh bench-out/release-1.2.1 --full --jobs 1 --plan
```

Before timing, finish builds/tests and stop other benchmark or implementation
work. Build and freeze the benchmark executable, retaining the source and
binary hash. Use fresh output and corpus directories, one timed lane at a
time, and the shared measurement lock. On this Apple Silicon host, macOS QoS
does not guarantee P-core-only placement; the command explicitly records
acceptance of that limitation.

```sh
test -z "$(git status --porcelain)" && cargo build --locked --release -p bumbledb-bench && mkdir bench-out/release-1.2.1-frozen && cp target/release/bumbledb-bench bench-out/release-1.2.1-frozen/ && git rev-parse HEAD > bench-out/release-1.2.1-frozen/SOURCE_REVISION && shasum -a 256 bench-out/release-1.2.1-frozen/bumbledb-bench > bench-out/release-1.2.1-frozen/SHA256SUMS
BUMBLEDB_BENCH_BIN="$PWD/bench-out/release-1.2.1-frozen/bumbledb-bench" BUMBLEDB_BENCH_DATA="$PWD/bench-data/release-1.2.1" scripts/bench-night.sh bench-out/release-1.2.1 --full --jobs 1 --shared --allow-macos-qos
```

Run both commands against the same unchanged candidate. Retain `MANIFEST.json`,
all lane JSON reports, logs, binary digests, and generated charts. Review failures
and regressions against 1.1.0 before refreshing `docs/perf/results.md` and
`assets/`; preserve the measured revision if later documentation commits follow.

Ordinary benchmark completion does not qualify real-S3/IAM, a populated
larger-than-memory workload, Graviton, or Linux x64 Node application timing.
The runner records these prerequisites separately. The correspondence tests
run in the correctness battery; the ignored three-way conformance lane can
be run separately with the command printed by the plan. Never mark missing
evidence as passed.
