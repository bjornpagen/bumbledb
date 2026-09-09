# Q2 ordinary timing: cold savings, unresolved warm losses

Q2 is NOT performance-accepted. This comparison is against frozen Q1, not
published v1.1.0. Q1 itself remains unaccepted. Memory reduction alone does
not settle the measured warm regressions, and the numeric panel does not
cover the production text memo (see Q2-CONSUMER-AUDIT.md).

## Identities and measurement

Both builds used the same comparison checkout path, unchanged q1_timing.rs,
separate fresh CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR, optimized ordinary
engine features exactly [collision-probe], and no allocation observer mounts.
Format, strict driver lint and build passed. The comparison tree is restored
clean at published b02a641e087364ec09c161a97c67c87cf626e6b2.

- Q1 executable: 44593bfb044e0330120229828f9c22b78ca987724721a3928d2ff587c691e8fc
- Q2 executable: f9369dda9fbd15b5485e4eb040f3407f92adfb72532f43166798b1b3378a1e85
- Q2 source: dc78c0d7123b10edc7f9df4751a9d3a4792b65e4a428458f639c8a9d6e936f3c

The rebuilt Q1 executable differs from its older timing executable; do not
join the old and new panels as one matched measurement or assume bitwise
cross-build reproducibility.

q2-timing-1 completed September 9 11:57:12–12:00:12 UTC, exit 0. The predeclared
18-case ABBA panel ran 72 processes, 16 prepare/first/combined/second samples
and 320 samples for each warm mode per process. Every exact numeric SQL check
passed. q2-timing-review-1 independently verifies 432 distributions, 50,688
raw values and 216 clock brackets. All 46 flagged brackets remain; proxy
endpoints span 1.3759–3.4096 GHz. No retries, filtering or normalization.
QoS requests are not a claim that macOS guarantees physical P-core affinity.

## Every case, including controls

Percentages are (Q2/Q1 - 1), using geometric centers of the two round ratios.
Each cell is p50 / mean; negative is lower elapsed time. Bracket labels in
parentheses identify flagged rounds for that mode; a dash means no flag.
These are descriptive centers, not confidence intervals or causal estimates.
Preparation/first/second and every tail are also retained in comparisons.tsv.

| Case / target rows | Prepare + first | Same-target warm | Alternating warm |
| --- | --- | --- | --- |
| DNF groups / 500 | -16.91 / -17.03 (A1) | +7.00 / +4.59 (-) | -27.71 / -36.01 (B0,A1) |
| Union / 100000 | -34.17 / -33.83 (A0,B1,A1) | -10.40 / -12.43 (A0,B1) | -8.10 / -26.09 (A0,B0) |
| Hashed groups / 8192 | -4.98 / -6.08 (-) | +0.96 / +0.88 (-) | -11.04 / -14.30 (-) |
| Written-union groups / 500 | -15.14 / -14.23 (-) | +10.18 / +7.09 (B1) | +12.66 / +10.67 (-) |
| Computed / 100000 | -7.82 / -7.76 (B0) | +5.90 / +2.13 (B1) | +5.10 / +4.09 (-) |
| Hashed groups / 100000 | -13.90 / -11.73 (A1) | -5.44 / -8.04 (A1) | -3.72 / -6.86 (A0,B0) |
| Pairs / 500 | -4.42 / -2.95 (A1) | +0.34 / +0.02 (-) | +1.07 / +0.54 (B0) |
| Computed / 0 | -4.97 / -5.00 (-) | -12.31 / -3.00 (B1) | +0.15 / +1.01 (-) |
| Computed / 5 | -1.46 / -1.86 (B1) | +0.00 / +1.37 (-) | +3.62 / +0.80 (-) |
| Hashed groups / 5 | +2.39 / +3.21 (-) | +0.00 / +2.10 (-) | -5.24 / -2.76 (B0,B1) |
| Reach / 6 | -3.80 / -2.37 (B1) | +0.91 / -0.66 (B1,A1) | +0.87 / -0.15 (B1,A1) |
| Saved triangle draw 0 / 5 | -0.01 / +0.76 (-) | -0.40 / -0.28 (A0) | -0.74 / -0.81 (-) |
| Saved triangle draw 3 / 0 | +0.03 / -0.37 (A0) | +0.10 / -0.17 (A1) | -0.39 / -0.69 (A0,B0,B1) |
| Saved range draw 0 / 2000 | -1.47 / -14.03 (A0,B1,A1) | -7.39 / -8.32 (A1) | -10.16 / -8.70 (A1) |
| Saved range draw 2 / 2000 | -1.84 / -3.89 (-) | -7.52 / -6.70 (B1) | -7.81 / -7.24 (A0,B1) |
| Saved point draw 0 / 1 | -9.93 / -5.43 (A1) | +0.00 / -3.03 (A0,A1) | +0.00 / -2.77 (A1) |
| Saved point draw 3 / 0 | -1.48 / -1.47 (-) | +0.00 / -2.82 (-) | -5.62 / -3.10 (-) |
| Dense / 3 | +1.10 / +7.46 (-) | -3.42 / -3.39 (-) | -3.20 / -7.02 (-) |

The strongest adverse results are not just flagged slow runs:

- DNF500 same-target warm medians A0/B0/B1/A1: 3.391/3.703/3.810/3.633 ms.
  Both comparison directions lose, all brackets unflagged.
- Written-union500 alternating: 3.216/3.706/3.681/3.342 ms. Both lose, all
  unflagged. Its same-target direction also loses twice, but B1 is flagged.
- Computed100k alternating: 3.256/3.388/3.122/2.941 ms. Both lose despite
  host/order drift, all brackets unflagged.

Written-union500's cold gain is more credible than union100k's apparent 34%
gain: the former has all cold brackets unflagged and both directions improve;
the latter has three flagged cold brackets. DNF500's cold directions also
improve but A1 is flagged. These gains fit the separately reconciled reduction
in growth/payload traffic; no exact cycle attribution follows.

Groups8192 warm same-target directions disagree, so its +0.96% center is not
a demonstrated general regression. Its alternating center improves twice,
but A1/A0 drift is +14.8% in median and +20.0% in mean despite no clock flag.
A threshold flag is not a complete noise detector.

Range warm centers improve even though that execution path bypasses the
changed map. Treat these as host/code-layout sensitivity, not a map-speed
benefit. Empty computed warm medians 333/292/292/333 ns yield -12.31%, while
means change only about -3%; 41 ns quantization matters. Present point warm
medians are all 417 ns; absent point same-target all 333 ns. Tiny/dense/point
percentages must not be promoted to headline speedups. Five-group combined
is adverse +2.39%/+3.21%; dense combined +1.10%/+7.46%, also retained.

## Tails and rollover

warm-tails.json contains all 144 warm distributions' top indices, chronological
blocks and neighboring samples. DNF alternating A1 has median drift 2.45x,
a 1.3759 GHz endpoint and maximum 80.944 ms. Its apparent gain is NOT a credible
warm win. Unflagged groups8192 B0 same-target maximum 3.19 ms at index 243
also stays; do not remove tails simply because the clock proxy passed.

Predicted triangle same-target rollover index 248 is not the dominant tail:
A0/B0/B1/A1 observations are 1.736541/1.781792/1.630875/1.622500 ms,
respectively 7.58/11.21/0.51/-0.06% from each median and descending ranks
21/12/101/167. All neighborhoods remain. An index coincidence alone does not
prove the reset caused those individual timings.

## Next work, not acceptance

Q2-INSERTION-CODE-REVIEW.md follows the exact arity-two scalar and bulk
branches in the frozen binaries. Warm allocation tuples are unchanged: the
next discriminator concerns per-row lookup/publication, not allocator traffic.
Keep Q2 frozen; refine in a separate candidate, then gate correctness and
inspect generated ordinary code before timing a targeted affected/control
panel. Do not rerun this identical panel for better numbers.

ResolveMemo holds real mutable string ranges. Numeric tests do not replace
saved-corpus text-output controls for it. No generic representation acceptance
until those semantics/owners/cold-and-warm costs are covered. No RSS, peak
memory, Pi qualification, full-trace permission or release readiness claimed.
