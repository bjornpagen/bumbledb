# Bumbledb 1.0

Historical notes for 1.0/1.0.1. The current release notes are
[Bumbledb 1.1](release-1.1.md); the
[original benchmark report](https://github.com/bjornpagen/bumbledb/blob/v1.0.1/docs/perf/results.md)
retains the measurements quoted here.
These notes preserve the original release plan; the
[GitHub release record](https://github.com/bjornpagen/bumbledb/releases/tag/v1.0.1)
records the subsequent CI waiver and completed npm publication.

## 1.0.1 patch release

The patch release consolidates the post-1.0 cleanup. Public SDKs,
persisted formats, hash widths and the production scope below are unchanged.
All five npm packages and their exact dependency pins advance together to
1.0.1. Publication requires all exact-revision checks to pass.

- Reuse compiled cursor routes and physical source layouts, including
  positive and negated interval probes, when the join cover is unchanged.
- Borrow a forced trie for each shared-cursor probe batch; load or prefetch
  child data only when a current or later query consumer needs it.
- Share probe hashing and aggregate reductions, borrow executor scratch,
  and remove redundant native row-codec copies and operand paths.
- Preserve queued output reservations and admit the minimum input-byte
  cost before reserving a native row array from a supplied row count.
- Check word-map capacity arithmetic, narrow unsafe boundaries, preserve
  crash-test protocol buffering and child ownership, and limit compiler
  fixture searches to directories that actually contain libraries.

These changes passed correctness, allocation, cross-platform and Miri checks
before the complete local benchmark suite ran on September 7, 2026. All 15
benchmark lanes completed, and all 27 measured charts were refreshed. The
performance report embeds every repository image, with synthetic fixtures
clearly distinguished from measured evidence. No new profiles were captured.

## 1.0 baseline

Bumbledb 1.0 is the 1.0 release of the embedded core and local
durable-history layer: a set-semantic application database built on LMDB,
COLT and Free Join. It targets a database per application user or tenant.

## What ships

- Rust core, macros and query builders, distributed from the `v1.0.1` Git tag.
- Effect-only TypeScript core and log APIs, with synchronous pure authoring
  and lazy, scoped native database work.
- Native packages for Apple Silicon macOS, Linux ARM64 and Linux x64.
  Linux binaries are built in Amazon Linux 2023, with glibc 2.34 as the floor.
- Exact set semantics, final-state write admission, snapshot readers,
  reusable prepared queries, bounded work and RAM-first scratch spilling.
- Structural UUIDs, ordered UUID values, floats and float aggregates,
  intervals with float endpoints, joins, negation and reachability.
- Local named commands, retained outcomes, backup/restore and generated
  TypeScript schema migrations, using core primitives throughout.

## Breaking cutover

This is a clean break from the pre-1.0 APIs and formats. The C API, legacy
identity allocation, Promise/synchronous database twins and public Rust log
SDK are not supported. Generate identifiers in the application; Rust uses
`uuid::Uuid` and TypeScript uses structural UUID strings.

Do not open an older-format store expecting an automatic upgrade. The core
layout marker is **7**; layout counters were reset during development, so
the number alone is not a compatibility promise. Preserve the old program
and backup, export application data with the matching old version, and
validate a fresh 1.0 import. Generated log migrations evolve supported 1.0
schemas; they are not an adapter for arbitrary historical storage formats.

## Release scope and limits

The release scope is embedded core and **local** log use. Hosted
history APIs are present, but generated hosted initialization/migration is
not supported through the TypeScript/native bridge. Never migrate an S3
tenant's local cache as though it were the authoritative history.

Real-S3/IAM and Graviton qualification were explicitly deferred by the owner
for 1.0. These checks remain **not run**, not passed. Linux ARM64 CI is not a
Graviton performance run. Browser and Edge runtimes are unsupported; use
Node **24+**, the exact Effect **4.0.0-rc.112** peer, and the repository's
Rust **nightly-2026-08-15** toolchain.

LMDB supports stores larger than RAM, but populated larger-than-memory
performance was not qualified in this release round. Size application work
budgets and tenant caches for the actual deployment. The engine has one
serialized writer and concurrent snapshot readers; set semantics do not
make arbitrary concurrent writes conflict-free.

## Verification and performance

The release process verifies Rust, the native bridge, TypeScript, Lean
correspondence and isolated installed-package consumers on the release
revision, including the manually dispatched Miri workflow. CI links and
artifact hashes accompany the GitHub release. Skipped or deferred external
checks do not become successful evidence because their workflow is green.

The full local benchmark completed on September 7, 2026: 32 read families,
34 scenario queries and all ordinary storage, lifecycle, mutation, scale,
heap and Primer-shaped lanes. Results are from the Apple M2 Max on a
shared, scheduler-boosted host. The engine revision measured was
`5e83ee60c4e5d88e8ca395daa3de4d92a0c03186`, version **1.0.1**. Subsequent
documentation commits do not change that measurement identity.

The [original performance report](https://github.com/bjornpagen/bumbledb/blob/v1.0.1/docs/perf/results.md) includes exact coverage, comparison
with the last published run, regressions, clock caveats and storage costs.
The 100,000-row native result median was 63.86 ms versus 69.34 ms in the
previous full suite; triangle-join and aggregate-statistics medians fell
19% and 12%. Temporal-overlap and tenant-activation medians rose 29% and 30%.
These are single shared-host observations, not controlled causal estimates.
Compacted storage sizes are unchanged. There is no claim
that this release beats every historical query or has zero storage overhead.
Older reports and profiles retain their original identities; no new profiles
were collected in this release round.

## Distribution

The GitHub release and npm publication are separate steps. The release
carries the five verified, immutable npm tarballs, native/duty artifacts,
benchmark evidence and `SHA256SUMS`. The owner publishes the platform packages
first, then `@bjornpagen/bumbledb`, then `@bjornpagen/bumbledb-log`, all at
**1.0.1**. Until that command runs, a GitHub release does not imply npm
availability. The final publishing command verifies the downloaded hashes
before publication; it does not rebuild from a moving checkout.

The Rust workspace remains `publish = false`; use the Git tag rather than
assuming a crates.io release exists. No public Rust log SDK is introduced.
