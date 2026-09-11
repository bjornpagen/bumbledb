# BumbleDB 1.2.1

This patch fixes typed query composition and local history maintenance.
Core, log, and the three native packages all use version 1.2.1.

- Imported computed and aggregate columns preserve their field descriptors,
  numeric kinds, and carrier information across query stages. Generic variable
  projections retain precise result types. Generated schemas defer unknown
  carrier classes to runtime validation.
- Explicit scalar measures can use interval-width capacity bounds, and
  interval measures can use scalar bounds in the same application unit.
  Unweighted counts remain incompatible with interval-width bounds.
- Local histories produce independent, verified backups in the standard
  backup format. Repeating a completed backup resolves its original capture.
- Cold migrations open the source from the verified generated schema chain.
  Migrated targets use the normal local tenant layout, and activated retries
  return the target binding.
- Completed writable restore retries resolve the original native activation
  and backup evidence, preserving later writes. A different operation or
  source artifact cannot overwrite the existing target.

Node 24+ and Effect 4.0.0-rc.112 are required. The supported packages cover
macOS ARM64, Linux ARM64, and Linux x64; Linux uses the Amazon Linux 2023
glibc 2.34 baseline. Use pnpm to install and publish.

All three supported-platform builds and full correctness batteries must pass
on the release commit. Static Linux ARM64 and Miri remain supplemental checks,
outside the publication gate. See the [publishing procedure](../ts/PUBLISHING.md).

The [full benchmark results](perf/results.md) measured 1.1.0 source;
this release makes no new performance claim. Real-S3/IAM, Graviton performance,
and larger-than-memory workloads have not been qualified. Hosted generated
migration orchestration is unsupported through the TypeScript/native bridge.
