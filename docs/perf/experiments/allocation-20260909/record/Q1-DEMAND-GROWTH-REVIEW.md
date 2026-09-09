# Q1: demand-grown result and group tables

Status: isolated implementation, correctness-gated and saved allocation
benefit verified. **Not performance-accepted.** No full trace, full benchmark,
commit, push, release, tag, publication or version bump. Q1 remains separate
from P2/P3/M1/E1/G1/G2 at published b02a641e.

The latest user instruction still prohibits a full trace until the saved
evidence has been exhausted. This experiment uses that evidence; it is not
an exhaustion declaration.

Subsequent update: Q1-BROADER-CONTROLS.md completes the high-cardinality,
union/DNF, dense/hashed aggregate and recursive/interior allocation controls.
It preserves genuine adverse first-use growth costs. Q1-ORDINARY-TIMING-REVIEW.md
now completes the fixed 112-process ordinary comparison and all 672 raw
distribution checks. Preparation improves substantially, but DNF/union first-use
and some warm controls regress. Q1 stays isolated, not performance-accepted.
Do not repeat either completed panel; study the saved adverse growth mechanisms.

## Change and correctness

Delete speculative join-cardinality allocation hints from preparation of main,
interior, recursive and computed sinks. Projection and all aggregate dedup
regimes construct empty SpillSets; hashed groups construct empty WordMaps.
The existing insertion-driven geometric growth and retained reset/release
behavior are unchanged. Exact dense-group radix tables and distinctness
witnesses are unchanged. No new allocator, quota, mode, fallback, SDK or
persisted-format change.

The estimate-collecting accessor became unused, which strict lint caught in
q1-gates-1. Its only consumer had been the deleted allocation hint route. The
accessor and unused execution PlanNode estimate field are now removed.
Planning Node estimates, join ordering and fold-split estimates are preserved.
The explicit-capacity fixture constructor is test-only so collision,
saturation and generation tests retain their established geometry.

Evidence preserved:

- q1-baseline-1 (session 64589): existing WordMap controls pass; the new
  zero-speculative-backing regression fails as intended, 83,886,087 vs 0.
- q1-focused-1 (39973): 67 sink tests pass.
- q1-gates-1 (31179): format and focused tests pass, strict lint fails on
  the unused estimates accessor. Later gates did not run in that attempt.
- q1-gates-2 (45625), completed September 9, 10:54:58 UTC: format, 67 sink
  tests, strict engine/benchmark all-target lint, 1,324 ordinary library
  tests and 1,337 allocation-enabled library tests pass (18 intentionally
  ignored in each library run). Ordinary release build and all 2,879
  independent query-oracle cases pass. No CI or full-workspace claim.

New tests cover initial empty table ownership, five-key exact capacity,
widths 0/1/2/4/8/dynamic, duplicates, insertion order, geometric growth,
300 resets including stale generations, warm reuse without allocation,
release/refill, all aggregate dedup regimes and exact dense-group tables.
The unchanged suite exercises bulk insertion, recursive deltas, union/DNF,
computed/Pack, cancellation/sticky errors, actual forced spill and lifetimes.
Passing those tests does not establish the cost of larger first-use growth.

Gate-2 source fingerprint:
`9f87b492b1248093d241a0217b2cb089db08d8ed82a6f1f72a5fce3a43d6b418`.

## Matched saved measurements

q1-audit-1 (29469), completed 10:55:38 UTC, uses the frozen S/seed-1 database,
saved range/triangle parameters, schema, SQL answers and owner observers.
All 32 execution windows pass exact SQLite-result checks. Preparation is
measured in its own process before any execution. No elapsed-time claim.

The original baseline preparation helper is preserved. Its candidate variant
omits exactly two lines printing runtime estimates AFTER the allocation
snapshot, since execution no longer carries those estimates. The audit
verifies that exact textual difference and all other helper/input hashes.
It also verifies every ordinary source file against the gate archive after
logically removing the five cfg(test) observation mounts.

q1-audit-review-1 verifies all 32 allocation windows, all result-map owners,
all 32 answer owners, all 100 active/parked view records and all 64 spare
buffers. Every non-result owner record exactly matches the frozen baseline.
The main agent reviewed every execution counter, result/answer owner and
both full preparation records; unchanged views/spares were checked in full
by byte-for-byte record comparison, not inferred from a sample.

Preparation counters, in the order allocs/deallocs/requested/freed bytes:

| Query | Baseline | Candidate |
| --- | --- | --- |
| range | 173 / 103 / 613,519 / 597,279 | 169 / 99 / 23,672 / 7,440 |
| triangle | 299 / 146 / 83,938,163 / 15,840 | 295 / 145 / 52,028 / 15,816 |

Both prepared result maps have zero table payload. Range no longer allocates
and immediately discards its 589,831-byte hinted table. Its requested-byte
reduction is 589,847 after also removing the 8-byte collected estimate and
8-byte execution-plan field. Triangle saves 83,886,135 requested bytes:
83,886,087 backing plus 24-byte collected estimates plus 24-byte plan fields.

Actual result-map retained capacities:

| Case | Baseline | Candidate |
| --- | ---: | ---: |
| Before triangle execution | 83,886,087 | 0 |
| Five-result triangle, fresh or warm | 83,886,119 | 199 |
| Empty fresh triangle | 83,886,087 | 0 |
| Empty draw after prior five-result draw | 83,886,119 | 199 |

The five-result table is 16 slots: 23 control, 128 key, 16 stamp and 32 dense
capacity bytes. Vec<()> contributes zero value payload despite its
usize::MAX capacity. Answers retain their separate 120 bytes when applicable.
The result-table reduction is 83,885,920 bytes, not a COLT-pool saving.

### Do not hide the work moved to execution

Nonempty cold triangle adds **6 requests, 3 deallocations, 254 requested bytes
and 87 freed bytes**. The existing table growth allocates 87 bytes at eight
slots then 167 at sixteen; the old 87-byte arrays are freed. Dense-list growth
is unchanged. Exactly the same extra work occurs at the first nonempty draw
of the first parameter rotation. Other execution allocation tuples match
baseline exactly; all warm and second-rotation tuples remain zero.

Prepare PLUS first nonempty triangle execution therefore has **2 more
requests and 2 more deallocations**, **83,885,881 fewer requested bytes** and
63 more freed bytes. Net requested-minus-freed is 83,885,944 lower, including
24 bytes of smaller execution-plan metadata. For draw 0, total requested
bytes are 100,535,311 → 16,649,430; all three nonempty draws have the same
delta. The empty draw saves 83,886,135 requested bytes and four requests.

Range execution counters and owners are unchanged. Combined preparation and
cold execution save four requests and 589,847 requested bytes, but retained
net bytes fall only eight: the speculative range backing was already freed
before execution. Do not present that transient saving as retained memory.

These are measured Rust allocation layouts and owner capacities on this Mac,
**not touched pages, RSS, peak memory, elapsed-time improvement or 512 MB Pi
qualification**. OS lazy zeroing/overcommit can keep large backing nonresident.

## Remaining work, before acceptance

1. Broader allocation controls are COMPLETE: all 41 case/draw combinations,
   328 windows and 488 map-owner records per variant reconcile, including
   adverse first-use requests. See Q1-BROADER-CONTROLS.md.
2. Matched ordinary timing is COMPLETE: all 112 processes, 672 distributions,
   78,848 raw values and 336 clock brackets checked; 26 flags retained.
   Actual preparation+first intervals and rollover-length warm samples are
   included. See Q1-ORDINARY-TIMING-REVIEW.md for every case and disposition.
   Next discriminate the saved DNF-500/union-100000 cold regressions and
   groups-8192/range warm losses without assuming allocation explains all
   elapsed time. Derive growth work and inspect final map/code geometry.
3. Keep this candidate isolated; do not add independent E1/G2 savings or call
   the overall research accepted. Output Cell representation and survivor
   conjunction allocation remain measured, lower-priority leads.

No full trace is needed for any of these next steps.

## Closeout

q1-candidate-closeout-1 (56182), completed 10:56:04 UTC, verifies all five
temporary observer mounts are removed, exact gate-2 source restored,
format/diff checks passing, all frozen observation dependencies unchanged
and all six prior candidate fingerprints preserved. All sessions are terminal.
The research goal remains active, incomplete and not blocked.
