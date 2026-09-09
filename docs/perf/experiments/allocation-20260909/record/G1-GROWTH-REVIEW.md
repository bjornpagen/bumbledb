# G1: reclaim unpublished construction tails during growth

E1 timing is complete, with a strong empty-only win and mixed point controls;
all four previous candidate fingerprints and E1's unmounted source revalidate
in `e1-closeout-1/`. No identical timing rerun or new full trace is justified.

## Source and arithmetic evidence

Saved ownership identifies occurrence 1's nonempty root: 100,000 rows,
43,236 keys, 16,384 final groups. `grow_map` appends replacement ctrl/bucket/
dense ranges and leaves old tables behind. `force_unforced` owns a rollback
mark; ingestion only creates unforced children, never nested maps. Thus a
map under construction owns the three arena tails, with published maps before
them. Generic direct `grow_map` tests also exercise published maps: retain
that non-destructive behavior, do not silently give it a tail-only contract.

`g1-geometry-1/` is an arithmetic model, not a candidate measurement. Its
append model reproduces all three observed lengths AND capacities exactly.
It preserves the existing reserve_pool doubling and load thresholds.

| One root, three arenas | Current append | Final-only compact | Compact each growth |
| --- | ---: | ---: | ---: |
| Length bytes | 4,229,620 | 2,401,168 | 2,401,168 |
| Capacity bytes | 4,423,680 | 4,423,680 | 3,604,480 |
| Requests | 21 | 21 | 20 |
| Requested bytes | 7,176,160 | 7,176,160 | 6,094,816 |
| Additional copy bytes | 0 | 2,401,168 | 3,499,620 |

This predicts an 819,200-byte retained-capacity reduction (18.52% of these
three arenas), at the cost of 3.50 MB of copying. It is not a peak/RSS or speed
result, and not an allocation-policy proposal. Final-only compaction cannot
reduce the observed retained root capacities; do not re-run that hypothesis.

## Isolated experiment, not yet accepted

G1 starts from published b02a641e in `g1-source`; P2/P3/M1/E1 stay untouched.
Use the existing append+rehash, then overlap-safe leftward relocation back to
the old map's starts and truncate only that unpublished map's construction
tails. No shrinking, owner, allocator, load-factor, API or persisted-layout
change. The next growth can reuse the freed tail lengths, so the existing
geometric reservation is smaller. Do not bundle M1's duplicate-growth change.

The new construction-only call must assert it owns all three tails. Leave
generic grow_map unchanged so earlier readable maps survive rehash failure.
Cooperative chunked copying must never write before the construction starts.
If copying fails, the existing force rollback discards the unfinished map and
its descendants; it must not publish a partially relocated Map. Preserve
dense key/child order and all previously published map ranges exactly.

First establish an expected baseline failure for dead-tail retention, plus
passing multi-map/order/child/clone controls. Candidate checks must cover
zero/fixed/wide keys, multiple growths, mixed singleton/promoted children,
older maps and tokens, reset/rebind reuse, copy overlap and cancellation with
healthy reuse. Test internal copy failure precisely; do not label a synthetic
seam test as a public-operation cancellation demonstration.

Then measure actual saved-corpus owner/request deltas using the existing
external observer and exact SQL draws, followed by bounded ordinary cold and
warmed controls on frozen binaries. Preserve adverse/noisy results. No full
trace, full benchmark, commit, push, release or Pi qualification.

## Baseline discrimination complete

`g1-baseline-1/` (session 76404) completed 09:38:36 UTC. The multi-map,
zero/fixed/wide-key, dense-order, child, clone and reset control passes against
unchanged production code. The dead-tail assertion fails at its intended
retention check, not at a build or answer check. Its raw exit 101 is preserved
and explicitly classified as the expected baseline failure.

The isolated candidate now calls a construction-only wrapper around unchanged
generic grow_map. It asserts all three tail extents, rehashes, then copies in
64 KiB-or-smaller leftward pieces with cooperative checkpoints. There is no
new allocation during relocation and no shrinking. The local Map bases only
commit after all three arena relocations succeed. Any failure still returns
through force_unforced's original rollback. New helper tests exercise exact
overlap/chunk boundaries and a deterministic refusal after one actual copy,
checking untouched published prefix plus discard/reuse. The latter is an
internal relocation-seam test, not a new public-boundary cancellation claim.

Focused 1 passes all four tests at 09:39:46 UTC (session 98153).
Gate attempt 1 stops on three lint findings: poll/pool similar names and two
missing documentation backticks. No logical test or allocation measurement
failed. The failed gate remains intact; only those naming/docs issues are
corrected for gate 2.

Gate 2 (session 24937) completed 09:45:04 UTC, exit 0. All four focused
tests, format, strict engine/benchmark all-target clippy, 1,325 ordinary
library tests, 1,338 allocation-enabled library tests, ordinary release build
and 2,879 independent query-oracle cases pass. Both library runs retain 18
ignored tests; they were not run or counted as passing. Oracle stamp:
`ca43fe522175ebb42c1821a0b9364252ebd15c76ed0dfc9ab458f54a7edf4a7e`.
This is correctness/build evidence, not memory savings or speed acceptance.

Frozen gate-2 source fingerprint:
`084f09fd3a2eaaf73b9f0d2c13ddf4f7e12422612e995e27c1872b48547b38df`.
Ordinary executable SHA-256:
`7b67966ba1e74b3997289c2d5fae981c24b67bb670d0f315bf04e5b1e6092764`.

## Actual saved-corpus allocation and ownership

`g1-saved-1/` (session 91139) completed 09:46:09 UTC, exit 0. It uses exactly
the M1 baseline observer/test/schema and SQL draws on a byte-identical private
DB copy. The gated production force/grow bytes were checked before its build;
all source/observer/corpus identities remained fixed. Original inputs are
unchanged. Instrumented debug executable SHA-256:
`8927241f38955a19ad42560eefa4b42c7ffd2a795343616fe1c0d9ab404dd0ad`.

`g1-saved-review-1/review.log` completed 09:46:15 UTC. All 20 exact SQL windows
and 78 active/parked owner pairs checked, including all 50 changed snapshots.
Every map's key count, width, bucket count and published-table bytes match
baseline. Every candidate map arena now contains only live tables. All clone
counters remain zero; no claimed actual-workload cloning benefit.

| Saved window / owner | Baseline | G1 |
| --- | ---: | ---: |
| Cold `[1,6)` allocation requests | 166 | 162 |
| Cold `[1,6)` requested bytes | 16,597,148 | 15,442,076 |
| Cold `[1,6)` freed bytes | 4,806,726 | 4,507,718 |
| Cold `[1,6)` map-arena lengths, bytes | 4,284,644 | 2,433,592 |
| Cold `[1,6)` map-arena capacity, bytes | 4,497,408 | 3,641,344 |
| Cold `[1,6)` all COLT-pool retained bytes | 6,876,432 | 6,020,368 |
| Four cached draws: all COLT-pool retained bytes | 8,249,384 | 7,319,592 |

The three nonempty cold draws each save four requests, 1,155,072 requested
bytes and 856,064 retained COLT-pool bytes. For `[1,6)`, lower requested bytes
minus the reduction in freed bytes equals the retained-capacity saving exactly:
1,155,072 - 299,008 = 856,064. The other nonempty cold windows have the same
deltas; their absolute image/other-pool costs differ slightly.

The large occurrence-1 root matches the model exactly: three-arena lengths
2,401,168 bytes and capacities 3,604,480, versus 4,229,620 / 4,423,680.
Its retained capacity shrinks by 819,200 bytes (18.52% of these three arenas),
not by the full 1,828,452 bytes of removed content. The smaller occurrence-0
arenas halve from 73,728 to 36,864 capacity bytes per nonempty cached draw;
their image/filter position owners are unchanged. Four cached draws save
929,792 retained COLT-pool bytes in total.

The fresh empty draw saves one request / 1,081,344 requested bytes and
819,200 retained bytes, but still builds the unnecessary root because G1 does
NOT contain E1. The rotating empty draw's requests are exactly unchanged;
the large root was already built. Do not add isolated E1/G1 savings together
or treat this as an integrated result.

All eight same-draw warm/repeat windows and four second-rotation windows still
request and free zero bytes. First-rotation additional nonempty views each
save three requests / 73,728 requested bytes; the retained saving then grows
by 36,864 per cached view. SQL answers remain five accounts per nonempty draw
and zero for the empty one. No sample/window or adverse result was dropped.

These are actual requested layouts and payload capacities, not allocator
usable size, process RSS, mmap size, peak live ownership or Pi qualification.
The arithmetic copy cost remains a prediction from growth geometry, not an
ordinary speed measurement. Correctness plus smaller retained owners does not
establish that the copying pays for itself in elapsed time.

## Ordinary timing and next step

All temporary observer hooks have been removed. `g1-closeout-1/` verifies the
exact unmounted gate-2 identity, clean diff/format checks, and unchanged
E1/M1/P2/P3 fingerprints. A subsequent matched per-draw ABBA completed
09:54:12 UTC (session 25892); all 128 distributions and 96 clock brackets
were reviewed at 09:54:24. `G1-PER-DRAW-TIMING.md` is the timing authority.
Nonempty cold p50 changes are -4.49%, +1.42%, +1.71%; warmed results and
point controls are mixed, with four clock flags retained. This does not
establish a consistent speed win. Keep G1 isolated and unaccepted; do not
repeat the identical panel to obtain cleaner controls.

`g1-closeout-2/` revalidates the exact restored gate-2 source and all other
candidate fingerprints after temporary timing mounts were removed, with
format/diff checks passing. All sessions are terminal. The next lead is
`G2-STAGED-GROWTH-LEAD.md`: reuse the existing scratch owner for occupied
key/child records and avoid whole-table copying. Its four-owner arithmetic
explicitly includes a larger retained scratch buffer; no G2 production code
or measured saving exists yet. Preserve the distinction between model and
measurement, and between isolated candidates and an integrated result.
No full trace, full benchmark, commit, push, release, API/layout change or
trace-exhaustion declaration is justified by this result.
