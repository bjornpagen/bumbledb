# Bumbledb 1.0

Bumbledb 1.0 is the production-ready release of the embedded core and local
durable-history layer: a set-semantic application database built on LMDB,
COLT and Free Join. It targets a database per application user or tenant.

## What ships

- Rust core, macros and query builders, distributed from the `v1.0.0` Git tag.
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

The production-ready scope is embedded core and **local** log use. Hosted
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

The full local benchmark completed on September 6, 2026: 32 read families,
34 scenario queries and all ordinary storage, lifecycle, mutation, scale,
churn, heap and Primer-shaped lanes. Results are from the Apple M2 Max on a
shared, scheduler-boosted host. The engine revision measured was
`3ed3303eb226de6c98955ab75c296887eaf7704b`, before the version/package/docs
cutover; raw reports correctly retain `0.20.3`.

The [performance report](perf/results.md) includes exact coverage, completed
controls, regressions, clock caveats and storage costs. There is no claim
that this release beats every historical query or has zero storage overhead.
Supplemental native86 profiling was stopped before capture when release
preparation was requested; older profiles are not relabeled as fresh ones.

## Distribution

The GitHub release and npm publication are separate steps. The release
carries the five verified, immutable npm tarballs, native/duty artifacts,
benchmark evidence and `SHA256SUMS`. The owner publishes the platform packages
first, then `@bjornpagen/bumbledb`, then `@bjornpagen/bumbledb-log`, all at
**1.0.0**. Until that command runs, a GitHub release does not imply npm
availability. The final publishing command verifies the downloaded hashes
before publication; it does not rebuild from a moving checkout.

The Rust workspace remains `publish = false`; use the Git tag rather than
assuming a crates.io release exists. No public Rust log SDK is introduced.
