# BumbleDB 1.2.0

This release provides structural schema declarations, checked query
descriptions, inspectable change sets, and noncommitting admission judgment.
Core, log, and all three native packages use the same release version.
See [1.2.1](release-1.2.1.md) for the subsequent query and local-history fixes.

## Declarations and queries

Relations, closed rosters, selections, faces, keys, and constraints are checked
structural data. Equivalent declarations work across schemas, codecs, queries,
and writes. Explicit declared keys determine the exact fields accepted by
lookup. Schema admission retains structured diagnostics and the engine's
canonical fingerprint.

The typed query builder and `queryFromDescription` share validation, lowering,
and execution. `describeQuery` exposes logical ordinals and named results for
inspection and generation. Checked result-field descriptors infer returned
row types. Joins, negation, aggregates, computed expressions, intervals,
intermediate results, and linear recursive reachability use the same engine.
Generated parameters are checked against their uses at execution; their names
are not inferred statically. Parameterized query imports and imports of
recursive queries remain unsupported.

Mutable inputs are copied at their admission boundary. Compilation and query
inspection return detached byte buffers. Mutating inspection data cannot
change a compiled schema or query. Caches reuse only checked, deeply immutable
declarations; byte-bearing graphs do not qualify as immutable cache keys.

## Changes, judgment, and resources

Change sets expose requested distinct additions/removals, bounded record
inspection, canonical serialization, checked deserialization, and composition.
Composition uses exact-fact add-wins set semantics. It is commutative,
associative, and idempotent; distinct rows that conflict on a key remain present
for admission to reject. Requested counts differ from effective store changes.

`Db.judge` checks a change set without committing it. It returns the actual base
witness, moved-state information, effective additions/removals, and structured
violations. Preparation, judgment, and abort occur in one native operation.
Application still checks its own expected witness.

Scoped snapshots and preparations retain explicit lifetimes. Completed results
own their evaluation output; bounded cursors control delivery without claiming
streaming evaluation. Cancellation and closure release native ownership.

The log SDK retains its supported local history and migration functionality.
Its shipped executable uses Effect's Node runtime for process and signal
handling. Notes and packaged consumers exercise the same public APIs.

## Distribution and qualification

Node 24+ and Effect 4.0.0-rc.112 are required. The release family is the core
SDK, log SDK, and native packages for macOS ARM64, Linux ARM64, and Linux x64.
The Linux addons target the Amazon Linux 2023 glibc baseline. Native packages
publish before core, followed by log; only qualified, version-matched staged
tarballs may be published by the owner.

The full host battery and installed-tarball tests are separate from Linux and
static Linux ARM64 qualification. Old artifacts, unexecuted CI lanes, and
unpublished local substitutes are not release evidence. There is no new
benchmark claim. Existing real-S3/IAM, Graviton, and larger-than-memory
qualification limitations remain in force.

See the [SDK guide](../ts/README.md), [cookbook](../ts/COOKBOOK.md),
[log guide](../ts-log/README.md), and [publishing procedure](../ts/PUBLISHING.md).
