# Q1 ordinary timing: large preparation saving, real first-use costs

Status: **ordinary panel complete; Q1 remains isolated and not performance-accepted**.
This advances the saved-evidence investigation without a full trace, full
benchmark, engine edit, commit, push or release. Correctness and allocation
results in Q1-DEMAND-GROWTH-REVIEW.md and Q1-BROADER-CONTROLS.md remain valid.
The saved evidence is not exhausted.

## Frozen builds and verification

Published baseline and the exact Q1 gate-2 candidate were built from the SAME
q1-control-baseline-source checkout, using the same external q1_timing.rs and
byte-identical q1_control_cases.rs. Each build used fresh, phase-specific
CARGO_TARGET_DIR AND CARGO_BUILD_BUILD_DIR. Fresh engine artifacts, ordinary
release opt-level 3/no debug assertions, and exactly [collision-probe] engine
features were verified. There is no allocator or CPU-profile instrumentation.

- Baseline build 1 failed strict lint on numeric literal formatting in the
  shared fixture. It remains INCOMPLETE with its original output preserved.
  The driver now scopes its lint exception to the frozen fixture; fixture
  content is unchanged. Its untimed numeric checker avoids per-row allocations.
- Baseline build 2 completed 11:19:54 UTC, format/lint/build passed.
  Executable SHA-256:
  f49101f7131295c8a141d612bc7a5d872214b77591ce39ab954479d429990dbc.
- Candidate build 1 (session 61664) completed 11:24:00 UTC, format/lint/build
  passed. Executable SHA-256:
  73ba3f1dd792cd07f211cc5b0a8941b2ecd68c1f900b2d1d9970d24121ccbcaa.

The comparison source and temporary Cargo mount were restored to a completely
clean published checkout BEFORE timing. The Q1 source fingerprint remains
9f87b492b1248093d241a0217b2cb089db08d8ed82a6f1f72a5fce3a43d6b418.
All six prior candidate identities are unchanged.

q1-timing-1 (59889), September 9, ran 2026-09-09T11:24:28.288893+00:00 to
2026-09-09T11:27:39.263291+00:00, exit 0: **112 processes, 672 distributions and
78,848 retained raw values**. q1-timing-review-1 completed 11:27:44 UTC, exit 0.
It independently recomputed every integer mean and nearest-rank percentile,
checked all 336 independent clock brackets and all paired joint intervals.
Every timed operation passed an exact numeric SQLite-result check after
timing. Input databases, shared dependencies and executable hashes still match.

## Protocol

One fixed A0/B0/B1/A1 sequence for each of 28 predeclared case/draw pairs.
Saved triangle, point and range use original parameters and handwritten
golden SQL. Broader controls include small and large computed/union output,
hashed/dense groups, high-input-cardinality DNF/written-union aggregation,
named interiors and recursion. No favorable-only selection, retries, clock
normalization or dropped samples.

Cold: two discarded pairs followed by 16 fresh DB-reopen samples, recording
prepare, first execution, their actual joint elapsed interval, and second
execution. Open, AST/bind-array/empty-answer construction are excluded; fresh
WorkContext creation is inside prepare/read, matching the ordinary convention.
The combined distribution is NOT a sum of separate percentiles.

Each warm mode uses a separate prepare, eight warmups and 320 individually
timed target calls. Alternating mode executes/checks the next fixture draw
outside the target timer. Warm means re-execution, not cached final answers.
One non-retrying clock bracket surrounds the cold group, one each warm group.
The untimed checker sorts one numeric row buffer; it can still affect caches.
Reopened DB is not an OS-cold-cache claim.

macOS user-interactive QoS is verified steering, not hard P-core affinity.
There are **26 flagged brackets out of 336**, with proxy values ranging from
2.5304 to 3.5072 GHz. No flags or slow samples were removed. Endpoints cannot
exclude interference inside a window. These are shared-Mac ordinary timings,
not RSS, peak memory, full-suite or 512 MB Pi Zero 2 qualification.

## All case centers

Each entry is descriptive candidate/baseline geometric round-center
**p50 / arithmetic mean**. Ratios below 1 favor the candidate. These are not
confidence intervals or causal estimates. A draw index names the frozen
fixture, not its numeric parameter: see the metadata in review STATE.json.

| Case / draw index | Prepare | First | Combined | Second | Same-target warm | Alternating warm |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| saved_triangle-0 | 0.0273 / 0.0295 | 1.0224 / 1.0205 | 0.9223 / 0.9160 | 1.0093 / 1.0202 | 1.0132 / 1.0159 | 1.0020 / 1.0030 |
| saved_point-0 | 0.8564 / 0.8631 | 0.8790 / 0.8498 | 0.8708 / 0.8598 | 0.9261 / 0.9385 | 1.0000 / 1.0098 | 0.9494 / 1.0022 |
| saved_range-0 | 0.2927 / 0.3448 | 1.0035 / 1.0072 | 0.9961 / 1.0003 | 1.0286 / 1.0707 | 1.0307 / 1.0274 | 1.0219 / 1.0208 |
| saved_triangle-1 | 0.0218 / 0.0250 | 0.9990 / 0.9915 | 0.8974 / 0.8901 | 1.0021 / 0.9987 | 1.0012 / 1.0053 | 1.0004 / 1.0017 |
| saved_point-1 | 0.7865 / 0.7871 | 0.8625 / 0.8556 | 0.8038 / 0.8036 | 0.9225 / 0.9477 | 0.9988 / 0.9323 | 1.0000 / 0.9768 |
| saved_range-1 | 0.2992 / 0.2977 | 1.0101 / 1.0099 | 1.0032 / 1.0009 | 0.9894 / 1.0017 | 1.0217 / 1.0184 | 1.0129 / 1.0146 |
| saved_triangle-2 | 0.0280 / 0.0284 | 1.0064 / 1.0077 | 0.9033 / 0.9034 | 1.0030 / 1.0021 | 1.0019 / 1.0204 | 0.9936 / 0.9913 |
| saved_point-2 | 0.9115 / 0.8470 | 0.8745 / 0.8567 | 0.8963 / 0.8483 | 0.8286 / 0.8513 | 1.0128 / 1.0116 | 1.0116 / 0.9819 |
| saved_range-2 | 0.2485 / 0.2642 | 0.9907 / 0.9940 | 0.9823 / 0.9856 | 1.0291 / 1.0409 | 1.0089 / 0.9821 | 1.0128 / 1.0102 |
| saved_triangle-3 | 0.0239 / 0.0268 | 1.0110 / 1.0112 | 0.9019 / 0.9037 | 1.0050 / 1.0182 | 1.0091 / 1.0126 | 0.9996 / 1.0028 |
| saved_point-3 | 0.8881 / 0.9309 | 0.9645 / 0.9990 | 0.8958 / 0.9473 | 0.9483 / 0.9610 | 1.0000 / 1.0092 | 1.0612 / 1.0499 |
| saved_range-3 | 0.2098 / 0.2181 | 0.9410 / 0.9324 | 0.9322 / 0.9228 | 0.8739 / 0.8631 | 1.0047 / 1.0067 | 1.0262 / 1.0241 |
| computed-0 | 0.1223 / 0.1277 | 1.0039 / 1.0056 | 0.9777 / 0.9792 | 1.1270 / 1.1946 | 1.0000 / 1.0172 | 0.9985 / 0.9870 |
| computed-1 | 0.1209 / 0.1312 | 1.0098 / 1.0134 | 0.9836 / 0.9866 | 1.1705 / 1.2376 | 1.0448 / 0.9895 | 0.8624 / 0.8625 |
| computed-4 | 0.1547 / 0.1580 | 0.9898 / 1.0043 | 0.9788 / 0.9923 | 1.0790 / 1.0955 | 0.9692 / 0.9670 | 0.9796 / 0.9500 |
| union-2 | 0.1548 / 0.1556 | 0.9871 / 0.9780 | 0.9622 / 0.9526 | 0.8055 / 0.8073 | 0.8766 / 0.8765 | 0.9961 / 0.9754 |
| union-4 | 0.2296 / 0.2239 | 1.0575 / 1.0703 | 1.0452 / 1.0562 | 1.0055 / 1.0114 | 0.9653 / 0.9498 | 0.9963 / 0.9962 |
| groups-1 | 0.6438 / 0.6629 | 0.9885 / 0.9973 | 0.9849 / 0.9953 | 0.8764 / 1.0739 | 1.0345 / 0.9964 | 1.0000 / 1.0162 |
| groups-3 | 0.6584 / 0.6703 | 1.0119 / 1.0180 | 1.0092 / 1.0164 | 1.0083 / 1.0039 | 1.0558 / 1.0596 | 1.0485 / 1.0465 |
| groups-4 | 0.8111 / 0.8130 | 1.0036 / 0.9868 | 1.0028 / 0.9864 | 0.9749 / 0.9957 | 0.9655 / 0.9316 | 1.0110 / 1.0090 |
| dnf_groups-0 | 0.1641 / 0.1632 | 1.0548 / 1.0534 | 1.0353 / 1.0308 | 1.0025 / 0.9929 | 1.0045 / 1.0111 | 1.0089 / 1.0096 |
| dnf_groups-2 | 0.2699 / 0.2574 | 1.1766 / 1.1148 | 1.1651 / 1.1038 | 1.0672 / 1.0741 | 1.0142 / 1.0037 | 0.9987 / 1.0265 |
| pairs-2 | 0.1383 / 0.1381 | 0.9960 / 0.9920 | 0.9817 / 0.9774 | 0.9962 / 0.9986 | 1.0126 / 1.0228 | 1.0073 / 1.0235 |
| dense-2 | 0.9786 / 0.9934 | 1.0032 / 1.0135 | 1.0091 / 1.0069 | 0.9751 / 0.9863 | 1.0000 / 1.0130 | 1.0323 / 1.0422 |
| interior-4 | 0.2238 / 0.2154 | 0.9444 / 0.9519 | 0.9349 / 0.9427 | 0.8793 / 0.8634 | 1.0333 / 1.0475 | 1.0814 / 1.1537 |
| reach-2 | 1.0170 / 1.0746 | 1.0699 / 1.1148 | 1.0379 / 1.0902 | 1.0174 / 1.0425 | 1.0101 / 1.0181 | 1.0097 / 1.0113 |
| interior_reach-2 | 0.9383 / 0.9418 | 1.1057 / 1.1112 | 0.9972 / 1.0057 | 0.9804 / 0.9858 | 1.0042 / 1.0100 | 0.9876 / 0.9822 |
| union_groups-2 | 0.2394 / 0.2284 | 1.0888 / 1.1328 | 1.0786 / 1.1209 | 1.0604 / 1.1256 | 1.0120 / 1.0079 | 0.9988 / 1.0136 |

Every individual A0/B0/B1/A1 statistic, including min/p90/p95/p99/max,
remains in q1-timing-review-1/distributions.tsv and comparisons.tsv. Raw
samples remain in the original per-process logs. The review STATE stores
all comparative statistics and source/input/output identities.

## Interpretation and adverse controls

The triangle preparation reduction is large and consistent: baseline
individual medians are about 1.00–1.10 ms versus candidate 21.7–38.8 us across
all draws. Prepare+first p50 centers fall 7.77%, 10.26%, 9.67% and 9.81%.
This prices the measured deletion of about 80 MiB of speculative backing.
It does NOT imply a similarly large warm speedup: same-target p50 centers
are +1.32%, +0.12%, +0.19% and +0.91%, including flagged windows.

Range preparation falls from approximately 50–63 us to 12–17 us, but its
roughly 5 ms first execution dominates combined cost. All four same-target
warm p50 ratios remain adverse (+0.47% to +3.07%), and alternating p50 is
+1.28% to +2.62%, with no warm clock flags. Execution allocation/owners were
unchanged in the saved allocation audit; do not attribute this to additional
warm growth or dismiss it merely because it is small.

The high-cardinality costs matter more than another tiny-allocation cleanup:

- DNF aggregate threshold 500: 500 groups, 99,872 distinct input pairs.
  Combined p50/mean rises **16.51% / 10.38%**, all cold brackets unflagged.
  Individual combined medians in A0/B0/B1/A1 order are
  11.713 / 14.438 / 13.106 / 11.900 ms. Both candidate rounds lose.
  First-execution p50 rises 17.66%; second p50/mean rises 6.72% / 7.41%.
  Warm alternating median is nearly flat, but its mean is +2.65% and its
  maximum ratio is 1.742. Keep that tail.
- Written-union aggregate threshold 500: combined p50/mean
  **+7.86% / +12.09%** and second +6.04% / +12.56%, unflagged.
  Round medians 11.712 / 14.653 / 12.601 / 13.550 ms show substantial
  order variation; this does not repeat uniformly in both paired directions.
- 100,000-row union: combined **+4.52% / +5.62%**, all cold brackets
  unflagged. Round medians 8.270 / 8.815 / 8.592 / 8.383 ms both lose.
  First p50 is +5.75%, whereas alternating warm p50 is -0.37%.
- 8,192 hashed groups: combined +0.92% / +1.64%; same-target warm
  **+5.58% / +5.96%**, alternating +4.85% / +4.65%, all unflagged.
  Same-target medians 487.4 / 499.4 / 532.5 / 489.4 us are adverse in
  both paired directions. Equal final capacity is not a warm-speed proof.
- 100,000 computed rows: combined center looks slightly favorable, but
  A0/B0/B1/A1 medians 11.044 / 9.483 / 10.790 / 9.671 ms change sign
  across paired directions. Second p50/mean is +7.90% / +9.55%, unflagged.
  An unflagged candidate warm maximum is 13.316 ms at index 85; the baseline
  alternating maximum is 17.510 ms at index 239. Neither is filtered.
- 100,000-row interior has a favorable combined center confounded by slow
  baseline A0. Warm centers/tails are adverse, including the flagged B0
  alternating 54.493 ms at index 29 (19.64x its median). No warm win claimed.
- Plain reach combined p50/mean is +3.79% / +9.02%, unflagged. Interior
  reach first p50 is +10.57%, although cheaper preparation nearly balances
  its combined center. Dense and point controls are tiny/timer-quantized,
  not permission to claim exact causal percentage gains.

The broader allocation audit already measured roughly 2.36 MB extra joint
requested bytes for large computed/union/interior cases, roughly 295 KB for
large hashed groups and 2.14 MB for the 500-group DNF/union cases. Those are
real additional growth requests. Ordinary timing now identifies costs worth
investigating, but does not prove all first/second/warm differences are rehash
time. Final table placement, preparation metadata/code layout, state history
and shared-host interference remain possible contributors.

## Periodic clear and tail review

All 224 warm distributions' top indices and adjacent windows are preserved in
warm-tails.json; no sample was discarded. The source-model prediction fixed
before running was same-target index 248 for the nonempty single-sink saved
triangle: eight warmups, first empty reset does not advance generation.

At that index baseline triangle is 2.36–8.93% above its corresponding median,
with descending ranks 5–58, not a dominant maximum. Candidate observations
range from 1.49% below to 4.85% above median. This is not direct event
attribution, and not proof of a general warm-tail reduction. The large
candidate triangle-2 spike is at indices 146–150, not 248, with a flagged
bracket; it remains in every statistic.

Exploratory, NOT predeclared, inspection also found computed-five-row
baseline maxima at index 248 in both rounds (1,625/1,666 ns versus medians
458/500 ns). Candidate maxima are 625/625 ns. The small hashed-group
baseline also has its maxima at 248. These are useful source-consistent
observations, not a reason to reinterpret every unrelated outlier as rollover.
The range baseline has a 50,167 ns outlier at index 93 even though its
speculative hash backing was discarded before execution.

## Complete clock-flag inventory

Cold flags apply to prepare/first/combined/second as one shared bracket.
All unflagged and flagged numeric stamps are in clocks.tsv.

- saved_triangle-0-B1/cold
- saved_triangle-0-B1/alternating
- saved_triangle-0-A1/alternating
- saved_triangle-1-B1/cold
- saved_triangle-1-B1/same_target
- saved_triangle-1-B1/alternating
- saved_point-1-A1/same_target
- saved_triangle-2-B0/same_target
- saved_point-2-A0/cold
- saved_point-2-A0/same_target
- saved_point-2-A0/alternating
- saved_point-2-B0/cold
- saved_point-2-B0/same_target
- saved_point-2-B0/alternating
- saved_point-2-A1/cold
- saved_range-3-A0/cold
- computed-4-A0/same_target
- union-2-A0/cold
- groups-4-B0/cold
- groups-4-B0/same_target
- dnf_groups-0-A0/cold
- dnf_groups-0-B0/cold
- dnf_groups-2-A0/same_target
- pairs-2-B0/same_target
- interior-4-B0/alternating
- interior-4-B1/same_target

## Disposition and next saved-evidence discriminator

Q1 has demonstrated correctness, a large allocation/preparation benefit,
and real adverse ordinary controls. **Keep it isolated; do not claim universal
performance acceptance or hide those regressions in an aggregate score.**
No identical panel rerun for better numbers, and no new full trace.

Next study the saved DNF-500, union-100000 and groups-8192 cases at the map
growth mechanism. Initial/final owner records already bound the work:
large seen maps grow from baseline 131,072 slots versus candidate zero to
524,288; the 8,192-group map grows from baseline 16,384 versus zero to
32,768. Derive the extra rehash entry/word counts from the actual distinct
counts and unchanged load/growth rules, then use a minimal count-only
discriminator only where the recorded data cannot establish the path.
Include duplicates and zero/small outputs; offered input rows are not distinct
keys, so do not reintroduce speculative reservation from a batch length.

Separate first-use extra work from warmed-only losses. Examine exact final
map geometry/probe behavior and initialization history, then the removed
runtime PlanNode field/code-layout difference for the unchanged range path.
No claim that a cause has already been identified. Price any genuinely new
mechanism with matched ordinary controls, keeping the current panel intact.

This is higher priority than the eight-byte dense finalization request.
Existing output Cell and survivor-conjunction allocation leads and all prior
candidates remain open. The goal remains active, incomplete and not blocked.

