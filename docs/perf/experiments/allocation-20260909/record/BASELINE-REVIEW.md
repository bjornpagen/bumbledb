# Current ordinary baseline review

Source: `b02a641e087364ec09c161a97c67c87cf626e6b2`, unchanged engine.
Timing binary SHA-256:
`84b920460fe27b49181a8e8a444e6557971f559e96972fb5e5780f305d47cc74`.
This is the published 1.1.0 timing executable, not a candidate build.
The full ordinary suite completed at **2026-09-09 01:30:44 UTC** with
`LOCAL-LANES-COMPLETE`, all 13 supported measurement lanes plus verification
and chart rendering successful, and process session 18763 returned exit 0.
This is execution/report completion, not every performance target passing or
qualification of the manifest's explicitly unrun prerequisites.

## Curves: completed 2026-09-09 01:15:07 UTC

Read the entire JSON and Markdown, including all S/M/L distributions, all
warmth distributions, counts, caps, frequency flags and provenance.

- All four registered families have S/M/L engine timings. Two SQLite points
  (triangle M and L) hit the **timing** cap. Their oracle gates completed;
  missing SQLite distributions are not 30-second measurements or speedups.
- Triangle L now passed its oracle gate and provides engine evidence at
  25,263,139 ledger facts: p50 266.339 ms, p99 1,055.083 ms, mean 303.134 ms,
  mean 375 output rows/call. Published triangle L had capped at its gate and
  had no engine distribution, so there is no historical L latency to compare.
- Triangle M: p50 23.216 ms, p99 104.918 ms, mean 32.048 ms. The warm/cold S
  first-execution difference is also material (10.249 ms reopen-cold versus
  1.662 ms warm); open/prepare and OS-page-cache flushing are NOT in that timer.
- Busy-scan L: p50 555.208 us, p99 908.625 us, mean 37,535.25 output rows/call.
  Both canonical and hand-tuned SQLite distributions completed. This is a
  useful materialization/scan control, not the same work as a one-row aggregate.
- Closure-fanout L: p50 7.042 us but mean 4.975 ms and p95 17.825 ms. Its mixed
  parameter draws make the median a poor proxy for the expensive fanout draws.
  Study full cycles, output mixes and native caller paths, not the median alone.
- Five of twelve scale points carry `ghz.contaminated`: point M/L, busy-scan M,
  closure-fanout M/L. Closure's warmth panel is also flagged. Seven scale
  points were retried. Preserve all flags; an exit-zero lane is not evidence
  of uniform measurement quality.

The shared-host large busy-scan p50 is about half the published value, and
closure-fanout's median changed much more, despite identical timing binaries.
Other points went the other direction. These are controls on interpretation,
not improvements or regressions. Candidate acceptance needs same-session
baseline/baseline and alternating baseline/candidate controls with the full
draw mix. Current CPU traces and allocation evidence still determine the
highest-impact shared mechanism; the probe/map leads are not yet selected.

## Main reads: completed 2026-09-09 01:20:14 UTC

Read all 32 read distributions and all 12 write/cold distributions, the per-engine
frequency flags, provenance, corpus/verification stamps and report gates.
Every read used batch 1. `partial=false`, `all_win=true`, `budget_ok=false`,
`budget_gates=false`: exit zero is not a claim that every latency target passed.
`all_win` only means no read was classified Loss; report-only families and all
write comparisons are not required wins. `spread` misses the report's p99 target
at 16.753 ms. All three displaced-probe rows exceed it too, but are report-only.

Largest read medians are displaced probes (128.317–153.161 ms), spread
(13.622 ms), triangle (1.635 ms), skew (1.117 ms), and rsvp_union (0.907 ms).
Point remains 417 ns. Keep high-tail mixes visible: containment_walk's median
is 2.250 us but its mean is 154.039 us and p99 924.625 us. Native profiles must
retain full draws rather than repeat a median cheap key.

The displacement ordering is not monotone: d96 is faster than d24 and d0 in
this single run. Do not conclude cache pollution improves the engine or accept
a candidate from one such ordering. Per-engine source traces and alternating
unprofiled controls are required.

Six read engine brackets and eight write/cold engine brackets are frequency
flagged. Durable commit_batch has p50 22.126 ms versus SQLite 13.117 ms despite
the read report's all_win flag; this remains a real control to inspect in the
full write traces. insert_stream is 608.370 ms (328,300 facts/s), not a read
latency. Cold containment-after-delete includes a real relation-version rebuild
(p50 166.333 us, p99 9.577 ms); the unrelated-Org variant is not the same event.

### Corrected allocation denominator before running diagnostics

Source inspection prompted by displaced p99=max reveals that its default is
**12 samples**, not report.config.samples=256. driver/bench.rs passes the
optional CLI override separately into displaced::bench_families; closure and
regular reads instead inherit the ordinary protocol. No --samples override is
used in this round. Therefore the six displaced allocation windows must be
divided by 12 * batch, and their harness sample vector is 96 requested bytes,
not 2,048. The ignored diagnostic driver now records per-family sample counts,
operation denominators, and sample-vector sizes. Raw reports remain untouched.
This is a measurement-interpretation fix, not a measured engine improvement.

## Scenarios, writes, storage and lifecycle

Read all 34 scenario distributions and their comparison outcomes. There are
three capped SQLite lanes: r4_bomb_t2, r6_two_path_count and canonical
t2_overlap_join. The tuned t2 comparison completed. These remain caps, not
timings. Scenario JSON has seed/warmups/samples but no per-query frequency
brackets; use manifest/log provenance, never infer that an unflagged field is
a clean measurement. Eight warmups and 64 measured calls were used per query.

- r4_bomb_t2: p50 1.525 s, min 1.226 s, p99 6.173 s, mean 2.048 s.
  This remains the largest ordinary per-call engine cost. Same binary as the
  published run; the large tail does not establish a code regression.
- t2_overlap_join: p50 41.386 ms, p99 80.304 ms; o4_segment_category:
  p50 27.213 ms, p99 61.288 ms. r1/r2 and r6 also carry material join costs.
- Several scenario `answers=0` records are integer-truncated mean output-row
  counts over hit/miss mixes, not proof that all draws returned empty. In
  particular a one-row Count result does not report the number counted.
- All nine durable write ladder/stream rows reviewed. At batch 1000, insert
  p50 is 29.462 ms versus SQLite 19.464 ms; delete is 36.225 versus 22.925 ms.
  Five rows have frequency warnings, including inserts. Smaller fsync-bound
  writes are near parity; do not let read-win flags hide write-side costs.
- All four S/M storage rows reviewed, with zero outstanding SQLite WAL bytes.
  Compacted ledger is about 131 bytes/fact; calendar about 161 bytes/fact.
  These are file sizes, not RSS, heap or populated mmap residency.
- All four native app reports have protocol 2. Warm projection p50 1.708 us;
  first projection after a relevant write 3.406 ms; cold open/prepare/project
  358.250 us; full-result execution/delivery 5.150 ms total (1.443/3.707 ms
  component medians). Tiny tenant activation p50 354.125 us, fd_growth=0.
  These are native operations, not TypeScript/Effect or hosted S3 qualification.
- All 11 CRUD rows and six lawful rows reviewed; both poststate checks are ok.
  Preserve losses and frequency warnings. All lawful frequency stamps warn,
  so its 23–44 us rejection medians need controlled evidence before attribution.
- Heap get/contains/scan, four admission sizes, publish and join reviewed.
  They compare distinct storage contracts, not interchangeable deployment modes.
- Hash equivalence passed. Every input-size/alignment timing row, initialization
  and mixture row was reviewed. KAT was explicitly NOT RUN, and reusable-state
  timing was NOT RUN. Fresh-state/finalize/Vec timings are not isolated hash
  kernels and cannot justify a persistent digest change or a security claim.

The manifest also leaves correspondence-oracle tests, three-way conformance,
real hosted S3, large-populated/cgroup residency, Graviton and Linux x64 Node
qualification unrun. No release or external-green claim follows from this run.

## Next diagnostic, no new full trace

Only after the baseline session ended and its children were confirmed gone,
started `diagnostics.py allocations` under the measurement mutex (session
26191, outer log `allocations-driver.log`). It builds and freezes an allocation
counter executable and runs the complete 32+34 family count roster. Census
follows serially. Both full-trace phases remain guarded off per the latest user
instruction; mine the saved traces and exhaust their supported improvements.
