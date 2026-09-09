# P2 compact routing — not performance-accepted

Historical gate-4 candidate below. Comparison 2 completed and is fully
reviewed in P2-THREE-VARIANTS-REVIEW.md. P2-INPLACE-REVIEW.md is the latest
source/gate/continuation state; neither prior candidate was accepted.

## Why the initial P2 was refined

The original linear-getter P2 is NOT accepted. Ordinary comparison 1 found
repeatable o4 centers about 7% lower, but controls remain mixed (especially
j4), and the completed wide discriminator proves O(K*W) work is unacceptable:
16/16 key/group widths cost about 1.25x staged, 64/64 about 2.84x, and 128/128
about 5.1x. Those three wide-group cases had unflagged clock boundaries. This
is mechanism evidence, not the published-baseline complete code generation.
Raw logs and frozen test source/binaries remain in p2-wide-1 (compile failure)
and p2-wide-2 (complete), not overwritten or discarded.

## Replacement and ownership

The refined candidate replaces cached_outer_slots with a slot-indexed
Vec<Option<NonZeroU32>> (four bytes/slot: None=outer, Some=one-based leaf word).
Shape refresh constructs it once per layout/reset. Group-constant detection,
integer fold-input preparation, float input selection and row reads share it.
Required-word lookup is O(1), not a repeated linear key search. The distinct
batch takes/restores this existing cache owner around group mutation; ordinary
dedup batches retain full staging and outer reads once per batch. Semantic
Elided sinks no longer allocate unused binding staging; execution-scoped
physical witnesses retain it for the next reset. There is no new owner,
width threshold, unsafe indexing, storage/SDK change or allocation policy.

Borrow/lifecycle audit: shape metadata is validated unique; refresh overwrites
every route, and aim/reset invalidate the cache even for unchanged key_slots.
Group spilling does not read or clear either taken owner. Typed row-fold
failures return to the enclosing loop/caller, which restores it before return.
Only permanent Elided drops staging; ordinary/Union/DNF and temporary physical
witnesses preserve it. Release drops both route and staging capacities and
reset restores the ordinary staging width. Zero-word grouping is an empty
all-outer span, not permission to skip dedup. Actual Vec capacities must be
measured: small/minimum-capacity and mostly-leaf ordinary plans can retain more
routing bytes even though semantic Elided drops unused full-row storage.

## Gate 4

Completed at 06:41:35 UTC, session 38048 exit 0: formatting, strict all-target
clippy, all 1,327 library tests, 292 allocation-enabled executor tests, release
build and all 2,879 independent oracle cases passed. Frozen binary:
p2-gates-4/bumbledb-bench. SHA256:
e0d1711a2912e4c330f09d81ff6ea70d6c4bb3cd487f29e7f659a757903ff429.
Production source fingerprint:
1a84596e19e845989f2150137fbde99977ce3a126f321d4dc9927fd2cf57c572.
Private corpus p2-gates-4/data; verify stamp:
ee75234984a71b4b58b0bbc38e69d1b12b32a06d4040f2bb2dfbb4d1bd152c1e.

## Focused discriminator complete

Session 60101 completed p2-wide.py 4 3, exit 0 at 06:44:16 UTC under the serial
lock. The temporary test-only fold_row.rs hook has been REMOVED. The complete
gate-4 source fingerprint was revalidated, as was the production file SHA256:
2dcb38826fa120559b85c9c9723431bd6b62b3abbac56a57d39a7e453d8b7d32.

The revised control has independent preallocated row/outer-slot storage; no
per-row searching, allocation, or take/restore overhead is charged to it. Both
arms share candidate shape refresh and numeric folding; this is an intentionally
favorable staged control, NOT the old complete implementation. Each case also
reports actual scratch/routing capacities. No speedup is accepted from gates.

All 36 clock-boundary windows are unflagged; this does not exclude smaller
frequency differences or interior pauses. All independent expected answers
passed. The wide lookup cliff is gone: 16/16, 64/64 and 128/128 widths now
favor direct inputs instead of losing with increasing width. Narrow 1/1
also favors direct reads in this small fixture. The total measured workload
is intentionally short; do not generalize tiny ratios to real applications.

Frozen test binary: p2-wide-3/bumbledb-wide-test, SHA256:
0ca4f7bf92c792c15c73a8f02eaf3f1227e2774a2f740aa107aab28061d588ea.
Raw samples and capacities: p2-wide-3/measure.log. Cold staging/routing
payloads in these semantic-Elided cases (shared key-layout Vec excluded):
K=1: 72 -> 20 bytes; K=4: 96 -> 32; K=16: 192 -> 80;
K=64: 576 -> 272; K=128: 1088 -> 528. These are actual retained capacities
for this fixture, NOT total query memory or RSS and NOT ordinary-dedup plans.

Direct/staged geometric centers of round medians (all samples retained):

| Key/group words | Initial linear lookup | Compact lookup |
|---|---:|---:|
| 1/1 | 0.9367 | 0.9639 |
| 4/1 | 0.9083 | 0.8581 |
| 4/4 | 0.9594 | 0.8648 |
| 16/1 | 0.8493 | 0.6344 |
| 16/16 | 1.2638 | 0.8207 |
| 64/1 | 0.7626 | 0.3065 |
| 64/64 | 2.8464 | 0.7825 |
| 128/1 | 0.7958 | 0.1656 |
| 128/128 | 5.0597 | 0.7959 |

The columns are SEPARATE test-binary ablations, not a direct old/new speedup:
the staged controls differ as documented, and some initial narrow-case clock
windows were flagged. Their value is the removed width-dependent lookup cliff.

## Ordinary three-variant comparison (now complete)

Session 53558 runs p2-compare.py 4 2 --reference-gate 3. Source is frozen;
do not edit tracked files until it ends. Phase p2-comparison-2 uses ABC/CBA:
A=published baseline, B=linear-getter P2 (gate 3), C=compact routing (gate 4).
Each round measures all 12 joins/OLAP queries (24 samples, 8 warmups) and
point/range/stats/triangle/disp_probe reads (32 samples, batch one). It retains
independent binary-bound corpora, source and executable hashes, all raw
reports, and control/clock caveats. This is bounded ordinary measurement,
not a new full benchmark or trace. Acceptance requires reviewing all controls,
not just o4 or the passing focused discriminator.

Initial A0 is noisy and cannot be credited as a candidate win: j4 median
2.427 ms / mean 11.982 ms / max 76.078 ms is far above earlier baselines,
and stats/disp_probe clock windows are marked contaminated after the existing
harness's built-in retry. Keep the run and flags. No manual rerun or sample
dropping is justified by this observation. Read clock boundaries cannot screen
scenario windows, which have no such guard. Scenario reports retain pooled
summary statistics, not per-operation/per-parameter raw timings.

Static control audit: j4_five_way projects two strings; make_plain_sink selects
ProjectionSink for its all-Var finds. It does not execute aggregate folding.
AggregateSink remains boxed within EitherSink, so its scratch representation
does not enlarge the inline projection variant. This does not rule out code
layout/allocator/host effects, but a j4 timing shift is not direct evidence of
executing the P2 route. Its four parameter sets include an impossible reversed
year window and very different selectivities. o2 likewise rotates three
different day windows plus an empty one. A pooled median is not a homogeneous
operation cost; don't explain a change from one quantile alone.

Host spot-check at 06:47:30 UTC (read-only, no scheduling/settings changes):
the benchmark used about one core; Spotlight mds used 54.7% CPU and many
CGPDFService processes each used about 20-30%, alongside the app renderer.
Swap was 9,140.12 MiB used out of 10,240 MiB. pmset reported no thermal or
performance warning. This is evidence of concurrent host activity, not proof
that any specific pause or candidate/baseline gap came from one process.
Do not kill unrelated processes, clear caches, disable indexing globally, or
change the host halfway through the source-frozen comparison.

## Remaining correctness regression before acceptance

Add an explicit borrowed-batch route-owner test once comparison 2 is terminal:
semantic witness, first nonconstant-group batch, force checked group-count
overflow on a different full binding in the same group, assert typed terminal
reply and unchanged route pointer/capacity/contents, reset, changed key-word
layout, release, and reset/refill with a zero-key all-outer batch. Check exact
final output and zero staging capacity. Existing gates cover ordinary staged
refusal, physical-witness reset, Float/Pack layouts, empty selections and union
reaim; the new route Vec's take/restore deserves its own direct regression.
This is test coverage for the current source, not evidence of a found defect.

## Next isolated ownership experiment (not yet implemented)

C0/C1 o4 medians are 24.299/24.334 ms, versus B0/B1 21.847/22.003 ms.
The width fix does not automatically preserve the initial narrow-leaf gain.
The staged/direct wide fixture batches 128 rows at once and barely exposes
the cache take/restore overhead paid on every singleton pinned-leaf batch.

Keep constant-time lookup, but let the row-reader closure receive a borrowed
lookup slice AT EACH USE inside fold_row. Scalar/staged readers ignore it.
The batch reader then captures only LeafBatch/entry, not a reference to a
sink field, so fold_batch_rows need not mem::take/restore cached_leaf_words.
The group-key read finishes before probe_group's full mutable borrow; numeric
and Pack reads can borrow the lookup alongside their separate accumulator
fields. This adds no owner, unsafe access, new allocation or width heuristic.
It is a falsifiable ownership/code-generation refinement, not a guaranteed
explanation of all C/B timing differences. Add singleton-batch mechanism
coverage and keep wide and ordinary controls. Wait for comparison 2 to be
terminal before modifying tracked source.

The first two wide attempts are terminal, and their test hook was removed and
the gated 66f441... source fingerprint was restored before this refinement.
See P2-COMPARISON-REVIEW.md for initial ordinary ABBA, including all noisy and
losing controls. No full trace, release, tag, npm publication, commit or push.
