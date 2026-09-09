# P2 gate 6: split the hot layout check from rebuild — unaccepted

The latest user instruction still prohibits a new full trace. Saved evidence
has open P1/P2/P3/M1/M2/M3 leads, not an exhaustion declaration.

## Evidence and isolated change

Matched gate-5 mechanism attempt 5 (p2-wide-5/) points at production dispatch,
not cache movement alone: D/C adds roughly 9-17% in several narrow singleton
cases; C/B changes are mostly a few percent. All 144 blocks, including 16
boundary flags, remain. Ordinary gate-5 disassembly in p2-codegen-1/ shows
refresh_shape_cache reserves 112 stack bytes and saves six register pairs even
on a hit. The saved o4 nearest-owner attribution is 6.23% for that function;
it is historical attribution, not an additive predicted saving.

groups.rs now inlines only the identical real-slot-count and key-slot-content
comparison and calls a non-inlined rebuild helper on a miss. The rebuild body
is unchanged. No #[cold] assumption, pointer-identity shortcut, singleton
special case, raw indexing, arithmetic change, SDK/storage change or new
allocation policy. The additional regression mutates the same key-slot array
in place, checks a repeated hit, switches group inputs between outer/leaf and
fold inputs between leaf/outer, then checks independent Sum/Count answers.

The gate-5 fingerprint was revalidated before editing:
1ced1b6ce8fbd415371de00be3f5a4d4899e50feffb1968a7a28a1c6014b0e78.
An initial fingerprint command from the repo root failed to import diagnostics;
rerunning from the research directory succeeded. No source/build was changed
by that import failure. No temporary hook is mounted.

## Prospective verification and disposition

Gate attempt 6 completed under the serial lock (session 86248, exit 0,
07:12:18-07:15:22 UTC). Formatting, strict all-target lint, 1,329 library tests
(18 ignored), 294 allocation-enabled executor tests (11 ignored), ordinary
release build and all 2,879 independent oracle cases passed.
Source fingerprint: 3bc43f3f04a09fa8983abeaf183eb54152fc8159a4a5b26fd73cbceb9f26958c.
Frozen binary: p2-gates-6/bumbledb-bench, SHA256
e298a4295192a5dd6a54cbe945c407882a42eec579a970d51357852a0e5ad06b.
Private read corpus: p2-gates-6/data; verify stamp
f4cf917f986a0cfaef89052a43d1622b7bd67893c9855de942984072c67a33fd.

Static inspection completed at 07:15:30 UTC in p2-codegen-2/. The hot wrapper
has no out-of-line symbol; both emit_batch and begin_scan inline the same exact
slot-count/length/content comparison. Their successful compare branches skip
the rebuild call. emit_batch still reserves 160 bytes, unchanged from gate 5;
it no longer pays the additional 112-byte/six-pair rebuild frame on a hit.
rebuild_shape_cache retains that frame on actual misses. begin_scan also skips
the rebuild frame on a hit. This confirms the intended code-generation change,
not a measured throughput improvement. Raw disassembly and symbols are retained.

One bounded ordinary ABC/CBA comparison completed in session 48189, exit 0,
07:15:46-07:25:54 UTC; phase p2-comparison-3. All sessions are terminal.
Command: p2-compare.py 6 3 --reference-gate 5 --scenarios olap
--reads point,range,stats,disp_probe. A=published, B=gate 5,
C=gate 6; six OLAP scenarios (24 samples, 8 warmups), plus point/range/stats/
disp_probe controls (32 samples, batch one). Published/current comparisons
test P2 as a whole; C/B isolates this split. Scenario oracles precede timings;
read corpora are privately verified and bound to the exact executable. Keep
all results, clock flags and losing controls. Scenario windows have no clock
guard, and mixed-parameter quantiles are not single-operation costs. Do not
repeat broad comparisons merely to seek a favorable direction.

Acceptance requires correctness and repeatable main-path benefit without an
unexplained material loss in affected controls. Passing the gate or a favorable
isolated mechanism is not acceptance. A result compatible with host variation
is inconclusive, not zero cost. No warm per-row allocation, RSS, hard macOS
P-core affinity, or Pi qualification follows. No commit/push/release yet.

## Completed ordinary result: split does not establish the main-path gain

All six rounds and their independent oracle gates passed. Source fingerprint
was revalidated after completion. The table divides geometric centers of the
two round summaries; these are descriptive ratios, not confidence intervals.
No sample/round was dropped or normalized. The full reports are retained.

| Family | C/A p50 | C/A mean | C/A p90 | C/A max | C/B p50 | C/B mean |
|---|---:|---:|---:|---:|---:|---:|
| o1_revenue_by_region | 1.0116 | 1.0443 | 1.0756 | 1.4555 | 1.0142 | 1.0033 |
| o2_category_window | 1.0050 | 1.1674 | 1.3942 | 1.3636 | 0.9369 | 1.0741 |
| o3_promo_split | 0.9701 | 0.9678 | 0.9635 | 0.9926 | 0.9560 | 0.9420 |
| o4_segment_category | 0.9411 | 0.9152 | 0.9437 | 0.7417 | 1.0063 | 0.9906 |
| o5_store_extremes | 0.9780 | 1.0475 | 1.1988 | 1.5131 | 0.9985 | 1.0142 |
| o6_brand_drill | 0.9588 | 0.9578 | 0.9550 | 0.9430 | 0.9587 | 0.9478 |
| point | 0.9989 | 0.9723 | 0.9211 | 0.9666 | 0.9532 | 0.8678 |
| range | 0.9683 | 0.9500 | 0.9399 | 0.8155 | 1.0082 | 1.0077 |
| stats | 0.9946 | 1.0019 | 1.0551 | 0.9575 | 1.0165 | 1.0172 |
| disp_probe | 1.0894 | 1.0387 | 1.0009 | 0.9172 | 1.0113 | 0.9707 |

- o4 C medians are 23.022875/24.293125 ms; B is 23.174458/23.834750 ms.
  The split is +0.63% p50 / -0.94% mean versus gate 5, with mixed round
  directions. It does not demonstrate the hypothesized main-query benefit.
  A is 23.989292/26.323125 ms; A1's 46.298334 ms max amplifies the apparent
  C/A mean/tail gain. Neither the min nor the center alone accepts P2.
- o3 favors C repeatedly: 322.500/329.458 us versus A 332.500/339.583 us
  and B 339.875/342.083 us. This is a real observed favorable signal, not
  evidence that all aggregate paths improved. o6 also favors C modestly,
  but its pooled tiny mixed-parameter quantiles have the documented limits.
- o2 C1 has a 1.415375 ms maximum and 469.717 us mean; C/A p50 is near
  parity but mean/p90 are +16.7%/+39.4%. o1/o5 also retain candidate pauses.
  Do not conceal these by reporting only p50 or crediting A1's pauses.
- disp_probe C medians are 104.693291/130.849125 ms; B 132.868292/
  100.814459 ms; A 87.587042/131.778958 ms. C/A centers are +8.94% p50,
  +3.87% mean. This remains an unresolved losing control. The SAME published
  binary's median swings +50.46%; B swings -24.12%. This proves the protocol
  experienced large between-round variability, not its precise cause and not
  absence of a candidate regression. All six scan boundaries are unflagged.
- Flagged read windows after the harness's existing retry: C1 range (3.222/
  3.179 GHz proxy), C1 stats (3.079/3.226), A1 point (3.191/2.858).
  Every flag and report is retained. Unflagged windows do not exclude pauses.

After measurements, a read-only host snapshot found Nessie at 72.9% CPU,
the app renderer at 36.0%, and node at 26.7%; swap use remained 9,140.12 MiB.
pmset reported no thermal/performance warning. This snapshot is not concurrent
per-sample evidence or a causal attribution. No processes/settings were changed.

Disposition: keep gate 6 frozen as an UNACCEPTED candidate. The cache-hit-frame
mechanism is confirmed, but this split is not a demonstrated o4 performance
improvement. No additional identical broad comparison is justified. Preserve
the test/patch/binary and all losing results. Inspect the remaining saved
interval/index-ownership evidence next; do not bundle a second unaccepted
production optimization into this experiment or declare the trace exhausted.
