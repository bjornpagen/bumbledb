# P1 ordinary evaluation review — measurements complete, candidate unaccepted

No accepted performance result yet. Source remains frozen at the recorded
local P1 patch; no full trace or release. `p1-review.py` reads successful
hash-checked report receipts and prints descriptive adjacent-control ratios.
It never accepts a candidate from those ratios alone.

## Completed controls

Both oracle gates passed 2,879 cases. A0 scenarios and reads passed; A1's
34 scenarios passed at 03:57:38 UTC. A1 reads now run under original session
1549, child 94950 (observed live via the session). Recovery monitor session
35796 is still live but idle, awaiting the expected B0 read-stamp refusal.

The A1/A0 scenario p50 comparison is from the **same baseline executable**:

- r4: 1.0202 (about 2.0% slower).
- r3: 0.9781 (about 2.2% faster).
- r6: 1.0511 (about 5.1% slower).
- o4: 0.9467 (about 5.3% faster).
- temporal overlap t2: 1.0365 (about 3.7% slower).
- r2: 1.1024 (about 10.2% slower).

Cheap mixed-parameter medians vary more: g2 A1/A0 is 0.5297 and g6 is 0.7655.
Do not classify those as gains, use them to excuse all regressions, or combine
mixed-parameter percentiles as independent samples. Inspect complete p50,
mean and tail reports with the later alternating controls. A0's ordinary read
clock guard retried conflict_pairs and disp_probe; neither final window is
marked contaminated. Preserve both facts rather than discarding the rounds.

The A0 displaced d96 median (92.79 ms) is below d24 (131.48 ms). These are
sequential shared-host measurements, not a monotone cache-capacity proof.
Do not use the different displacement masses as an A/B optimization comparison.

### A1 reads completed

A1-reads passed at 04:05:13 UTC. Original session 1549 advanced to
B0-scenarios, candidate child 3836, revalidated live and CPU-active. The
source/executable hashes and all completed receipts still validate.

A1/A0 read p50 ratios (same baseline executable): point 1.0022, range 1.0956,
stats 1.1164, triangle 1.0336, conflict_pairs 0.5561, disp_probe 0.9017,
disp_probe_d24 0.7316, disp_probe_d96 1.0910, disp_stream_d24 1.1059. A1's
clock guard retried stats and disp_probe; no final read window is marked
contaminated. The d24 and mixed conflict-pair controls are especially noisy.
Small candidate deltas in those families cannot establish either a win or a
regression from one comparison. Preserve the original alternate rounds and
review the complete distributions; do not normalize them by SQLite timings.

## B0 scenarios and verification recovery

B0-scenarios passed all 34 queries at 04:26:12 UTC. The original driver then
failed at the expected B0 read-stamp gate, before timing (exit 1, session 1549
terminal). Its complete valid reports remain in the original attempt.

Recovery session **35796** acquired the serial lock and reverified the frozen
candidate against all 2,879 cases in its own candidate-data directory. The
stamp matches the original candidate verification. That passed at 04:27:19;
B0-reads is now running as child **10387** under
`p1-evaluation-2-resume-1`. Source, executable and adopted report hashes checked.
There was no rebuild, verification bypass or full capture.

### Preliminary candidate signal — not accepted

B0 compared with preceding A1:

| Family | p50 ratio | mean ratio | p90 ratio |
|---|---:|---:|---:|
| r4_bomb_t2 | 0.9310 | 0.9333 | 0.9330 |
| r3_bomb_t1 | 0.9031 | 0.9010 | 0.9041 |
| g4_mutual | 0.7979 | 0.7940 | 0.7190 |
| j4_five_way | 2.1395 | 1.2539 | 1.2584 |
| p3_bucket_fetch | 1.1237 | 1.1030 | 1.1025 |
| o4_segment_category | 0.9689 | 1.0029 | 1.0144 |

The heavy-join directions are encouraging, but j4 is a material regression
signal, not a result to hide behind an aggregate speedup. Its A0/A1 p50 values
were 843.584/749.500 us; B0 is 1,603.584 us. Its mean and p90 are also about
25% higher than A1, so this is not solely a median-label issue. Await A2 and
B1/A3 controls before assigning cause; if repeated, reject/refine P1 rather
than accepting just the r4 win. The p3 increase also needs the later controls.

Re-read saved `autoresearch-native81-j4_five_way/analysis82b.summary.json`:
the old path traverses four nested probe/pump stages and a pinned projection
leaf. Old nearest owners include probe_pass 9.73%, probe walk 8.00%, projection
batch source setup 7.93%, key_count 6.01% and pump 5.44%. Projection/finalization
changed since that capture, so percentages are not present-day shares. The
current j4 parameter roster has four distinct mixes, including an inverted
year-window no-match case; 24 samples cover six complete rotations. Inspect
mean/tails and, if needed, parameter-specific ordinary follow-up rather than
claiming every draw doubled. No new trace is needed for that investigation.

## Broad evidence checked during timing

`CONTROL-TRACE-REVIEW.md` records seven saved control-family reviews, exact
caller paths, source differences and obsolete costs. `MAP-OWNERSHIP-LEAD.md`
adds the downstream displaced-memory-model obligation and separates simple
duplicate-boundary growth from any later arena-tail compaction. No engine
change was added during these timing rounds.

The known verification handoff defect and receipt-preserving recovery are
documented in PROBE-LEAD.md. Revalidate the live handles before intervening;
do not restart because observation times out or because a phase is lengthy.

## B0 reads complete; second baseline control running

B0-reads passed at 04:51:57 UTC. Session **35796** advanced to A2-scenarios,
child **22512**, revalidated live and CPU-active. Source, binary and successful
report receipts still validate. No tracked code changed during the comparison.

The first candidate read window is mixed too. B0/A1 ratios:

| Family | p50 | mean | p90 |
|---|---:|---:|---:|
| point | 0.9978 | 0.9723 | 1.0000 |
| range | 1.0556 | 1.0992 | 1.0615 |
| stats | 0.9233 | 0.9506 | 0.9146 |
| triangle | 0.9376 | 0.9324 | 0.9177 |
| conflict_pairs | 1.3710 | 1.0715 | 1.0423 |
| disp_probe | 1.2847 | 1.1393 | 1.0428 |
| disp_probe_d24 | 0.9042 | 0.9342 | 0.9429 |
| disp_probe_d96 | 0.9576 | 0.9627 | 0.9748 |
| disp_stream_d24 | 1.0094 | 1.0845 | 0.9899 |

All nine final **engine** clock windows are unflagged. The d24 **SQLite**
window is contaminated and must not be used to normalize or excuse the engine
results. The engine range and displaced-stream maxima are elevated; preserve
these observations, not just the medians. The undisplaced probe mean also
increased, so its median regression cannot simply be dismissed as percentile
selection. Later alternating controls remain necessary before assigning cause.

`p1-review.py --metrics p50 mean_ns p90 p99` now reviews every family's
central/tail summaries, validates scenario answer-count equality, and prints
both explicitly preliminary one-sided ratios and adjacent-bracket ratios.
Optional `--families` filters display only, not receipt/roster validation.
All 34 scenario and nine read families were reviewed with these statistics.
With nearest-rank selection, p99 at 24 or 96 draws is the **single maximum**,
not evidence of a stable population p99. Original reports retain all seven
summaries; they do not retain draw-level timing/parameter pairs.

### Bounded regression follow-up if repeated

Current j4 rotates four parameter sets; eight warmups and 24 samples cover
two and six complete rotations. The saved capture recorded answer counts
2244/9402/29040/0. Current A0/A1/B0 each report the same integer mean of 10171
answers. The reported median is rank 12 of 24, at a parameter-distribution
boundary; its movement does not tell us which parameter became slower.
Mean and tail regressions still make this a real investigation requirement.

After the frozen protocol finishes, use a focused ordinary join comparison
and, if needed, retain per-draw timing plus parameter ordinal in a separate
diagnostic. Preserve the original rotating workload; timing one parameter in
isolation changes memo/cursor reuse. Use an untimed cursor-run census to
distinguish Row versus Map, singleton/alternating versus long repeated runs,
presence versus child mode, and cold forces. Do not infer these frequencies
from a top-symbol list. No full CPU capture is required.

The current candidate trades repeated prepare calls for tagged-cursor tests
and an indexed nested loop. The old carried loop used an enumerated slice
iterator. Possible extra bounds/branch or code-layout costs are hypotheses,
not an established cause; inspect the frozen generated code only after timing
releases the serial measurement lock. If supported, test a single general
iteration/dispatch refinement in isolation, retaining whole-batch prefetch,
refusal prefix and all differential/allocation gates. No workload-name rule,
unsafe indexing or persistent run metadata is justified by these results.

## A2 scenario control complete

A2-scenarios passed at 04:55:43 UTC, and live session **35796** advanced to
A2-reads, child **27426**. Every source/binary/report receipt check passed.

j4 is still a regression signal against **both** adjacent controls:
A1/B0/A2 medians are 749.500 / 1603.584 / 764.375 us; means are
2.281533 / 2.860809 / 2.493475 ms. Relative to the geometric midpoint of
A1/A2, B0 is 2.1186x p50, 1.1994x mean, 1.2247x p90. These are descriptive
comparisons, not a confidence interval or an accepted tradeoff. B1/A3 remain.

The primary r3/r4 bracket medians are 0.8964x / 0.9324x, with means
0.8925x / 0.9307x. The earlier p3 increase is not consistent across controls:
A2 p3 mean is 8.328 us, versus A1 6.116 and B0 6.746 us. Its candidate
bracket mean is 0.9452x and median 1.0189x, so calling a p3 regression from
the initial one-sided result would be premature.

All five A2 temporal scenario controls changed together to roughly twice
their earlier baseline timings. t2 A2 p50 is 78.682 ms versus A0 35.986 ms
and A1 37.299 ms. This creates apparent ~0.65x candidate bracket ratios,
which **must not be credited to P1**: the baseline itself changed. Scenarios
do not have the read suite's per-family clock guard. Shared-host effects,
scheduling and clocks remain possible confounders, not established causes.
The 05:12 UTC host check showed AC power, no recorded thermal/performance
warning and load averages 10.08/15.60/17.84. That snapshot neither explains
the earlier samples nor establishes hard P-core residency.

Continue the original frozen alternating protocol. Investigate repeated
regressions with focused ordinary work after it finishes; do not edit code,
discard these controls, attribute their shifts to the candidate, or launch
another full trace while waiting.

## A2 reads complete — contaminated controls explicitly disqualified

A2-reads passed the process/report gates at 05:16:31 UTC. That is **not**
a clean-timing certificate. The final engine clock windows for point, range,
stats and triangle are all marked contaminated, after the built-in retry:

- point: proxy 1.553 -> 1.561; p50 0.958 us.
- range: 1.618 -> 1.615; p50 10.708 us.
- stats: 1.619 -> 1.546; p50 714.500 us.
- triangle: 3.070 -> 3.359; p50 1626.584 us.

The first three correspond to the roughly doubled baseline timings observed
in the preceding temporal scenarios. This is direct clock-proxy evidence,
not proof of which physical core executed a particular sample. The triangle
median being close to A1 does not erase its flagged window. Any ratio using
these windows is disqualified for acceptance; `p1-review.py` now marks it
with `!` inline, while preserving raw values and the clock flags. Do not
normalize by clock proxies or silently discard the controls and call a win.

The remaining five engine windows are unflagged. B0 versus its A1/A2 bracket:

| Family | p50 | mean | p90 |
|---|---:|---:|---:|
| conflict_pairs | 1.3372 | 1.0495 | 1.0195 |
| disp_probe | 1.2516 | 1.1485 | 1.0781 |
| disp_probe_d24 | 0.8939 | 0.9302 | 0.9360 |
| disp_probe_d96 | 0.8908 | 0.8688 | 0.7978 |
| disp_stream_d24 | 1.0343 | 1.1067 | 1.0399 |

Undisplaced probe remains a material regression signal: B0 mean115.127 ms,
A1 mean101.049 ms (rounded), A2 mean99.438 ms. d96's improved bracket is
partly driven by a much slower A2 control (mean127.105 ms, maximum300.699 ms).
Do not treat an unflagged proxy as proof that all shared-host noise vanished.

Session **35796** advanced without restart to B1-scenarios, child **34665**,
confirmed live. The source/binary/report hashes still validate. Let B1/A3
finish. A repeat j4/undisplaced-probe loss calls for refinement/rejection, not
another broad capture. Only if the candidate is otherwise viable should the
four contaminated cheap-read controls need focused ordinary remeasurement;
there is no reason to redo every successful broad lane just to repair them.

## B1 scenarios complete — primary gains repeat, j4's large mean loss does not

B1-scenarios passed at 05:19:56 UTC. Session **35796** advanced to B1-reads,
child **39271**, confirmed live; source and report receipts still validate.
All 34 scenario rows were reviewed for median, mean, p90 and maximum/p99.

The same candidate executable now reports j4 p50 **839.959 us**, mean
**2.335215 ms**, p90 **6.038625 ms**, maximum **8.068500 ms**. Against A2,
those are 1.0989x / 0.9365x / 0.8718x / 1.0163x. Its mean is also close
to A1's 2.281533 ms. Thus B0's roughly 20% bracket-average loss and 2.1x
median did **not** repeat at that scale. This is new evidence, not a code
fix: no source changed. A3 and focused investigation still determine whether
there is a smaller real regression, noise, or an unfavorable workload mix.

The primary heavy-join direction did repeat:

| Family | B1 p50 | B1/A2 p50 | B1/A2 mean | B1/A2 p90 |
|---|---:|---:|---:|---:|
| r3_bomb_t1 | 2.291 ms | 0.8817 | 0.8829 | 0.8799 |
| r4_bomb_t2 | 1.137664 s | 0.9203 | 0.9150 | 0.8961 |

Do not generalize that stability to every earlier apparent win. g4 B1 mean
is 3.376437 ms, versus B0 2.436951 ms; the earlier large g4 gain is
not stable. B1 t2 is back at 37.207 ms median, consistent with earlier
baselines; A2's 78.682 ms is not a legitimate denominator for claiming
a near-2x temporal speedup. p3 B1 mean6.501 us lies between A1 and A2.

The B1 point-by-ID row has a very large single maximum and a near-baseline
median. Preserve its mean/tail report instead of averaging away the event
or attributing a direct storage-lookup regression to sibling-probe routing
without evidence. B1-read clock windows and final A3 controls are pending.
The candidate remains unaccepted; no new full trace.

## B1 reads complete — undisplaced probe loss repeats

B1-reads passed at 05:25:33 UTC. All nine final engine clock windows are
unflagged; all median/mean/p90/maximum summaries and engine/oracle clock fields
were reviewed. Session **35796** advanced to A3-scenarios, child **45756**,
confirmed live. Source/binary/report checks passed again.

The undisplaced probe is the strongest remaining regression signal:

| Round | p50 ms | mean ms | p90 ms |
|---|---:|---:|---:|
| A1 | 92.736 | 101.049 | 129.691 |
| B0 | 119.139 | 115.127 | 135.236 |
| A2 | 97.712 | 99.438 | 121.329 |
| B1 | 128.266 | 129.818 | 139.071 |

B1/A2 is 1.3127x median, 1.3055x mean and 1.1462x p90. This repeats the
direction of B0's loss, unlike j4's large mean increase. No acceptance based
only on heavy-join gains. Await A3, then isolate this shape with ordinary
controls and structural/code-generation inspection if it persists.

d24 changed direction (B1 mean108.484 ms vs A2 98.899 ms); d96 remains
below the noisy A2 control (B1 mean106.083 ms vs A2 127.105 ms). Neither
is a monotonic displacement proof. B1 conflict_pairs mean29.046 us and
displaced-stream mean176.397 us are close to A2's clean windows. B1's
point/range/stats are back near the earlier clean controls; comparisons to
their contaminated A2 windows remain unusable. Triangle's B1 mean1.635 ms
also cannot rehabilitate the flagged A2 window.

### Establish the changed route before attributing the loss

The displaced query has only two atoms, Spoke(id, hub, val) and Hub(id, tag),
grouped by tag. Source audit of `binary2fj`, `factor`, `fold_split`, `gj_split`
and `PipeTables::of` narrows a critical question: the usual Hub-first plan
has a Hub-tag prefix, then Hub-id cover probing the **uncarried** Spoke root,
then a Spoke suffix scan. P1 changed only carried sibling runs. Cover choice
is dynamic, so this source derivation must be confirmed for the measured
plan and corpus, not assumed from the query name or old probe symbols.

After the measurement lock releases, use the existing `Counters::probe_batch`
and `cover_choice` observer with the validated plan's entry levels/carried
table to count actual carried-versus-root sibling work. A test observer can
derive this without adding production per-row metadata or collecting a trace.
If the changed branch is not taken, investigate code generation/layout and
shared-host measurements before tuning cursor-run grouping to this regression.
The same monomorphized function contains both branches, so an unchanged source
branch is not proof that the generated instruction/register layout is unchanged.

## All scenario rounds complete

A3-scenarios passed at 05:28:58 UTC. All 34 rows from all six rounds have
now been reviewed, including answer-count equality, central statistics and
tails. Live session **35796** advanced to its last phase, A3-reads, child
**48692**. Source and completed report receipts still validate.

The two adjacent-bracket comparisons support a consistent primary signal:

| Family | B0 bracket p50 / mean | B1 bracket p50 / mean |
|---|---:|---:|
| r3_bomb_t1 | 0.8964 / 0.8925 | 0.8962 / 0.8961 |
| r4_bomb_t2 | 0.9324 / 0.9307 | 0.9226 / 0.9209 |
| j4_five_way | 2.1186 / 1.1994 | 0.9821 / 0.9170 |
| g4_mutual | 0.7456 / 0.7431 | 0.9923 / 1.0045 |
| o4_segment_category | 0.9499 / 0.9790 | 0.9475 / 0.9745 |

r3/r4 baseline medians range by only about 4.0%/2.0%; their candidate
improvement is larger and consistent. This does not by itself accept the
patch. j4 A3 mean2.600859 ms and median956.958 us place B1 well within
the control range, while B0's unusually high median remains preserved.
Neither the large initial j4 loss nor the large initial g4 gain is a stable
whole-candidate conclusion.

Do not credit the temporal bracket ratios: they still contain A2's doubled
baseline, while A3 t2 is back at median38.510 ms. B1's point-by-ID maximum
476.083 us remains an isolated large event against A3's 0.709 us maximum;
retain it for the final acceptance review. Smaller signals worth retaining
include B1 o5 mean+4.9%, p4 mean+5.3% and r5 p90+17.8% against their
brackets. They do not justify narrow workload-specific patches.

Next: finish A3-reads, review its raw clocks and distributions, then confirm
the measured displaced plan's actual carried/root route and inspect the frozen
code generation serially. No engine source changes, benchmark restart or full
CPU capture occurred during this comparison.

## Final A3 reads — comparison terminal

Recovery session 35796 exited 0; A3-reads passed at 05:35:36 UTC. State is
`MEASUREMENTS-COMPLETE-REVIEW-REQUIRED`. All source, executable and successful
report hashes validated after completion. There is no remaining comparison
process to wait for or restart, and source freeze is released.

Undisplaced probe A3 baseline slowed to p50 125.649 ms, mean 125.327 ms and
p90 143.861 ms, close to candidate B1 (128.266/129.818/139.071 ms). B1 versus
the final A2/A3 bracket is 1.1576x p50 and 1.1629x mean; B0's bracket remains
1.2516x/1.1485x. The earlier B1/A2 +30% is not an established engine regression
of that magnitude: the baseline median range is 35%, mean range 26%.
Nevertheless this unresolved signal precludes accepting P1 from r3/r4 alone.

All A3 final engine clock windows are unflagged; triangle retried successfully.
This does not establish hard P-core residency or absence of host interference.
The four flagged A2 cheap-read controls remain disqualified. d24 changed
direction (B0 bracket mean 0.9302x; B1 1.0421x). d96's B1 bracket mean 0.9078x
still includes the noisy slow A2 control. Displaced stream B1/A3 means are
176.397/176.095 us; the earlier B0 mean 201.597 us and 1.100459 ms maximum
remain in the evidence. Percentiles at these sample sizes are descriptive,
not confidence intervals or stable population-tail estimates.

Next is structural route counting and frozen code-generation inspection,
then only focused ordinary controls as needed. No new full trace.

### Frozen width-one machine-code audit

`p1-codegen-1/` retains raw symbols/disassembly, normalized assembly and diffs
for both children/presence modes on both frozen executables. No rebuild or
profiler was used. Each containing function gained 22 instructions (88 bytes).
Both per-key callbacks have identical instruction counts and matching normal
lookup instructions/register choices/local branches: every normalized diff
is an address used to report a panic/error, not a normal-path lookup change.
Raw instruction bytes still differ for relocated external calls.

The children-containing frame grows from 304 to 320 bytes. The warm uncarried
root path has different register allocation and a few setup loads; its loop
has eight instructions per key versus nine previously (the survivors pointer
stays in a register instead of being reloaded). It still calls the same-sized
per-key lookup callback. Thus changed surrounding code/layout is real, but this
inspection does not identify extra repeated root lookup work that explains a
15–25% timing loss. Do not turn that absence into proof of no regression.

The original two exploratory objdump invocations failed on option syntax;
the successful retained driver uses `-d --disassemble-symbols=...`. All four
baseline/candidate function and callback extractions completed and hashes
matched the timing executables. The separate route observer is a test-only
temporary module, never the binary used for ordinary timing.

### Actual displaced route confirmed, cold and warm

`p1-route-1/route.log` records one cold and two warm executions using the
normal prepared-rule engine with its existing structural Counters observer.
The test opened a private byte-identical copy of the A3 measured displaced
database; source and copy data.mdb hashes remained unchanged. All 1,024
tag/sum outputs matched an independent direct sum over the seed-1 generator,
not merely the same aggregate implementation on another route.

Every pass selected Hub tag, then Hub-id cover probing the Spoke root, then
Spoke id/value suffix. Per pass: **524,288 uncarried/root probe keys, zero
carried probe keys**, in 4,100 batches; 453,241 hits. The dynamic cover choice
was Hub on all 1,024 tag groups, both cold and warm. Thus P1's new carried-run
loop does not execute in this workload. Do not tune run-grouping logic as
though it had caused repeated extra carried work here.

The test-only absolute-path hook was removed after the successful audit;
its source and logs remain ignored research evidence. No production observer
field, trace, workload change, or new timing binary was introduced. Next is a
bounded ordinary ABBA control using the frozen binaries: undisplaced probe
plus the four previously contaminated cheap controls, 32 samples each.

## Focused ABBA completed — current P1 form not accepted

`p1-focused-1` finished at 05:49:13 UTC, session 64288 exit 0. Both frozen
binary hashes, all four report hashes and the original source fingerprint
validated before the next source edit. No more timing process remains.

Undisplaced p50/mean in ms: A0 107.302/109.044; B0 144.500/144.654;
B1 167.294/222.832; A1 131.929/132.768. B0's engine clock window is
contaminated after retry (3.359 -> 2.423); it is ineligible for acceptance.
B1's endpoints are unflagged (3.258 -> 3.264), but its p90 is 447.725 ms
and maximum 628.085 ms. B1/geometric(A0,A1) is 1.4061x p50/1.8520x mean.
This remains a regression signal, not a proven causal cost of carried grouping.

All cheap control endpoints were unflagged. Candidate/geometric(A0,A1)
p50/mean: point B0 1.1441/1.1408, B1 1.0503/1.0737; range B0
1.0592/1.0622, B1 1.0677/1.0685; stats B0 1.0651/1.1219, B1
1.0468/1.0414; triangle B0 1.0384/1.0407, B1 1.0279/1.0249.
Candidate cheap windows were near proxy 3.26 while many baseline windows
were near 3.4; do not normalize them into a claimed win or excuse all losses.

A contemporaneous host snapshot showed mediaanalysisd at 90.5% CPU, two
unrelated node processes at 72.9%/44.6%, no recorded thermal/performance
warning, and 9,148 MiB swap used. VM cumulative compression/paging counters
are not paging rates and do not establish the cause of the earlier samples.
No unrelated process was stopped or reprioritized.

Decision: preserve this entire P1 candidate, but do not accept/commit it as a
performance improvement. No further identical broad timing loop is justified
right now. Test one general iterator refinement: the current indexed nested
loop adds redundant survivor bounds bookkeeping in generated code. Use an
enumerated slice iterator across cursor runs, still retaining the full-batch
prefetch, order, borrow boundary and exact first-refusal prefix. Recheck code
generation and correctness before any new scoped timing. This is a new
candidate, not a claim that source changes fixed host noise or root latency.
