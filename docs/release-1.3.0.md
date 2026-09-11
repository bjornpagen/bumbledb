# BumbleDB 1.3.0

BumbleDB 1.3 adds exact integer quotients, derived interval rows, interval
measurement, and exhaustive schema alternatives. Core, log, and all three
native packages use version 1.3.0.

- `Compute.mulDiv(a, b, divisor, rounding)` uses an exact wide product and
  checks the final 64-bit result after rounding. The divisor must be positive;
  choose `towardZero`, `nearestTiesAwayFromZero`, or `nearestTiesToEven`.
- Query `intersection` and `difference` emit ordinary nonempty interval rows.
  They compose through existing stages with Allen predicates, joins, negation,
  `pack`, measurement, and aggregation. Multiple interval outputs form Cartesian
  combinations. Empty output removes the binding before scalar evaluation.
- `Compute.measure` returns exact `u64` widths for bounded integer intervals
  and once-rounded native `f64` lengths for dense intervals. Unbounded inputs
  and finite overflow have distinct errors. Clip rays before measuring them.
- `alternatives` expands declared scalar keys and a closed discriminator into
  ordinary containment and mirrors laws. It adds no native statement kind or
  admission pass; payloads remain ordinary relations with their own types.
- Imported and exported query descriptions preserve computed and interval
  field types. Clipping and packing correctly discard fixed-width refinements.
  Public structural descriptors are the same algebra used internally.
- Interval expansion respects cancellation before evaluating later bindings.
  Exact arithmetic, generated composition graphs, endpoint cases, migration
  expression roundtrips, and actual 1.2.1 database reopening have regression
  coverage. Both cookbooks show the complete staged compositions.

Row equations are not included. Application policy and imperative workflows
stay in the host language. The Lean project and implementation TODO were
removed; the native conformance corpus remains, alongside independent integer
and rational endpoint oracles.

Each binary interval operation compares endpoints, independent of represented
width: intersection emits at most one piece and difference at most two per
binding. Multiple independent differences can emit up to `2^k` combinations;
ordinary join, materialization, sorting, and collection costs still apply.
`mulDiv` uses bounded native arithmetic. This is not a promise that arbitrary
queries run in logarithmic time. See the [local structural measurements](perf/structural-algebra-20260911.md)
for measured source provenance; a full 1.3.0 benchmark rerun has not been run.

Node 24+ and Effect 4.0.0-rc.112 are required. Supported native packages are
macOS ARM64, Linux ARM64, and Linux x64. Linux targets the Amazon Linux 2023
glibc 2.34 baseline. Use pnpm for installation and publication.

The release gate is the three supported-platform build/test jobs on the exact
release commit. Static Linux ARM64 and Miri are supplemental. Publication also
requires the matching Git tag and a GitHub Release with the five tarballs and
checksums; see [PUBLISHING](../ts/PUBLISHING.md).

Real-S3/IAM, Graviton performance, and larger-than-memory qualification have
not been established. Hosted generated migration orchestration remains
unsupported through the TypeScript/native bridge. Historical benchmark
reports retain their original source revisions.
