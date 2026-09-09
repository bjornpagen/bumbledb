# P2 initial ordinary ABBA — review, not acceptance

Completed 2026-09-09 06:28:56 UTC; session 34694 exited 0.
All four rounds finished 12/12 joins/OLAP scenarios (24 samples, 8 warmups)
and 5/5 read families (32 samples, batch one), with unchanged source/binary
hashes, independent scenario oracles and private binary-bound verified read
corpora. All 20 bumbledb read clock windows are unflagged; a clean boundary
proxy does not rule out interior pauses or smaller clock differences.

Ratios use geometric centers of the two candidate rounds divided by the two
baseline rounds. Smaller is faster. Descriptive, not confidence intervals.
Mixed-parameter scenario medians are not homogeneous queries.

| Family | p50 B/A | mean B/A | p90 B/A | maximum B/A |
|---|---:|---:|---:|---:|
| j1_filmography | 0.9532 | 0.9868 | 0.9853 | 0.9876 |
| j2_costars | 1.0000 | 0.9934 | 0.9913 | 0.9957 |
| j3_keyword_kind | 0.8828 | 0.8918 | 0.8680 | 0.8371 |
| j4_five_way | 1.1039 | 1.0508 | 1.0363 | 1.1192 |
| j5_country_rollup | 0.9548 | 0.9564 | 0.9392 | 1.0147 |
| j6_keyword_neighborhood | 0.9950 | 0.9847 | 0.9867 | 0.9770 |
| o1_revenue_by_region | 1.0067 | 1.0060 | 1.0228 | 1.0057 |
| o2_category_window | 1.0657 | 1.0039 | 1.0024 | 1.0299 |
| o3_promo_split | 1.0016 | 0.9934 | 0.9678 | 0.9457 |
| o4_segment_category | 0.9281 | 0.9271 | 0.9274 | 0.9556 |
| o5_store_extremes | 0.9765 | 0.8635 | 0.6977 | 0.3618 |
| o6_brand_drill | 0.9687 | 0.9692 | 0.9615 | 0.9340 |
| point | 1.0480 | 1.0410 | 1.0448 | 0.9991 |
| range | 0.9917 | 0.9998 | 1.0002 | 1.0995 |
| stats | 0.9581 | 0.9287 | 0.8553 | 0.6989 |
| triangle | 1.0023 | 1.0007 | 0.9976 | 0.9903 |
| disp_probe | 0.8416 | 0.8329 | 0.8833 | 0.9231 |

## What repeats

- o4: A0/A1 p50 24.362083/24.103750 ms; B0/B1 22.718667/22.262333 ms.
  The p50, mean and p90 centers are each about 7.2% lower. This supports
  continuing P2's direct-input experiment.
- j5: A0/A1 4.319791/4.210333 ms; B0/B1 4.058542/4.085375 ms.
  About 4.5% lower median center, with similar mean/p90 direction.
- o1/o3 and triangle are close on centers; range mean is essentially equal.
- j3's improvement and tiny mixed medians should not replace the main
  trace-selected evidence.

## Concerns and limits

- j4's high B0 (p50 0.916208 ms, mean 2.272267 ms) does not repeat in B1
  (0.513292/2.025805 ms). A0/A1 are 0.719542/0.536375 ms medians and
  2.058224/2.025619 ms means. Its aggregate ratio still loses (~10.4%
  p50/~5.1% mean). Preserve this concern; neither call it clean nor repeat
  identical broad ABBA loops hoping the one loss disappears.
- o2's rotating-parameter median is ~6.6% above baseline but its mean is
  +0.4% and p90 +0.2%. Inspect the parameter mixture before attributing this
  to a uniform query slowdown.
- Point's p50/mean centers are ~4.8%/4.1% above baseline; actual medians are
  417/458 ns for A and 458/458 ns for B. Boundary clocks differ within the
  unflagged range. Do not frequency-normalize these observations into a win.
- Large o5 and stats tail/mean wins are partly baseline pauses: o5 maxima
  are ~1.478 ms in both A rounds versus ~0.52 ms medians; A1 stats reaches
  702.583 us versus its 362.166 us median.
- Displaced root p50 B0/B1 is 96.650042/133.109708 ms against baseline
  129.673750/140.072250 ms. Its large first-round win is unstable. The
  earlier route audit says this query has a constant outer grouping key
  with a suffix scan; do not attribute all of its ratio to the changed
  row-wise fold without checking which aggregate route executes.
- No Pi qualification, RSS result, per-row allocation reduction, or proof
  that all other scenarios improve follows from this subset.

## Next bounded discriminator

The wide witnessed-row direct getter has a source-level O(K*W) lookup concern.
p2_wide.rs prepares a same-process staged-control/direct ABBA over nine
key/group widths. It shares the new numeric helper intentionally, to isolate
copying versus source lookup; it is NOT a substitute published-baseline
binary. Every case has an independent exact integer group/count/sum result,
warm shape capacity, and reset per execution so no distinct witness licenses
replaying bindings within one fold. It keeps all boundary clock readings and
raw sample blocks, without retrying or normalizing them away.

Compile and freeze that test serially after this completed comparison, using
a temporary test-only hook; preserve its patch, executable hash and logs, and
remove the hook afterward. P2 remains unaccepted pending this discriminator
and a disposition of its controls. No new full CPU trace is authorized.
