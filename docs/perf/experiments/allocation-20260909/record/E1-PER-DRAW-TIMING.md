# E1 ordinary timing: saved draws, not a pooled triangle panel

The preceding goal turn made progress: `E1-EMPTY-INPUT-REVIEW.md` records
red/green construction and cancellation tests, completed correctness gates,
and verified cold-empty allocation savings. It did not establish speed.

## Matched executable construction

The standard curves panel pools four parameters and reports no raw sample
stream. E1 needs individual draws plus a warmed parameter-changing control to
price the additional run_join check. `e1_timing.rs` is a temporary ordinary
release binary, using the existing benchmark library's query/parameter
families, hand-written SQL goldens, value comparison, clock proxy, scheduler
priority and statistics implementation. No dependency or shipped CLI change.

The same external driver is compiled against the published engine and E1 in
the same isolated source path using the workspace fat-LTO release settings.
Only the two E1 production files differ. The temporary Cargo binary entry is
removed and E1's exact gate source restored before measurements. Build JSON
must identify ordinary optimized, non-test code and no alloc-counter feature.
No observer, allocation counter, full trace or concurrent compile is timed.

Candidate build attempt 1 stopped on the driver's constant-assertion lint;
no binary or measurement was produced. The assertion is now compile-time.
Candidate attempt 2 passed format, strict lint and release build, session 8456,
at 09:25:36 UTC. Driver SHA-256:
`13e2309e46788f7778394c80f16b910679da976e240531a8649971fd33ebb4d1`.
Candidate timing binary SHA-256:
`81525df743de36fa6878f52f5a7014f2eb864e77cadc01d22799e3f6954541c9`.
Its engine features are exactly `[collision-probe]`.

Baseline build attempt 1 passed format, strict lint and release build, session
12615, at 09:27:06 UTC. Its timing binary SHA-256 is
`1651f0b2496b92dc36964eaac2dfe3f1b7de6e5896754968199215257698b09b`.
Driver and engine-feature checks match candidate 2. The baseline build verified
that both production files exactly match published HEAD. After freezing the
binary, the two E1 hunks were restored and the temporary Cargo entry removed.
The ordinary source fingerprint matches E1 gate 1 exactly again:
`e44e04132d30992ab094d61c93d620207175e1390006046e53733c16fb52cd55`.
Other candidate worktrees remain untouched.

## Fixed comparison, no repeat-until-win protocol

`e1-timing.py` runs one fixed per-case ABBA panel. Eight cases alternate the
four saved triangle draws with the four point controls. Each process verifies
all four family draws against the existing hand-written SQLite golden before
any timing. It works on a private byte-identical copy of the same saved corpus
used for the allocation audit, with read-only SQLite; both original and copy
data hashes must remain unchanged. No corpus generation is performed.

- Cold/second: fresh DB reopen and preparation excluded from each timer;
  two discarded pairs then 16 first/second-execution pairs for the target.
  Parameters are built outside both timers, as in the existing cold panel.
- Rotating (alternating-parameter warmed): alternate `(target+1)%4` outside
  the timer, then time the target; eight warmup cycles plus 64 samples. This
  exercises changed bindings while reusing views/tables. The alternate is
  checked too; this is NOT avoidance of an answer cache.
- Memoized (same-target warmed): eight warmup calls then 64 batches of 16 identical-target calls,
  using the benchmark's existing maximum batching size to reduce timer noise.
  Rotating/memoized include the existing parameter helper inside the timer.

All individual calls except intermediate memoized-batch calls are value-checked
after timing; every batch's final reusable output is checked. These unmeasured
checks can warm output memory, symmetrically. Keep raw unsorted elapsed values,
per-operation integer statistics and all clock flags. Cold and second share
one bracket; rotating and memoized each have another. No retry, deletion,
frequency normalization, significance or Pi claim. macOS QoS is verified
steering, not hard P-core affinity.

## Completed comparison and scope

Comparison 1 in `e1-timing-1/`, session 63397, ran 09:27:38–09:28:21 UTC
on September 9, exit 0. No source edits or compilation overlapped measurements.
All 32 processes passed their four independent handwritten SQLite goldens and
timed-output checks. All original/private corpus hashes remained unchanged.
`e1-timing-review-1/review.log` completed 09:28:39 UTC, exit 0: all 128
distributions independently recomputed from raw samples and all 96 independent
clock brackets reviewed. Eight brackets were flagged; zero retries, no dropped
samples and no frequency normalization.

Empty triangle draw `[500,500)` round medians (A published, B E1):

| Window | A0 / A1 | B0 / B1 | Geometric round-center B/A p50 |
| --- | ---: | ---: | ---: |
| Cold | 9.104 / 8.558 ms | 4.205 / 4.178 ms | 0.474867 |
| Second execution | 1.489 / 1.489 ms | 709 / 709 ns | 0.000476205 |
| Alternating-parameter warmed | 1.485 / 1.498 ms | 666 / 666 ns | 0.000446628 |
| Same-target warmed, batched | 1.515 / 1.507 ms | 484 / 466 ns | 0.000314347 |

Cold mean ratio is 0.470810. This is a strong, repeatable local empty-case
benefit, not a database-wide speed claim. The cold path still builds images
and filter buffers; the already-bound empty path avoids the useless join.

Nonempty triangle B/A p50 / mean, same round-center convention:

| Draw | Cold | Second | Alternating-parameter | Same-target |
| --- | ---: | ---: | ---: | ---: |
| `[1,6)` | 1.0100 / .9694 | 1.0033 / .9695 | 1.0056 / 1.0007 | .9970 / .9904 |
| `[167,172)` | .9964 / .9877 | 1.0087 / 1.0171 | .9914 / .9934 | 1.0038 / 1.0025 |
| `[333,338)` | .9507 / .9746 | .9966 / .9918 | .9835 / .9737 | .9994 / 1.0066 |

These centers are close or favorable, but tails are not uniformly improved:
draw 2 same-target max ratio is 1.2767. Keep the raw distributions, not only
the central summary. No significance claim from two rounds per variant.

Point controls B/A p50 / mean:

| Draw | Cold | Second | Alternating-parameter | Same-target |
| --- | ---: | ---: | ---: | ---: |
| 0 | 1.0000 / 1.2807 | 1.0000 / 1.0543 | 1.5165 / 3.1849 | 1.4335 / 1.6240 |
| 1 | 1.0399 / 1.0774 | .9989 / 1.0155 | .9542 / .9953 | 1.0199 / 1.0154 |
| 2 | .9918 / 1.0697 | 1.0000 / 1.0571 | 1.0983 / 1.0161 | 1.0108 / 1.0001 |
| 3 | .9910 / .9685 | .9494 / .9814 | .9438 / .9819 | .9926 / .9840 |

Point 0 B0 has severe clock contamination in all three brackets, with proxy
values down to 1.16–1.43 GHz and a large rotating tail. Its rotating median
is 959 ns versus 417 ns in the other rounds. It remains in the comparison.
Point 1 cold is adverse without a clock flag (+3.99% p50 / +7.74% mean).
Point 2 rotating medians move 417 to 458 ns (one timer tick); mean is +1.61%.
Do not assert a precise causal 9.8% regression from that quantized median.
Point 2 cold mean/tails are also adverse, with a flagged baseline bracket.
Point queries do not enter the modified Free Join path; that separates the
mechanism but does not erase controls or establish global performance green.

All eight unique flagged brackets (cold also covers second execution):

1. triangle-0-A0 cold
2. point-0-B0 cold
3. point-0-B0 rotating
4. point-0-B0 memoized
5. point-1-B0 memoized
6. point-2-A0 cold
7. triangle-3-A0 rotating
8. triangle-3-B1 cold

## Terminology correction and disposition

The frozen driver and earlier commentary loosely described alternating
parameters as preventing a result/answer-cache hit. That assumption was wrong.
`api/prepared/execute.rs::run_rules` resets the sink and invokes rules each
time; resolved Free Join rules call `run_join` each execution. The benchmark's
"memoized" label means warmed query structures/bindings, not cached answer
rows. Nonempty same-target execution still takes about 1.6 ms. Use
**same-target warmed** and **alternating-parameter warmed** in interpretation.
Do not rewrite the frozen driver/build/run evidence to fix its old comment.

E1 has complete correctness gates, verified cold-empty allocation savings and
a strong scoped timing result. It is still isolated; no full-suite/universal
speed acceptance, Pi qualification or saved-trace exhaustion is established.
Do not repeat this panel merely to get prettier controls. Continue studying
the remaining retired construction arenas and earlier bind/filter work from
existing evidence. No new full trace, full benchmark, commit, push or release.
