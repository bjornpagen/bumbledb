# Bumbledb 1.1.0

This release simplifies allocation and resource ownership across the Rust
engine and TypeScript SDKs. It includes API changes; upgrade the core, log and
native platform packages together.

## Changes since 1.0.1

- One-shot execution releases its compiled plan. Scoped preparations reuse
  their plan and buffers instead of compiling again on each call.
- Obsolete interned text is reclaimed incrementally. Live snapshots, query
  values and completed results retain the text they still own.
- Checkpoint recovery and hosted migration consume verified chunks lazily,
  releasing earlier payloads. Complete records borrow their input; split
  records reuse decoder carry storage.
- Row sealing borrows the existing ordered representation. Native result
  delivery writes into the final output without a second complete result
  representation. Scratch reads reuse buffers, and projection gathering
  uses bounded windows.
- Duplicate map/set inserts avoid unnecessary growth. Cancelled rehashing
  preserves the original map. Explicit query-memory release drops execution
  storage without invalidating live results or discarding preparation.
- The Rust/TypeScript APIs use ordinary allocation and OS mmap. Execution
  quota accounting, hidden execution deadlines and quota-triggered restarts
  are removed. Cancellation, input checks, scheduling admission and durable
  history policies retain their separate responsibilities.
- Fully static Linux ARM64 musl builds have a reproducible toolchain,
  empty-filesystem checks and full-system QEMU qualification. This does not
  establish Raspberry Pi performance.
- Native addon replacement on macOS stages a fresh file before renaming it.
  The packaged Notes example now undergoes a complete typecheck and production
  build as well as route tests.

## Updating applications

Remove obsolete per-operation execution-budget arguments and runtime byte/row
quotas. Ordinary TypeScript calls no longer need a policy object;
`NativeRuntime.layer()` has defaults. Queue/owner admission settings remain
available. Rust `WorkContext` carries cooperative cancellation, not an
allocation allowance. Cancellation is not a hard out-of-memory recovery system.

Use scoped preparation for repeated queries. Ordinary reset retains warm
capacity; `releaseMemory()` / `release_memory` releases execution storage.
Database cache clearing is separate. Retained results remain independently
owned, and paging a completed result is not streaming query execution.

See the current [TypeScript guide](../ts/README.md),
[log guide](../ts-log/README.md), [cookbook](cookbook.md), and
[compiled consumer examples](../examples/consumers/README.md).
Node 24+, Effect 4.0.0-rc.112 and Rust nightly-2026-08-15 remain the pins.
The native bridge is version-matched, not a semver-stable binary ABI.

## Benchmarks and verification

The [benchmark report](perf/results.md) contains the full 1.1.0 run, raw
evidence, all charts, and the historical comparison. The runner measures
one timed lane at a time, with verified user-interactive QoS on this M2 Max.
macOS does not provide hard P-core affinity.
Linux instead uses an explicit CPU mask and verified absolute nice -10.

Latency comparisons use serial lanes to avoid competing database workloads.
Shared-host results remain descriptive, not a causal speedup claim. Workload
sizes, samples, correctness gates and durability settings were not reduced.

Required release checks cover macOS, Linux ARM64, Linux x64 and static Linux
ARM64 musl. The [GitHub release](https://github.com/bjornpagen/bumbledb/releases/tag/v1.1.0)
records the final commit's CI and artifact hashes.
Scheduled Miri and externally gated checks are separate; a skipped job is not
passed evidence. Real S3/IAM deployment, Graviton performance, 512 MB Pi behavior
and populated larger-than-memory performance remain unqualified.

## Distribution

The GitHub release includes version-matched native artifacts, five npm
tarballs, checksums, and the benchmark evidence. Creating the GitHub release
does not publish to npm. Install/publish only the matching 1.1.0 packages;
do not mix the TypeScript SDK with an older native addon.

Rust is distributed from the `v1.1.0` Git tag, not crates.io. There is no public
Rust log SDK, C API, browser runtime or Edge runtime support.
