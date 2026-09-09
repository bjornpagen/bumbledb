# G1 ordinary timing: construction copying and warmed table layout

The preceding goal turn made progress: baseline red/green retention tests,
full correctness/build/oracle checks and exact saved-corpus ownership audits
now establish real requested-byte and retained-capacity savings. This turn
revalidated all five candidate source fingerprints before acting. G1 remains
isolated and not speed-accepted. No new full trace or full benchmark.

## Protocol fixed before measurement

Reuse the byte-identical `e1_timing.rs` driver with published baseline and G1
in the same isolated g1-source path. A temporary Cargo binary entry is removed
before timing. Both executables use workspace fat-LTO, opt-level 3, one codegen
unit and ordinary non-test release code; no allocation counter or observer.
Only G1's force/grow production files differ. The complete gate-2 candidate
source is restored before measurement; other candidate worktrees stay frozen.

Use one fixed per-case ABBA panel: A0 B0 B1 A1 for triangle then point for
each of the four saved draws. Inputs are private byte-identical copies of the
existing S/seed-1 database and read-only SQLite oracle, with original/copy
hashes checked. Every process checks all four family draws against handwritten
SQL goldens before timing, and validates outputs outside the timed operations.

- Cold/second: reopen DB and prepare outside timers; two discarded pairs then
  16 first/second-execution pairs. Parameter arrays are outside both timers.
- Alternating-parameter warmed (`rotating`): alternate `(target+1)%4` outside
  the timer, then time target; eight warmups and 64 recorded samples.
- Same-target warmed (`memoized`): eight warmup calls and 64 recorded batches
  of 16 calls. These two warmed lanes include the existing parameter helper.

The frozen driver's old result-cache comment is wrong; keep its evidence
identity intact and use the corrected terminology here. Both warmed lanes
execute rules each time; they do not reuse cached answer rows.

G1 copies only during construction, but it changes live tables' arena offsets
and capacities, so warmed lookup timings must also be checked. Do not assume
that no warm copying means no possible warm performance change. The point
control does not use these COLT maps; retain it, including noisy/adverse results.

All raw unsorted samples, means, percentiles, maxima and clock flags stay in
the evidence. No retries, dropped windows, frequency normalization or repeat
until win. First/second share a clock bracket; the two warmed lanes each have
their own. Expected 32 processes, 128 distributions, 96 independent brackets.
macOS user-interactive QoS is verified steering, not hard P-core affinity.
No shared-host timing is a Pi, RSS or universal performance qualification.

Review every distribution and control before disposition. Ordinary build or
oracle failure is not a speed result. An unclean timing result does not erase
the already measured allocation saving, but memory alone does not establish
that additional construction copies are a net speed improvement.

## Matched builds

Candidate build 1 completed 09:50:43 UTC (session 30419), passing format,
strict driver lint and ordinary release build. Binary SHA-256:
`cf75ed8b50e5af393f461628ef84643b9281133ddf30a8b6941d96fe4f108692`.
The compiler reports engine features exactly `[collision-probe]`, opt-level 3,
non-test code and no debug assertions. No profiling/allocator code is enabled.
The driver remains the exact E1 driver, SHA-256
`13e2309e46788f7778394c80f16b910679da976e240531a8649971fd33ebb4d1`.

The candidate production files are frozen. Only the two G1 production changes
were temporarily removed in this isolated worktree for baseline build 1;
the baseline builder checks both files exactly match published HEAD. The
temporary test references are not part of the ordinary release target.
Baseline build 1 completed 09:52:34 UTC (session 13340), passing the same
format/lint/release and ordinary-profile/feature checks. Binary SHA-256:
`d5d31f98f78300b383542fe7ccf2d46bb9ceac1defc6622234f0aa0ccc44ff17`.
The baseline's two production files match published HEAD exactly; driver
hash matches candidate 1. Both builds are frozen and terminal.

Both candidate files were then restored byte-for-byte and the temporary
Cargo entry removed. All five ordinary candidate fingerprints match
g1-closeout-1, including G1's exact gate-2 source. Comparison 1 started in
`g1-timing-1/` at 09:53:26 UTC (session 25892). No source edits or compilation
overlapped this timed panel.

## Completed results and disposition

Comparison 1 completed 09:54:12 UTC, exit 0. All 32 processes passed their
four handwritten-SQL goldens and checked measured outputs. Original/private
corpus hashes stayed unchanged. `g1-timing-review-1/review.log` completed
09:54:24 UTC, exit 0: all 128 distributions independently recomputed from raw
samples, all 96 independent clock brackets reviewed. Four flags remain;
zero retries, no dropped samples or frequency normalization.

Triangle geometric round-center B/A p50 / mean (A published, B G1):

| Draw | Cold | Second | Alternating-parameter warmed | Same-target warmed |
| --- | ---: | ---: | ---: | ---: |
| `[1,6)` | .9551 / .9650 | .9887 / .9765 | .9693 / .9367 | 1.0090 / 1.0215 |
| `[167,172)` | 1.0142 / 1.0105 | 1.0028 / 1.0158 | .9928 / .9963 | .9954 / .9927 |
| `[333,338)` | 1.0171 / 1.0083 | 1.0104 / 1.0205 | .9945 / .9934 | 1.0030 / 1.0116 |
| `[500,500)` | 1.0048 / .9953 | 1.0009 / .9995 | 1.0076 / 1.0045 | .9970 / .9968 |

Nonempty cold p50 changes are -4.49%, +1.42%, +1.71%, with mean changes
-3.50%, +1.05%, +0.83%. This does NOT establish a consistent cold speed win.
Draw 0 round medians are A 9.788 / 9.220 ms, B 9.102 / 9.044 ms; draw 1
A 9.244 / 8.793 ms, B 9.607 / 8.702 ms; draw 2 A 8.752 / 8.709 ms,
B 8.756 / 9.004 ms. Keep these changing baselines visible.

Warmed centers are mostly close, not universally favorable. Draw 0 same-target
mean is +2.15%, maximum +36.95%, without a clock flag. Draw 2 second-execution
mean is +2.05%, maximum +18.19%, also unflagged. Draw 2 same-target mean is
+1.16%, maximum +17.81%; its B1 bracket is flagged, but the larger B0 mean
and tail must not be misattributed to that different B1 flag. Read the raw
rounds rather than using one flag to dismiss a whole family.

Point controls, same p50 / mean ratio convention:

| Draw | Cold | Second | Alternating-parameter warmed | Same-target warmed |
| --- | ---: | ---: | ---: | ---: |
| 0 | 1.0612 / 1.1318 | 1.0950 / 1.1386 | 1.0000 / 1.0260 | 1.0268 / 1.0244 |
| 1 | .9597 / .9533 | .9571 / .9341 | .9542 / .9698 | 1.0098 / .9993 |
| 2 | .9830 / .9822 | 1.0950 / 1.0500 | 1.0000 / 1.0253 | 1.0014 / .9979 |
| 3 | .9543 / .9215 | 1.0000 / .9484 | .9438 / .9517 | .9696 / .9697 |

Point 0 cold is an adverse unflagged control (+6.12% p50 / +13.18% mean),
with A medians 2,375 / 2,375 ns versus B 2,459 / 2,583 ns. Its second-call
medians span 417–500 ns (roughly 42 ns clock steps); do not turn the +9.5%
ratio into a precise causal regression. Point 2 second calls are similarly
quantized. Points do not use the modified COLT path, but their adverse results
remain in the evidence, not excused away or counted as performance green.

The four unique flagged brackets (cold includes second execution) are:

1. point-1-A0 cold: 3.157 -> 3.360 proxy GHz
2. triangle-2-B1 memoized: 2.958 -> 3.345
3. point-2-A0 memoized: 3.324 -> 3.145
4. point-2-B0 memoized: 3.234 -> 3.185

G1 remains a verified retained-memory saving with mixed ordinary timing, not
a speed-accepted or universal performance-green candidate. No identical-panel
rerun is justified merely to get cleaner results. All build/measurement
sessions are terminal; exact gate-2 source remains restored, no temporary
Cargo or observer hooks. No full trace or benchmark suite ran.

The next source-supported question is whether growth can stage only occupied
key/child tuples in the already-existing rehash scratch vector, expand the
unpublished table in place, and rewrite dense indices without copying whole
tables. That is a separate representation experiment, not an accepted G1
optimization. Explicitly price the larger retained scratch buffer; moving
waste into an uncounted owner is not a saving. Preserve G1 evidence and all
other candidates, and keep the full-trace gate closed.
