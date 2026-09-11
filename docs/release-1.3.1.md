# BumbleDB 1.3.1

BumbleDB 1.3.1 removes the migration programming language. Log emits independent
old/new schema bindings; applications write ordinary typechecked TypeScript
transformations using the same query readers, facts, and change batches as
the rest of the SDK.

**This release deliberately breaks the previous migration API and artifact
formats.** The generator, transformation AST/evaluator, plans, manifests,
prefix chains, repository manager, migration scalar namespace, and compatibility
exports are removed. Old migration records refuse explicitly. New transition
records have distinct tags and digest domains; older releases cannot read them.
Ordinary schema, command, receipt, and core storage identities retain their
existing meanings.

- `schemaSnapshot` emits native-verified canonical schema JSON. `schemaBindings`
  emits ordinary SDK declarations from one snapshot, without historical
  application modules. The CLI provides `snapshot` and `bindings` commands.
- `Transition` captures an exact frozen source, accepts sequential unpublished
  changes, judges the complete target once, and installs immutable ready
  content. Applications inspect and compare before explicitly activating.
  Retained contracts resolve interrupted operations; activated retries return
  original evidence even after subsequent target writes. Abort fences the
  target before thawing the source. Closing handles never decides an abort.
- Query heads compose by projection or fold semantics. Variables, computed
  scalars, and interval pieces can share a compatible projection column.
  Preparation handles every surviving arm; interval widths and class
  refinements survive only when all arms establish them. Existing query
  algorithms, arithmetic errors, aggregate meanings, and recursion restrictions
  remain in place.
- Field codecs expose the existing interpreter directly. Applications can
  validate and convert a field without inventing a one-field relation.
- Native failures preserve their context and exact size limits through Log.
  Expanded-law diagnostics include rejected/conflicting statement IDs and
  descriptors. Returned outcomes derive certainty from their variants;
  independent materialization health remains explicit.
- `HandleBusy` reports temporary contention. Closing is terminal for admission;
  finalization drains already-admitted population inputs and consumes aliases.
- Builds and packing select one immutable source and produce a coherent package
  family in private directories. A failed or overlapping build cannot remove
  another consumer's outputs. Notes initializes directly from its current schema
  snapshot and ordinary idempotent commands.

Correctness coverage includes generated-schema identity roundtrips, invalid
bindings, full final-law judgment, both parent/child batch orders, application
preservation checks, stale handles, cancellation, cold recovery, uncertain
installation, and activation/abort races. The full pinned 190-relation consumer
schema emits, typechecks, and recompiles to its identical native schema.

The [focused cutover measurements](perf/imperative-cutover-1.3.1.md) cover query
preparation/execution, bounded population, and memory at increasing sizes.
The full benchmark was not rerun; the [comprehensive benchmark report](perf/results.md)
continues to describe its measured 1.3.0 source. Transitions read and write their
data and judge/hash the final target. They add no transition bookkeeping to
ordinary queries or commands. Application transformation complexity remains
the application's responsibility.

Node 24+, Effect 4.0.0-rc.112, and pnpm are required. All five packages use
1.3.1: core, Log, macOS ARM64, Linux ARM64, and Linux x64. Linux targets Amazon
Linux 2023 / glibc 2.34. The TypeScript transition API supports local histories;
native hosted transitions use the existing conditional backend. Real-S3/IAM,
Graviton performance, and larger-than-memory qualification remain unestablished.
Applications own transformation, preservation checks, deployment adoption, and
backup policy.

Publication requires successful builds and complete correctness batteries for
the three supported platforms on the exact release commit, plus an annotated
Git tag and a GitHub Release containing five tarballs and `SHA256SUMS`.
Static Linux ARM64 and Miri are supplemental. See [PUBLISHING](../ts/PUBLISHING.md).
