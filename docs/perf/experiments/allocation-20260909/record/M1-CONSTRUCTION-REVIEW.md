# M1 construction ownership — isolated duplicate-growth discriminator

The preceding goal turn was progress: P3's actual allocation/branch census
completed, its ordinary comparison failed to establish a filter speedup, and
all temporary hooks were removed. No new full CPU trace is authorized.
P2 and P3 remain frozen, separate and unaccepted; no commit/push/release.

## Evidence and scope

This continuation reread current COLT force/growth/append/clone/reset ownership,
WordMap's existing duplicate-boundary handling, and the saved r4 caller owners
plus the baseline site census. r4's 8.13% copy_map_batch owner is map iteration,
not cloning; do not use it to promise a construction-copy CPU saving. The
recursive cold cap-164 census directly records two grow_map u64-pool requests
(10,240 requested bytes), two control-pool requests (640 bytes), and one
dense-pool growth (256 bytes). Those sizes are outer requested layouts; the
recorder's own activity is not production memory. This confirms the owner,
not the prevalence or benefit of the duplicate-at-threshold case.

Source verifies ingest_one grows before determining whether a key is already
present. Fix that decision separately from any arena compaction: probe first,
append duplicates directly, and grow/reprobe only a missing distinct key whose
insertion crosses the existing threshold. No new load factor, allocator,
fallback, quota, public SDK or persistent layout change is proposed.

A detached m1-source worktree starts at b02a641e, without P1/P2/P3 changes.
The first permanent discriminator covers widths 0/1/2/3/4/5/8, 25/26 distinct
keys, grouped/interleaved schedules, two resets, exact child rows, later child
force, older root readability and same-shape clones. Row id is the FIRST image
column, so canonical row sorting does not erase the two ingestion schedules.
Zero-width keys have one distinct group, not 25 invented keys. A separate
cancellation test requires duplicate append allocation refusal to propagate
without changing the root layout/contents, followed by a healthy retry.

The temporary test-only audit observes 28 cells, distinguishing ctrl/bucket/
dense live lengths, retained capacities, all execution-pool retained payload,
cold allocation requests, clone requests and zero-request reset/reforce. Images,
compiled shape and inspection work are outside request windows. Exact row
models and clone arena comparisons run outside those windows. These synthetic
cells are not end-to-end speed, RSS, query-peak or Raspberry Pi measurements.

Baseline audit 1 (session 7926) failed before any measurement because the new
fixture used RelationImage.len instead of its row_count API. The source/build
failure is preserved in m1-audit-baseline-1/. This is a fixture compile error,
not the expected growth regression. The two calls are corrected; fresh
baseline audit 2 follows with unchanged production source. Keep the source and
observer frozen during it, then verify the intended regression fails on this
baseline before modifying force.rs.

## Baseline reproduced; isolated fix and candidate ownership complete

Baseline audit 2 (session 23086) completed 08:40:07 UTC, exit 0. Its 28 cells
pass exact row/clone/reset semantics. The subsequent frozen-binary regression
at 08:40:21 UTC fails precisely at width=1/distinct=25/interleaved, with
16 table groups rather than 8. m1-baseline-regression-1/ preserves exit 101
and labels the expected old-policy failure, not a passing test. Production
source was unchanged before this failure was verified.

Only m1-source now changes ingest_one: found keys return append_child's
original Result directly; missing keys retain the existing load test, and a
real growth reprobes the new table before insertion. Ordinary inserts/hits
still do one probe; only a resizing insertion probes twice. No unsafe code,
new owner, threshold or policy. Existing insertion publication and fallible
allocation ordering remain unchanged after the selected slot is known.

The related benchmark layout formula now uses the final distinct count rather
than distinct+1. A new test distinguishes 25/25, 50/25, 50/26 and 512/102/103
position/distinct boundaries as well as empty/singleton cases. It still models
only final control/bucket words; the stock displaced fixture's 34 MiB claim
does not change. Retired tables are a separate remaining experiment.

Candidate audit 1 (session 18419) completed 08:41:24 UTC, exit 0, using exactly
the same observer SHA256 9ff2b67af865558e9ca0fdb7f00b17ea84e66910e1f1078e344608907999c67b.
Baseline debug executable: 0635f7c38978d582bb301f9c226bae5c25c1f8aaf0326a47ffe3573e13086b11.
Candidate debug executable: 8f648f6cd47d2298a1f46b27db61bc99cdb4dc1e4cebf466ea8da999d8cd6324.
All rows, cloned arenas, and reset/reforce ownership agree with their models.

Both ingestion schedules have identical observed ownership/costs in each
25-key width below; all cells are retained in m1-audit-review.py and raw logs.

| Key words | Three-arena live A/B | Execution-pool retained A/B | Cold requested bytes A/B | Clone requested bytes A/B |
|---|---:|---:|---:|---:|
| 1 | 3464 / 1188 | 8136 / 5768 | 11112 / 7528 | 5364 / 3088 |
| 2 | 5000 / 1700 | 11720 / 8328 | 15208 / 10088 | 6900 / 3600 |
| 3 | 6536 / 2212 | 15304 / 10888 | 19304 / 12648 | 8436 / 4112 |
| 4 | 8072 / 2724 | 18888 / 13448 | 23400 / 15208 | 9972 / 4624 |
| 5 | 9608 / 3236 | 22472 / 16008 | 27496 / 17768 | 11508 / 5136 |
| 8 | 14216 / 4772 | 33224 / 23688 | 39784 / 25448 | 16116 / 6672 |

Each affected cell falls from 16 to 8 table groups and 24 to 20 cold requests.
Clone request COUNT stays seven; only requested bytes shrink. Table control
length/capacity changes 192/192 -> 64/64; dense length/capacity 50/64 -> 25/32.
The old dense length contains the rehashed first 25 entries PLUS the retired
construction's 25 entries. This is measured arena content, not 50 live keys.

All 16 negative-control cells (zero width or 26 distinct keys) have EXACTLY
identical owners and cold/clone request tuples, not merely similar totals.
Every one of the 28 cells has zero requests/frees/bytes for reset/reforce on
the same image. Retained execution-pool figures exclude the compiled shape,
shared image, allocator/header overhead and RSS; this is not workload-wide
memory reduction or a speed claim. Smaller tables also change probe occupancy,
so ordinary construction and probe controls still matter before acceptance.

The temporary external audit module hook has been removed. Gate 1 completed
2026-09-09 08:47:43 UTC, session 45189 exit 0. Format, strict engine+benchmark
all-target clippy and the ordinary release build pass. Library: 1,323 passed,
18 ignored; allocation-enabled library: 1,336 passed, 18 ignored. All six
displaced model/oracle tests pass. The independent corpus oracle passes 2,879
cases; its stamp is f3081a8f6676588d097923d6fe6f066b60ec6209241db3762f8d986f3d0fc6d6.
The ignored tests were not run or counted as passing. Frozen ordinary binary:
78b50d781da898bfe7b46ed83d87f60b4055d27cf155265f7905204c29300b61.
Unhooked source fingerprint remains
11340858461482ca80cea8c55ce83661d8ae794e2f193438241fc66f47cd50fd.

This establishes correctness/build and the preceding synthetic ownership
result, not ordinary speed acceptance. The existing curves warmth panel is
a candidate construction/memoized control: two discarded reopen rounds,
16 measured cold+warm rounds, then 8 warmups/64 memoized samples. Its single
non-retrying clock bracket covers the entire panel; parameter draws are pooled.
Read these limits before selecting the workload or interpreting comparisons.
No compaction, new full benchmark, trace, commit, push or release is bundled.

## Ordinary construction/probe control, completed 08:50:09 UTC

m1-comparison-1/ (session 22822, exit 0) reuses the existing warmth lane at
scale S/seed 1 for triangle and point, ABBA. A is the ordinary published-source
binary from p3-gates-baseline-1 (84b920460fe27b49181a8e8a444e6557971f559e96972fb5e5780f305d47cc74);
B is the gate-1 duplicate-only binary above. No instrumentation/build ran
concurrently. All four inline independent multiset gates pass, with 253,264
corpus facts and mean answers 3.75 triangle/0.75 point in every round.

All 64 distributions (including SQLite) and 16 clock brackets were read and
retained. A0 triangle curve and A1 point curve report a bounded retry. B1
triangle warmth (3.413 -> 3.075 proxy GHz) and B1 point curve (3.350 -> 3.106)
are flagged; nothing is dropped or frequency-normalized. Warmth is bracketed
as one panel, not per sample/engine. No causal timing claim from those flags.

| Engine window | Geometric round-center B/A p50 | Mean | Max |
|---|---:|---:|---:|
| triangle curve | +1.22% | +3.18% | +20.50% |
| triangle cold | -7.45% | -6.65% | -3.34% |
| triangle second execution | -1.40% | -1.84% | -6.86% |
| triangle memoized rotation | +0.35% | -0.03% | +4.96% |
| point curve | -4.29% | -0.18% | +8.30% |
| point cold | -1.65% | -1.99% | -7.76% |
| point second execution | +9.05% | +5.86% | +15.42% |
| point memoized rotation | 0.00% | -2.51% | -6.88% |

Triangle cold medians: A0/A1 9.762416/9.500208 ms; B0/B1
8.855375/8.970084 ms. Its ordinary curve medians: A0/A1
1.552833/1.587167 ms; B0/B1 1.551209/1.627833 ms. Point second-execution
medians: 459/458 ns versus 500/500 ns, approximately one timer tick. Do not
turn quantized tiny-control deltas into a precise regression or dismiss them
without recording them. These are descriptive two-round centers, not
statistical significance, whole-suite acceptance or Pi qualification.

The result supports investigating the cold mechanism; it does not establish
universal speed acceptance. Next inspect actual active/parked COLT live tables,
retired construction lengths, capacities and clone calls on the saved
triangle corpus, independently checked against SQLite. This is a bounded
ownership diagnostic, not a new CPU trace or another identical timing loop.

## Actual saved-workload ownership, completed 08:55:19 UTC

The first diagnostic build (m1-saved-duplicate-1/, session 34858) failed on a
missing ParamId import in the external test. It ran no measurement and remains
preserved. Only the diagnostic import was repaired.

m1-saved-duplicate-2/ completed 08:54:50 UTC (session 52231) and
m1-saved-baseline-1/ completed 08:55:19 UTC (session 60350). Both exit 0.
They use identical observer, test and schema hashes recorded in STATE.json:
observer bb69530ae3be06d0a7e0de577d284029bf8cf2e0160b7121aad378f3f1ab6def;
test ad23b9199a92a040293e95ec926fd63628078e58161fdce70df352f0dce5a88d.
Candidate debug binary: 0d8626d94977f48e1313276e7de3972820fdc6bcdf037316b181917825ffc118.
Baseline debug binary: 447a045e0ca2f78366207c0471647229afade8bf3b910f0302c00d2bfc032599.

Each uses a byte-identical private copy of gate 1's canonical S/seed-1 corpus.
Read-only SQLite establishes exact account sets for the four actual triangle
draws (1..6, 167..172, 333..338, 500..500). Cold/second/third executions reopen
and prepare independently per draw; two further full draw rotations share a
prepared query. All 20 windows per variant match SQL. All 78 active/parked
COLT snapshots per variant check nonoverlapping current table ranges.

**Every recorded allocation tuple, clone counter, owner length/capacity and
map histogram is exactly identical between baseline and candidate.** The raw
logs differ only in the untimed test harness duration. m1-saved-review.py
verifies every row. No clone call occurs in any of these 40 windows. All eight
warm/repeat windows and all four second-rotation windows in each variant have
zero requests/frees/bytes. The ordinary cold timing signal must NOT be credited
to fewer allocation requests or smaller retained ownership for this workload.
Control flow/code layout and shared-host variation remain possible causes;
the synthetic duplicate-boundary fix is real but this corpus does not trigger
its final-table reduction, even where one table finishes at 102 distinct keys.

| Actual window (identical A/B) | Published table bytes | Arena lengths in bytes | Three-arena capacity bytes | All COLT pool retained bytes |
|---|---:|---:|---:|---:|
| cold 1..6 | 2,433,592 | 4,284,644 | 4,497,408 | 6,876,432 |
| cold 167..172 | 2,433,448 | 4,284,500 | 4,497,408 | 6,876,432 |
| cold 333..338 | 2,433,460 | 4,284,512 | 4,497,408 | 6,880,528 |
| cold empty | 2,401,304 | 4,229,756 | 4,423,816 | 6,795,880 |
| all four cached draws | 2,498,300 | 4,394,552 | 4,645,000 | 8,249,384 |

There are 1,828,452 bytes of retired construction content in occurrence 1's
single root: 100,000 rows, 43,236 distinct keys, 16,384 final groups.
Its published table is 2,401,168 bytes, arena lengths 4,229,620 bytes, and
three-arena capacity 4,423,680 bytes. Merely copying/truncating the tail would
not shrink these retained capacities, and this workload does not clone them.
Do not implement compaction just to celebrate shorter lengths: price the extra
copy against actual retained/cold/clone effects on a suitable workload first.

The larger new lead is **known-empty positive input**: even the empty draw
constructs that full occurrence-1 root. It requests 16,414,780 bytes in 95
events over the cold query window and retains 6,795,880 COLT-pool bytes,
despite occurrence 0's already-empty filtered view. These figures are measured
costs, not a prediction that every byte is avoidable. EMPTY-INPUT-LEAD.md
records the narrower general early-exit hypothesis and correctness obligations.

All temporary hooks are removed; the duplicate-only production and tests are
restored byte-for-byte to the unhooked gate-1 fingerprint above. Format passes
(session 90776). P2/P3 stay separate. No full trace/suite, compaction, commit,
push, release, version or SDK/persisted-layout change. M1 is correctness and
boundary-allocation verified; broad speed acceptance is not established.
