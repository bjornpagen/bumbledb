# P2 ordinary ABC/CBA — neither candidate accepted

Completed 2026-09-09 06:55:16 UTC; session 53558 exit 0. All six rounds
completed 12 joins/OLAP scenarios (24 samples, 8 warmups) and 5 read controls
(32 samples, batch one). Source and binary hashes stayed fixed; independent
scenario answers and private binary-bound read verification held throughout.

A=published baseline, B=initial linear getter (gate 3), C=compact route cache
with per-batch take/restore (gate 4). Complete reports are in p2-comparison-2.
Each ratio below divides geometric centers of the two rounds. Lower is faster;
these are descriptive summaries, not confidence intervals. Scenario statistics
pool rotating parameter sets and do not retain per-parameter raw samples.

| Family | C/A p50 | C/A mean | C/A p90 | C/A max | C/B p50 |
|---|---:|---:|---:|---:|---:|
| j1_filmography | 1.0696 | 0.9610 | 0.9500 | 0.9181 | 1.2647 |
| j2_costars | 0.9473 | 0.9795 | 0.9850 | 0.9855 | 0.9638 |
| j3_keyword_kind | 0.9748 | 0.9999 | 0.9962 | 1.0238 | 1.0549 |
| j4_five_way | 0.6227 | 0.4621 | 0.4476 | 0.3167 | 1.0115 |
| j5_country_rollup | 0.9643 | 0.9568 | 0.9518 | 0.9099 | 1.0278 |
| j6_keyword_neighborhood | 0.8261 | 0.9279 | 0.9907 | 0.7732 | 0.9794 |
| o1_revenue_by_region | 0.9619 | 0.9170 | 0.8543 | 0.6271 | 0.9700 |
| o2_category_window | 0.9899 | 0.9815 | 0.9808 | 1.0130 | 0.9714 |
| o3_promo_split | 1.0298 | 1.0453 | 1.0460 | 1.2277 | 0.9862 |
| o4_segment_category | 0.9294 | 0.9365 | 0.9554 | 0.9537 | 1.1091 |
| o5_store_extremes | 1.0070 | 1.0783 | 1.0665 | 2.1571 | 0.9815 |
| o6_brand_drill | 0.9035 | 0.9121 | 0.8559 | 0.8789 | 0.9504 |
| point | 0.9542 | 0.9641 | 0.9581 | 0.8523 | 0.9542 |
| range | 0.9877 | 0.9912 | 0.9920 | 1.0038 | 1.0208 |
| stats | 0.9674 | 0.9498 | 0.8958 | 0.8312 | 1.0027 |
| triangle | 0.9646 | 0.9618 | 0.9399 | 0.8883 | 1.0042 |
| disp_probe | 1.1479 | 1.1109 | 1.1260 | 1.1651 | 0.9388 |

## Interpretation and open controls

- C o4 medians are 24.299375/24.334000 ms. B repeats 21.847250/22.002584 ms:
  C/B is 1.1091 p50, 1.1099 mean and 1.1655 p90. C fixed the wide-row cliff
  but did not retain B's main narrow-leaf result. A medians are 27.604250 and
  24.799959 ms, so C's apparent ~7% baseline gain partly includes the high A0.
- j5 favors both candidates over A but C trails B by about 2.8% p50/2.6% mean.
- j4 A0 has a 76.078 ms maximum and 11.982 ms mean; A1 is 5.859/2.030 ms.
  Its apparent 38-68% C improvement is NOT an optimization claim. C0/C1 also
  vary (0.542/0.910 ms p50), and C/B centers remain about 1% apart. The source
  audit identifies this as a ProjectionSink query, not an aggregate route.
- j1 C0/C1 p50 667/458 ns versus A 458/583 ns; its mean/tails do not show a
  matching loss. j2/j3/j6/o6 mixed-parameter medians cannot establish uniform
  speedups. j6 A0 has a long tail, so do not credit its large p50 ratio alone.
- o1/o2 centers favor C, but o1 baseline pauses amplify mean/tail gains. o2's
  four unequal windows make its pooled p50 unstable. o3 C mean is 4.5% above
  A, with C1 max 474 us versus a 340 us median; retain this concern. o5 C
  medians are near A, but C maxima of 1.087/1.247 ms lift mean/tails against
  A maxima about 0.54 ms. Do not hide these candidate pauses either.
- Point/range are close in absolute time (point C medians 458/417 ns;
  range C 5.167/4.917 us). Stats and triangle centers are modestly lower,
  but A0 stats is flagged and C/B stays close. No claim that P2 accelerated
  projection/point access or removed warm per-row allocations follows.
- disp_probe is an explicit unresolved losing control: C medians
  100.926916/101.398833 ms versus A 92.692125/83.794875 ms. A0 is flagged,
  A1 unflagged; C p50/mean centers are +14.8%/+11.1%. B is also slower than
  A, and earlier baseline/candidate directions were unstable. Its constant
  outer grouping/suffix scan is not the per-row copy route. Preserve the
  result and isolate it if necessary; don't repeat broad loops until it wins.

Only A0 stats and disp_probe have contaminated read boundary windows (after
the harness's existing built-in retry); all remaining bumbledb read windows
are unflagged. There is no scenario clock guard. A read-only host observation
at 06:47:30 UTC found active Spotlight/PDF indexing and substantial swap use;
see P2-COMPACT-REVIEW.md. No global host settings or processes were changed.
This does not assign causality for individual timings or exclude internal
pauses from unflagged windows. No normalization or manual retries were used.

## Disposition

Neither B nor C is accepted. B has a demonstrated wide O(K*W) failure; C
removes it but gives back narrow-leaf performance and retains mixed controls.
The next experiment keeps O(1) routing and eliminates taking/restoring its
Vec on each distinct batch by borrowing the lookup at each reader use.
An added regression checks typed overflow, no partial output, route ownership,
reset, changed key layout, release and zero-key all-outer refill. The focused
comparison must include singleton as well as 128-row batches, with staged,
taken-cache and in-place readers, plus independent outputs. No natural-width
special case or arbitrary threshold is introduced. No new full trace is
needed, authorized or run; deferring either form is not trace exhaustion.
