# G2 ordinary timing: staged occupied-entry growth

The preceding goal turn made progress: G2 correctness/build/oracle gates and
matched saved-corpus ownership establish real memory savings, including the
larger scratch. All six candidate fingerprints are revalidated against
g2-closeout-1 before acting. G2 remains isolated, not performance-accepted.
No new full trace or full benchmark is authorized or needed for this step.

## Protocol fixed before measurement

Reuse the byte-identical external e1_timing.rs driver, including its original
hash. Its old comment about a result-cache hit is wrong; preserve the frozen
driver and use the correct interpretation: both warmed lanes execute rules
again, not cached answer rows. All definitions and controls are the same as
the completed G1 panel, now with a genuinely new G2 implementation.

Build published baseline and G2 in the SAME g2-source path with workspace
fat-LTO release, opt-level 3 and one codegen unit. Engine features must be
exactly [collision-probe], with no allocator/profiler/test code or debug
assertions. Only grow.rs differs in ordinary engine code. Remove the temporary
Cargo binary mount and restore the exact gate-3 source before any timing.
No compilation, symbol processing or source edits overlap measurements.

One fixed ABBA per case: A0 B0 B1 A1, triangle then point for each of four
saved draws, using private byte-identical copies of the existing S/seed-1
database and read-only SQLite oracle. Every process checks four handwritten
SQL goldens before timing and validates measured outputs outside timers.

- Cold and second execution: DB reopen/prepare and parameter-array creation
  are excluded. Two discarded pairs then 16 first/second pairs. Cold means
  fresh execution state, not evicted OS/file caches.
- Alternating-parameter warmed (rotating): alternate draw outside timer,
  then target; eight warmups and 64 samples.
- Same-target warmed (memoized): eight warmups and 64 batches of 16 calls.
  Both warmed lanes include the existing parameter-array helper.

All 32 processes, 128 distributions and 96 independent clock brackets must
be reviewed. First/second share a bracket; each warmed lane has its own.
No retries, sample removal, frequency normalization or repeat-until-win.
macOS user-interactive QoS is verified steering, not hard P-core affinity.
Keep every point control and cold/warm tail, including adverse unflagged
results and timer quantization on sub-microsecond second-call controls.

G2 only rehashes during construction, but changes arena offsets/capacities;
no warm copying does NOT prove warmed lookups cannot regress. Existing memory
savings do not by themselves establish a speed win. Shared-host measurements
are not full-suite, RSS, peak memory, storage-layout or Raspberry Pi results.

## Completed builds and panel

Candidate build 1 (session 82898) completed 10:24:19 UTC; published baseline
build 1 (67466) completed 10:25:59. Both ordinary binaries use the same
g2-source path and the frozen external driver, SHA-256
`13e2309e46788f7778394c80f16b910679da976e240531a8649971fd33ebb4d1`.
Candidate executable: `e6e3386316f32a9123b282ebbd425bae4c2a52008577c24d68861111e2f6d038`.
Baseline executable: `c14157f19676af5c3a028e1eb9295296071227ce8a88c58777fe7c8ea120282d`.
Only ordinary grow.rs differs. force.rs is published code; no test, allocator
or observer instrumentation is present. Temporary Cargo mounts were removed
and all six exact candidate identities verified by g2-closeout-2 BEFORE timing.

g2-timing-1 (session 66246) ran 10:26:14–10:27:02 UTC, exit 0. The fixed
32-process panel completed with every SQL golden/output check passing. Review
1 completed 10:27:08, exit 0. All 128 distributions were recomputed from raw
samples; all 96 clocks and 32 matched summaries were read. **25 clock brackets
are flagged**, zero retried. All results are retained, without normalization,
sample removal or another identical-panel run.

## Matched results

Each entry is candidate/baseline geometric round-center **p50 / mean**.
These are descriptive ratios, not confidence bounds or claimed causal gains.
The raw review also preserves p90/p95/p99/max and every individual round.

| Triangle draw | Cold | Second | Alternating warmed | Same-target warmed |
| --- | --- | --- | --- | --- |
| 0 `[1,6)` | .9872 / .9831 | 1.0112 / 1.0214 | 1.0043 / 1.0088 | .9981 / .9962 |
| 1 `[167,172)` | .9942 / .9890 | 1.0053 / 1.0016 | .9829 / .9868 | .9904 / .9912 |
| 2 `[333,338)` | .9470 / .9587 | .9843 / .9806 | .9863 / .9796 | .9931 / .9911 |
| 3 empty | .9517 / .9307 | .9989 / .9987 | .9303 / .9062 | .9758 / .9183 |

| Point draw | Cold | Second | Alternating warmed | Same-target warmed |
| --- | --- | --- | --- | --- |
| 0 | 1.0165 / 1.0794 | 1.0460 / 1.0569 | .9989 / .9786 | .9927 / .9859 |
| 1 | 1.0420 / 1.0577 | 1.0878 / 1.0817 | 1.0480 / 1.0012 | 1.0164 / 1.0268 |
| 2 | .9130 / .8563 | .9571 / .9691 | .9989 / .9779 | 1.0214 / 1.0398 |
| 3 | .9541 / .9480 | .9132 / .9397 | .9483 / .9425 | .9958 / .9929 |

Nonempty cold triangle medians change -1.29%, -0.58%, -5.30%. The following
adverse/control details prevent treating that as a clean speed win:

- Triangle 0 second: mean +2.14%, maximum +13.35%. Both candidate brackets
  are unflagged; A0 is flagged. B0 max 2.103 ms exceeds A0/A1 1.722/1.772 ms.
- Triangle 1 alternating: maximum +3.36% despite the improved center; B0
  bracket flagged. Triangle 2 cold max +1.99% despite its center; A1 flagged.
- Empty triangle apparent warmed wins are confounded by slow/flagged A1.
  Alternating baseline medians 1.479/1.867 ms versus 1.547/1.544 candidate;
  same-target baseline means 1.540/1.860 versus 1.556/1.552 candidate. A1
  post-clock proxies are 2.623/2.841 GHz. Do not claim a ~9% empty warm win.
- Point 1 cold is unflagged adverse: median +4.20%, mean +5.77%. Point 1
  same-target is also unflagged adverse: mean +2.68%, maximum +17.78%.
- Point 0 cold mean +7.94%, maximum +36.89%, with B1 flagged. Point 2
  same-target mean +3.98%, maximum +15.99%, with A0/B0 flags. Conversely
  point 3 improves across centers with no flags: controls are mixed, not
  uniformly slower or uniformly unaffected.
- Point second-call medians are roughly 417–542 ns in ~42 ns timer steps.
  Report the raw quantization, not precise causal percentage claims.

Complete flag inventory (cold includes the shared second-execution bracket):

| Case | Cold | Alternating | Same-target |
| --- | --- | --- | --- |
| triangle 0 | A0 | A0 | B0, B1, A1 |
| point 0 | B1 | A0 | none |
| triangle 1 | B1, A1 | B0 | B1 |
| point 1 | none | B0, B1 | none |
| triangle 2 | A1 | B1, A1 | A1 |
| point 2 | A0, B0 | none | A0, B0 |
| triangle 3 | A0, B1 | A1 | A1 |
| point 3 | none | none | none |

The authoritative raw record is `g2-timing-review-1/review.log`; input,
output and build manifests remain beside the original panel. A flag is a
clock-proxy diagnostic, not proof of cause; an unflagged bracket does not
guarantee a noise-free shared host.

## Disposition

Correctness gates and measured memory savings remain valid. Ordinary timing
is mixed/noisy, so G2 remains isolated and **not performance-accepted**. No
identical-panel rerun. This is neither full-suite nor RSS/peak/Pi qualification.
Keep mining the existing CPU and allocation evidence for another supported
general issue; do not infer that a deferred candidate exhausts its lead.
