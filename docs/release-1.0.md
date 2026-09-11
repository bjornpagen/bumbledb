# BumbleDB 1.0 / 1.0.1

BumbleDB 1.0 introduced the embedded core and local durable-history layer:
a set-semantic application database built on LMDB, COLT, and Free Join.
The [1.0.1 GitHub Release](https://github.com/bjornpagen/bumbledb/releases/tag/v1.0.1)
records publication and its artifacts.

## What shipped

- Rust core, macros, and query builders, distributed from Git.
- Effect TypeScript core and log SDKs, with synchronous schema/query
  construction and scoped native database operations.
- Native packages for Apple Silicon macOS, Linux ARM64, and Linux x64.
  Linux binaries use the Amazon Linux 2023 / glibc 2.34 baseline.
- Set semantics, final-state write admission, snapshot readers, reusable
  prepared queries, UUIDs, floats, intervals, joins, negation, and reachability.
- Local named commands, retained outcomes, backup/restore, and generated
  TypeScript schema migrations.

## 1.0.1 fixes

- Reuse compiled cursor routes and physical source layouts, including
  positive and negated interval probes, when the join cover is unchanged.
- Borrow a forced trie for each shared-cursor probe batch and load child
  data only when a query consumer needs it.
- Share probe hashing and aggregate reductions, borrow executor scratch,
  and remove redundant native row-codec copies.
- Check minimum input-byte cost before reserving a native row array, and
  check word-map capacity arithmetic.
- Narrow unsafe boundaries and preserve crash-test buffering and child ownership.

## Compatibility

The 1.0 APIs and formats replaced the pre-1.0 interfaces. Application code
owns identifier generation; Rust uses `uuid::Uuid` and TypeScript uses UUID
strings. There is no C API or public Rust log SDK.

Older-format stores require export with the matching old version and import
into a fresh 1.0 database. The core layout marker is 7; development resets mean
the number alone does not establish compatibility. Generated log migrations
operate on supported schemas, not arbitrary historical storage formats.

Node 24+, Effect `4.0.0-rc.112`, and Rust `nightly-2026-08-15` are the release
pins. Hosted generated migrations, browser runtimes, and Edge runtimes are
unsupported. Real-S3/IAM, Graviton, and populated larger-than-memory workloads
were not qualified for this release.

## Measurements

The September 7, 2026 local suite measured 1.0.1 source
`5e83ee60c4e5d88e8ca395daa3de4d92a0c03186` on a shared Apple M2 Max:
32 read families, 34 scenario queries, and the storage, lifecycle, mutation,
scale, and heap lanes. All 15 lanes completed.

The 100,000-row native result median was 63.86 ms versus 69.34 ms in the
previous suite. Triangle-join and aggregate-statistics medians fell 19% and
12%; temporal-overlap and tenant-activation medians rose 29% and 30%.
These are shared-host observations, not controlled causal estimates.
Compacted storage sizes were unchanged. No new profiles were captured.

The [original benchmark report](https://github.com/bjornpagen/bumbledb/blob/v1.0.1/docs/perf/results.md)
retains the measurements, comparisons, and limitations.
