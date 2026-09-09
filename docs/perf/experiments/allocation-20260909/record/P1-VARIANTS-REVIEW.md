# P1 ABC/CBA disposition — defer both forms

Completed 2026-09-09 06:13:03 UTC, session 9080 exited 0.
All six rounds completed 18/18 scenarios with 24 samples, eight warmups,
unchanged rotating parameter streams, independent scenario oracles, and equal
answer counts. C separately passed its full 2,879-case oracle. The final
driver source fingerprint and all executable hashes matched. No profiler
or new full CPU trace was involved.

A is published baseline; B indexed adjacent-cursor reuse; C iterator reuse.
Each ratio below uses the geometric center of that variant's two rounds,
divided by A's two-round geometric center. Smaller is faster. These are
descriptive shared-host ratios, not confidence intervals or causal estimates.
There is no per-scenario clock guard. Rotating-parameter percentiles are not
homogeneous-query samples; p99 at 24 draws is just the maximum.

| Query | p50 B/A / C/A | mean B/A / C/A | p90 B/A / C/A | maximum B/A / C/A |
|---|---:|---:|---:|---:|
| j1_filmography | 0.9634 / 0.9655 | 0.9767 / 0.9610 | 0.9734 / 0.9531 | 0.9640 / 0.9561 |
| j2_costars | 1.0745 / 1.0002 | 1.1653 / 0.9797 | 1.1496 / 0.9795 | 1.2674 / 0.9723 |
| j3_keyword_kind | 0.9745 / 0.9489 | 0.9881 / 0.9723 | 0.9718 / 0.9751 | 1.0682 / 0.9555 |
| j4_five_way | 1.4888 / 1.1789 | 1.1256 / 1.0646 | 1.1329 / 1.0553 | 1.2230 / 1.1580 |
| j5_country_rollup | 1.0138 / 1.0218 | 1.0110 / 1.0209 | 1.0152 / 1.0317 | 0.9780 / 1.0626 |
| j6_keyword_neighborhood | 0.8049 / 0.7976 | 0.9425 / 0.9319 | 0.8991 / 0.8904 | 0.9419 / 0.9025 |
| o1_revenue_by_region | 1.0489 / 0.9816 | 1.0308 / 0.9473 | 1.0769 / 0.9216 | 0.9693 / 0.6780 |
| o2_category_window | 1.1050 / 1.1181 | 1.0650 / 0.9917 | 1.3072 / 1.0183 | 1.1781 / 0.9360 |
| o3_promo_split | 0.9962 / 0.9789 | 0.9851 / 0.9814 | 0.9879 / 0.9840 | 0.9083 / 1.0351 |
| o4_segment_category | 1.0631 / 1.0173 | 1.1043 / 1.0308 | 1.1601 / 1.0981 | 1.4704 / 1.1077 |
| o5_store_extremes | 1.0107 / 0.9787 | 1.0276 / 0.9701 | 1.0680 / 0.9892 | 1.1252 / 0.7872 |
| o6_brand_drill | 0.9179 / 0.9171 | 0.9407 / 0.9366 | 0.9300 / 0.9139 | 0.9293 / 0.9156 |
| r1_wash_ring | 1.0335 / 0.9465 | 1.0706 / 0.9643 | 1.1171 / 0.9523 | 1.0834 / 0.9223 |
| r2_temporal_ring | 1.0031 / 0.9892 | 1.0157 / 0.9678 | 1.0253 / 0.9528 | 1.0456 / 0.9890 |
| r3_bomb_t1 | 0.9185 / 0.9006 | 0.9448 / 0.9077 | 0.9240 / 0.9123 | 1.3929 / 0.9299 |
| r4_bomb_t2 | 0.9366 / 0.9247 | 0.9519 / 0.8482 | 0.8669 / 0.6717 | 0.9025 / 0.4959 |
| r5_reciprocal | 1.0591 / 1.0046 | 1.0640 / 0.9663 | 1.0655 / 0.8907 | 1.0930 / 0.8195 |
| r6_two_path_count | 1.0533 / 1.0470 | 1.0454 / 1.0425 | 1.0093 / 1.0702 | 1.0360 / 1.0420 |

## Interpretation

- C retains the main r3/r4 signal: about 9.9% and 7.5% lower descriptive
  p50s than bracketed baseline. C versus B p50s are about 1.9% and 1.3%
  lower, respectively: this is a modest refinement, not a new large win.
- r4 baseline medians are stable at 1.218183 / 1.218442 seconds. C medians
  are 1.122489 / 1.130659 seconds. A0 nevertheless has large pauses
  (mean 1.442375 s, p90 2.284234 s, max 4.319387 s), as does B0
  (max 3.568754 s). Do not credit C's 50% apparent maximum reduction to
  cursor reuse or use that tail to inflate the central result.
- j4 is unresolved: C0/C1 medians are 1.027125 / 0.544917 ms, while
  A0/A1 are 0.601458 / 0.669583 ms. The mixed parameter stream and round
  instability do not establish a clean pass. B's two medians both lose
  here (0.903875 / 0.987583 ms).
- r6 has a smaller but consistent concern: C p50/mean centers are about
  4.7%/4.2% above baseline, and its p90 center is about 7.0% above baseline.
  o4 is also not a C win (p50 +1.7%, mean +3.1%). Do not hide these behind
  favorable geomeans or unrelated noisy controls.
- C materially improves the j2 signal relative to B in these rounds, but
  the previously unresolved ordinary root-read controls have not been
  rerun for C. Saved route proof still shows zero carried keys in the
  displaced root fixture. This comparison cannot clear that control.
- Tiny mixed-query medians and the o2 median/mean disagreement merit
  restraint, not additional identical full-roster loops.

## Decision and next work

**Neither B nor C is performance-accepted.** Preserve all patches, binaries,
raw reports, prior failures, route observations and code-generation evidence.
Defer P1 and restore only the experimental production probe hunk to baseline;
retain its useful store-free differential/allocation regression test.

No more identical broad P1 timing loops are selected. P1 remains an open
saved-evidence lead (possible root/carried code-generation isolation requires
a separate justified experiment); deferral is not trace exhaustion. Proceed
with standalone P2 direct aggregate input reads, without P1 bundled in the
candidate. See P2-IMPLEMENTATION-SKETCH.md for the smaller first experiment
and the quantified reason not to bundle new dedup-routing metadata.
