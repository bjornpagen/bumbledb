# Saved control traces reviewed after A0 completed

Read the existing analyzed owners and exact caller tails for point, stats,
triangle, displaced probe/stream and recursive fanout, plus the newer range
analysis. No capture or symbol-processing job ran alongside the timings.
These are historical CPU attribution, not current latency or speedup claims.

## Sources and source changes

- Native83 point/stats/triangle: `d67de96a8e1469603e597197499b00e98762333f`,
  `bench-out/autoresearch-native83-*/analysis.summary.json`.
- Native81 displaced probe/stream d24 and closure_fanout:
  `77fcc4eddd1a82ce401392b1ba967fd0f866ab5d`,
  `bench-out/autoresearch-native81-*/analysis82b.summary.json`.
- Newer range: `df737a6f`,
  `bench-out/autoresearch-87-df737a6f/native-reads/range/whale-analyzed.summary.json`.

The sampled old deadline and ExecutionPolicy paths have been removed. Current
ordinary work cancellation is not evidence that those old timer costs remain.
Source diffs also show substantial changes to aggregate scan dispatch,
keys-only COLT iteration, projection routing, resident result finalization and
recursive image delivery. Do not add their old percentages to today's leads.

## What each control tells us

**Point:** the strongest old paths end in `probe_determinant_bucket -> heed
get_greater_than_or_equal_to -> LMDB cursor/page/node search`, not sibling
probing. Historical nearest owners were determinant lookup 33.93%, physical
comparison 14.45% plus its closure 4.95%, and now-obsolete native timer 15.47%.
The comparator is unchanged; row lookup changes are chiefly cancellation
plumbing. This is a useful negative control for P1, not a reason to expect a
probe-run optimization to speed direct keyed storage reads.

**Stats:** the old caller path reaches pipeline -> leaf `iter_map ->
copy_map_batch<2>`, then constant-group min/max reduction. Its old map-copy
owner was 49.79% and strided min/max 23.05%. Current iteration adds a CHILDREN
compile-time switch and keys-only path, so the old child-load/cursor-write
cost is not still owed. Key copying and strided reduction remain. Any future
borrowed map-key fold should be isolated, preserve forced-map distinctness and
batch cancellation, and not be folded into P2's ordinary row-getter rewrite.
Min/max code itself is unchanged; signed sum code has been unified separately.

**Triangle:** old prefetch (15.53%), gather/hash (14.92%), probe walk (12.21%)
and sibling batch (11.95%) all lie on the same pipeline probe path. Those exact
old helpers were subsequently refactored, so the newer unchanged r4 caller
evidence remains the P1 attribution authority. Triangle remains an affected
control, particularly for prefetch regressions; P1 deliberately leaves the
whole-batch prefetch pass unchanged.

**Displaced probe d24:** old pipeline/probe routing dominates but scan-side
dispatch and column access are also substantial. The exact hot scan stack
indexed `FoldSource` repeatedly, which has since changed to prepared shared
fold inputs. Do not count that old 15.67% scan owner as a new removable cost.
Current public-operation allocation counters show only the 24-byte work owner,
not per-row Rust allocation. Keep all displacement levels: A0's d96 p50 is
below d24 on this shared host, so cache-mass ordering alone cannot establish a
candidate effect.

**Displaced stream d24:** 63.37% of old captured CPU is outside the engine;
the largest exact stack is the deliberate ForeignStream, not database work.
The next leaders are dense sum (18.11%) and the obsolete deadline timer
(13.06%). This is a sum/scan negative control, not a sibling-join optimization
target. Ordinary query timings exclude the deliberate between-draw work;
sampled CPU per draw from this profile cannot replace those latencies.

**Recursive fanout:** entry/probe hashing into the result set dominates the
old paths (11.01% entry, 10.95% probe), followed by derived-image delivery.
The census independently attributes substantial cold key/dense growth to the
recursive result set. Old ResidentRows-per-next and image-delivery code changed;
review current wordmap submodules before proposing a duplicate fix. In fact,
`wordmap/entry.rs` already checks duplicate keys at the load boundary before
growing, with a direct regression test in `tests/behavior.rs`. M1 is specific
to COLT's still-unfixed construction path, not a missing WordMap optimization.

**Range:** the newer trace still contains the old resident Cell-fill and
projection-copy sites, but both source files differ materially at HEAD. The
current 72 requested bytes/public operation (parameters plus work owner) do
not establish a per-row heap problem. Retain range as an output-path control;
do not repeat the previous resident-representation rewrite based on old shares.

## Limits and next action

Unresolved-stack shares of the six older captures range from 0.10% to 1.56%;
missing leaf source is much larger (10.15%–45.88%). Nearest source owners are
attribution conventions and include descendant library work. Parameter mixes,
old timer overhead and changed source prevent direct cross-era percentages.

This pass keeps P1 first and strengthens its distinct negative controls. It
adds a lower-priority map-key-copy question but does not qualify it as a
measured opportunity after the keys-only changes. P2/P3 and M1 remain open;
the saved traces are not exhausted. Await the frozen alternating ordinary
comparisons before changing engine source or launching another diagnostic.
