# P3: flat overlap filtering before a tree redesign

Source/site audit while the frozen P1 experiment runs; no code or trace change.

The saved `t2_overlap_join/resumed-analyzed.summary.json` from df737a6f has
25.86% exclusive CPU under OverlapCache::query_into, 7.33% under lookup, 6.78%
under Walk::report and 6.29% under overlap_gather. Source-line attribution
narrows the first owner substantially:

| Current overlap.rs site | Saved exclusive CPU share |
|---|---:|
| line 177: start/end cutoff in the flat loop | 4.02% |
| line 180: end > query-start predicate | 6.11% |
| line 181: push the matching position | 9.49% |

Those files are unchanged from the captured source. Thus at least the named
19.62% sits at the flat filter/push sites, not the large-group implicit max-end
tree. Missing leaf source accounts for 19.51% of trace CPU and unresolved
stacks 0.48%; do not invent attribution for it or treat correlated samples as
independent measurements. The main ordinary t2 baseline median is 41.386 ms.

## First experiment to consider

The flat arm handles len <= 128 and produces matching position IDs in start
order. Compare its existing branchy append with a safe compacting pass over
the eligible start-prefix: copy the prefix positions to retained output, then
compact by `end > q_start` and truncate. This may avoid unpredictable per-hit
branches but adds copying/writes, so it is a hypothesis, not an obvious win.
No new persistent buffer, unsafe set_len shortcut, fixed capacity policy, or
tree representation change is needed for that isolated experiment.

Memory tradeoff to measure explicitly: copying all eligible positions may grow
the output to the eligible-prefix length even when few positions match; the
current append path grows only for hits. Record both retained capacity and
cold requests, and include early-cutoff/no-hit queries where an extra binary
search or copy may be worse. Reusing a Vec is not itself proof of lower memory.

Before importing a change, verify current tests cover zero/one/127/128/129
groups, no/all/alternating/random hits, duplicate starts, rays and equal/abutting
endpoints, and compare start-order as well as the unordered hit set. Include
invalid/intersected query windows exactly as current callers produce them;
candidate generation may be conservative, but exact Allen rechecking must
remain unchanged. Test reuse with decreasing/increasing hit counts and no warm
allocation changes. Ordinary temporal and calendar controls, sparse and dense
overlap draws, decide whether additional copies actually help.

## Later alternatives, not bundled

`run_node` consumes the fully staged overlap_hits in batches. A streaming or
bounded iterator could reduce retained peak hits, but is a larger lifetime and
cancellation change and cannot be justified as per-row warm allocation: the
current vectors already retain capacity. Preserve first-emission/refusal order,
exact residuals and no missing candidates. The present trace says to first
test the smaller flat-filter mechanism rather than rewrite the whole index.

The directory key is occurrence plus ancestor binding words. Replacing that
with tagged cursor identity is only valid after proving view/cursor lifetime,
cover level and interval-column identity for every caller. The current executor
resets the overlap directory each execution. Do not declare a missing-key bug
just because an interval-word index is not stored explicitly; the single-leaf
plan may make it invariant. A small last-directory reuse experiment would also
need actual repeated-key evidence and negative controls, not just fewer hashes
in source.

## Further source audit during P2 comparison (September 9)

Important correctness discriminator: DO NOT add a q_start >= q_end early
return. overlap_enumerate combines constraints for one interval word using
max(query starts) and min(query ends). The resulting pair can be reversed
without making the conjunction impossible: a cover interval [0,5) overlaps
both [1,2) and [3,4). The combined filter is start < 2 AND end > 3, and that
cover must remain a candidate. Equal endpoints can likewise represent two
abutting constraint windows. query_into is a conservative two-inequality
candidate filter, not a standalone validity check on an interval value.
The downstream exact Allen residuals remain authoritative. Existing random
cache tests generate valid windows, and their one empty-window test uses 0/0
where nonnegative stored starts happen to give no candidates; it does not
justify an unconditional empty/reversed-window optimization.

Before changing filtering, add a boundary matrix with lengths
0/1/16/127/128/129, duplicate starts, finite/infinite ends, scrambled position
IDs, and equal/reversed query endpoints. Compute the expected ordered sequence
from the actual cache's start-sorted permutation (unstable equal-start order
is not a cross-run contract), then apply the two independent inequalities.
Keep exact index enumeration order through reuse/reset/growth, not only a
sorted hit-set comparison. Test sparse/no/all/alternating hits and shrinking
then growing output. The end-to-end two-disjoint-Allen-constraints fixture is
an additional independent check, not permission to rewrite the planner.

No P3 production change or new measurement/capture has been made here; P2's
ordinary three-binary comparison remains source-frozen.

## Separate representation lead from the same saved evidence

Current probe/build constructs a complete 2*p max-end tree for EVERY group,
including the len<=128 groups whose query arm only reads the contiguous end
leaves and never visits an internal max node. The original temporal generator
has 150,000 random spans over 2,000 keys with a 1/50 hot-key skew and 1/50 rays;
the ordinary groups are therefore relevant, not a fabricated microcontroller
case. Do not infer their exact distribution or memory from the averages.

After choosing the flat-filter experiment, quantify actual per-group lengths,
padded tree words, live slab lengths/capacities and build cost using the saved
corpus/generator. A small-group representation could retain only len end words
in the SAME tree/end slab; large groups retain their existing implicit max tree.
This removes unused internal nodes and padding without adding a buffer owner,
indirection per entry, memory policy or output streaming rewrite. For example,
75 real end words currently occupy 256 tree words; that equation is live
payload, NOT a measurement of retained Vec capacity or RSS.

Keep this separate from filter acceptance. Both Dir's tree_base/p conventions
and the small-arm end offset would need to agree; boundary 128/129 and mixed
small/large groups sharing the slab are mandatory. Later table/slab growth,
reset/rebuild, old directory reads, empty/singleton groups and equal/reversed
constraint filters must remain correct. A final Vec may retain similar high-
water capacity after reuse, so shortened lengths alone are not a memory win.
No implementation, timing result, or priority promotion is implied yet.

## Saved-corpus structure audit, 07:27:54 UTC September 9

p3-saved-structure-1/ completes a read-only SQL group count against the frozen
ordinary corpus identified by full/MANIFEST.json. The database SHA256 is
recorded and unchanged after inspection; raw schema, every per-key count,
source/manifest hashes and the driver are preserved. No index code changed,
new corpus was generated, process was profiled or workload timed.

Actual saved data: 150,034 spans across 2,000 keys. Exactly 1,999 keys have
46-106 spans (146,952 positions total), so they fit the existing <=128 flat
arm. The one remaining key has 3,082 spans. This is a measured corpus size
distribution, not an inference from the generator's average.

IF one full cache group is built per key, the current 2*next_power_of_two(n)
tree representation occupies 481,536 u64 words. Keeping only the end leaves
for flat groups and the existing tree for the large group would occupy
155,144 words: 326,392 fewer words, or 2,611,136 bytes (~2.49 MiB) less live
payload. These are structural projections from actual counts and current code,
NOT measured executor directory counts, retained capacities, allocation requests
or RSS savings. Cache build order and Vec growth can change retained capacity.

Next discriminator: measure actual cache directories/slab lengths/capacities
and construction allocations on these saved groups, confirming the executor's
group/key layout and build coverage rather than assuming it from SQL counts.
Keep the flat-filter CPU experiment separate, and add the previously described
two-inequality boundary/order matrix before changing either production path.
The 128/129 transition must be tested even though this saved corpus has no
groups near that boundary. This lead remains open; it does not exhaust P2 or
authorize a new full trace.
