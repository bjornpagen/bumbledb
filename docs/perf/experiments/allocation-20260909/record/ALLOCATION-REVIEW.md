# Current allocation diagnostics

Source: `b02a641e087364ec09c161a97c67c87cf626e6b2`, unchanged.
Frozen counter executable SHA-256:
`aeda7dbe2456389e4bdb8f6bff01d9de60ac69ba533b262448e6afba2a2a28ef`.
These are Rust allocation requests, not physical allocator traffic, RSS or mmap
residency. The instrumented latency/frequency results are not timing authority.

## Read windows completed, 2026-09-09 02:19:03 UTC

Reviewed all 32 records from `allocations/reads/report.json`; every record
contains counters and batch=1. Ordinary/closure windows are 256 calls; each of
the six displaced windows is 12 calls, despite the global config.samples=256.

Every ordinary window contains one extra timing-vector allocation of 2,048
bytes; each displaced window contains one of 96 bytes. The raw records below
are unmodified. Counts include requested reallocations.

| Families | Calls | Raw requests / frees | Raw requested / freed bytes |
|---|---:|---:|---:|
| point, containment_walk, chain, balance, string, skew, entries_for_account_set, postings_without_tag, mandate_overlap, deep_chain, busy_scan, conflict_pairs, slot_scan, slot_booking_overlap, closure_depth, closure_fanout | 256 | 513 / 512 | 14,336 / 12,288 |
| range, triangle, mandate_at_instant, meets_chain, conflict_free, free_busy | 256 | 513 / 512 | 20,480 / 18,432 |
| stats | 256 | 513 / 512 | 10,240 / 8,192 |
| spread, latest_posting_per_account, rsvp_union | 256 | 257 / 256 | 8,192 / 6,144 |
| disp_probe, disp_probe_d24, disp_probe_d96, disp_stream, disp_stream_d24, disp_stream_d96 | 12 | 13 / 12 | 384 / 288 |

After accounting **only** for the independently audited harness vector, these
window averages are one or two requests and 24, 32, 48 or 72 requested bytes
per public operation. All corresponding bytes are freed in the window. This
does not measure the large owners retained from setup/warmup.

Fresh `bench_work()` constructs the shared `Arc<AtomicBool>` cancellation owner
inside every operation; parameterized reads also collect a temporary argument
vector. Empty parameter sets do not allocate that vector. These known owners
explain the regular pattern, pending direct site confirmation.

One useful exception is **stats**: it has no parameters but still requests
32 bytes per operation. Current `aggregate/spill.rs::for_each_ram_group`
allocates `vec![0u64; radixes.len()]` when iterating dense groups. Its one-word
key is a source-supported explanation for the extra eight-byte request, not
an excuse to prioritize it over the shared expensive join machinery. Check
dense multi-dimension scenarios too. This lead remains open for site/model
confirmation and a simple reusable-owner fix.

The expensive displaced probe and its one-relation scan control each request
only 24 bytes per call beyond harness overhead. These counters rule out a
large **warm per-row Rust allocation** problem in these measured windows.
They do not rule out cold/rebuild allocation, retained dead pool slots,
substantial memory reads/copies, or non-Rust allocation.

The broad command also ran its existing write rows before serializing the
read report; those write rows are not allocator windows. No supported long
workload was restored. Scenarios are running separately; the site census has
not yet started. The complete diagnostic phase must finish before engine edits.

The next continuation classified the previous turn as progress: the read
windows completed and their tiny warm counts changed the memory investigation
from per-row allocation to cold/rebuild and retained ownership. Original
allocation session 26191 and child 89679 were revalidated live, without restart.

The census is now queued as **session 23105**, wrapper PID 4525, under the same
measurement mutex. It cannot start its source check/build until that mutex is
released, and an additional `allocations/STATE.json == COMPLETE` prerequisite
refuses it if the counter phase failed. Outer log: `census-driver.log` (created
with shell no-clobber). Revalidate this handle; do not launch another census.

`probe_runs_tests.rs` is an ignored, rustfmt-checked but **uncompiled** candidate
test. It uses a shared store-free image and scalar differential probes across
widths 0/1/2/3/4/5/8, repeated/alternating tagged cursors, sparse unordered
survivors, both child modes, both carried modes, cold forcing and warm counter
reuse. It is not imported by the product or the frozen diagnostics yet.

## Complete counter roster, 2026-09-09 02:33:36 UTC

Session 26191 returned exit 0. `allocations/STATE.json` is COMPLETE, source
checks passed, and all 32+34 counter records and sample denominators were
validated. Reviewed **every** scenario counter, not only the slow families.
All scenario windows have 64 calls and one extra 512-byte sample-vector
allocation. Raw counts and requested bytes below are unmodified.

| Scenario families | Raw requests / frees | Raw requested / freed bytes |
|---|---:|---:|
| j1, j2, j6; g1, g2, g3, g4, g5; p1; r1, r2, r5; t1, t3, t4, t5 | 129 / 128 | 3,584 / 3,072 |
| j3, o6, p3, p4 | 129 / 128 | 5,120 / 4,608 |
| j4_five_way | 129 / 128 | 8,192 / 7,680 |
| g6_weighted_hop | 129 / 128 | 6,656 / 6,144 |
| j5_country_rollup, o1_revenue_by_region, o3_promo_split | 129 / 128 | 2,560 / 2,048 |
| o2_category_window | 193 / 192 | 5,632 / 5,120 |
| o4_segment_category | 129 / 128 | 3,072 / 2,560 |
| o5_store_extremes, r3_bomb_t1, r4_bomb_t2, r6_two_path_count, t2_overlap_join | 65 / 64 | 2,048 / 1,536 |
| p2_by_key | 385 / 384 | 8,608 / 8,096 |
| p5_keyed_get | 609 / 608 | 22,816 / 22,304 |

The ordinary expensive ring and overlap families have only one 24-byte request
per call after the known harness vector is separated. o4 has two requests / 40
bytes, consistent with cancellation plus its two-word dense key. j5/o1/o3 are
consistent with the single-word dense-key case observed in stats. o2 adds that
case to its parameters. These are source-supported patterns; the site census
and targeted owner tests remain the direct attribution checks.

p2 and p5 are exceptions to the one/two-request pattern: six requests / 126.5
bytes and 9.5 requests / 348.5 bytes per draw, respectively, after harness
accounting. They use different text-key and owned-fact-return contracts. Keep
their mixed hit/miss draws and exact caller-owned results visible; do not call
all of those requests needless internal allocation. They remain cheap-read
controls while the large CPU and cold-construction leads are investigated.

No operation's requested-byte balance grows over these warmed windows, aside
from the harness vectors retained through snapshot. This is not a census of
the already-live query/image/map owners, nor any claim about RSS on a Pi.

The queued census acquired the lock and started its profiling build at
02:33:37 UTC (child 10948). Session **23105** is now the authoritative live
diagnostic handle. Do not re-run either phase. Full CPU tracing stays disabled.

## Census fixture failure and scoped retry, 2026-09-09 02:40 UTC

Session 23105 exited 1. Its profiling build and symbol UUID check passed, but
the test exited 101 before any CENSUS/SITE window: the fixture referenced
Account.id without a declared key. The original `census/` directory is retained
unchanged. The engine has not changed. The fix appends the Account.id
functionality declaration (preserving the other statement indices) and adds
`census_fixture_smoke` to the normal test run. That check validates all schema
widths, populates the fixture, prepares the larger query shapes and executes
one draw of each execution shape without collecting backtraces. It passed in
0.22 seconds after compilation.

The separate `census-retry.py` writes `census-retry-1/`, records the full
test-only patch and fixture hash, and refuses any other tracked source change.
It repeats the smoke check before building/capturing and verifies source again
afterward. Active retry handle: session **33201**. Original failure and all
ordinary/counter baselines remain unchanged; no full tracing has started.

The isolated Item insertion fixture does not need seeded Account rows:
capacity constrains existing target rows, not containment of every source.
Confirmed in `schema/judge.rs::capacity`, whose second pass evaluates only
satisfying target rows. Do not change this fixture's workload unnecessarily.
