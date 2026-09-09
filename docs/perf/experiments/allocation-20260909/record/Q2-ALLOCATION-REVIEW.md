# Q2: packed WordMap allocation checkpoint

Q2 is isolated on frozen Q1. It is not performance-accepted, merged or
published. No full benchmark or CPU/allocation-site trace was run. Ordinary
Q2-vs-Q1 timing is still required. Gate and design details are in
Q2-DENSE-WORDMAP-LEAD.md.

## Matched evidence

`q2-controls-q1-1` completes 11:48:23 UTC; `q2-controls-q2-1` completes
11:49:52. Both build in the same disposable source path with separate fresh
target AND build directories, fresh allocation-enabled test artifacts and
distinct frozen executable hashes:

- Q1: `5e44826e2594a7f2985376ae124b1c2332deef3baafb5736714477e3f585aea7`
- Q2: `0d51fd0cc54100d494c0ecbb7d19822c2bdea9f0a073a351388982b033fc90f3`

The common-field packed-payload regression fails on Q1 with the expected
one-live-row/eight-value-slots assertion, and passes on Q2 across widths
0/1/2/8/9. Q2's permanent tests separately exercise payload pointer/capacity
stability on growth, insertion order, stale ordinals, rollover and panic.

Review 1 finishes 11:49:58, auditing all 41 cases, 328 allocation windows and
488 map-owner records per variant. All query outputs match independent SQL,
including changed parameters and release/refill. The new same-path Q1 run
reproduces the saved Q1 candidate's owners and request tuples exactly.

Every cumulative requested-minus-freed delta reconciles exactly with changed
map ownership, with zero unexplained residual. Answer, projection, spill-set,
aggregate/dense/fold and other aggregate-owner records are unchanged. Warm,
small-parameter, return and warm-refill request tuples are unchanged in every
case. This is not a claim that all those windows allocate zero.

## Actual ownership and first-use tradeoff

The fifth owner has deliberately changed meaning: Q1 retained a dense list
of sparse-slot indices; Q2 retains sparse slots containing dense row ordinals.
The review checks key/value live lengths, stamp/index lengths, mirror padding,
load bound and byte totals under the new schema. A zero-sized value's Vec
capacity is not bytes: its element width is zero.

| Saved case | Q1 retained map bytes | Q2 retained map bytes | First-use requested-byte change | Request-count change |
| --- | ---: | ---: | ---: | ---: |
| computed / union, 100,000 output rows | 9,961,479 | 5,242,887 | -9,437,104 | +1 |
| hashed groups, 8,192 rows | 622,599 | 327,687 | -589,776 | -1 |
| DNF / union groups, threshold 500 | 10,000,398 | 5,263,374 | -9,473,920 | 0 |
| interior, 100,000 rows, both maps | 19,922,958 | 10,485,774 | -18,874,208 | +2 |
| computed, 5 rows | 327 | 231 | -112 | +1 |
| recursive reach, parameter 63 | 718 | 526 | -352 | 0 |

The DNF-500 table covers 99,872 distinct input pairs plus 500 groups, not
just its 500 output rows. The 100,000-row two-word seen table still uses
524,288 probe slots, but only 200,000 live key words (262,144-word retained
capacity), versus Q1's 1,048,576 sparse key words. Retained table memory falls
from 9.5 MiB + 7 bytes to 5 MiB + 7 bytes, about 47.37%; first-use requests
fall by almost 9 MiB. Index growth no longer allocates or copies payload.

Across the 41 cases, retained map bytes and first-use requested bytes improve
in 29 and match in 12; none grow. Request counts increase in 17 cases (by one
or two), decrease in three and match in 21. Keep that event-count tradeoff.
The exact dense radix tables and point-path owners remain unchanged.
Preparation tuples also remain identical to Q1, which already removed the
speculative reservation. Q2 should not claim Q1's preparation win as its own.

These are actual Rust requested layouts and retained backing, not RSS, peak
process memory, OS resident pages, Pi qualification or proof of faster queries.
The extra ordinal load and per-row payload reservations still need ordinary
timing, including the earlier Q1 losing/warm controls. No allocation panel
repeat is needed for this exact source.

## Next controlled step

The fixed 18-case panel in `q2_time_experiment.py` was written before these
results. The build/timing/review scripts are prepared but have NOT run.
Mount only frozen Q1 plus the ordinary driver in the clean comparison tree,
build with both fresh Cargo caches, then do the same for exact Q2. Restore
that tree before running ABBA. Use the unchanged driver and preserve every
clock flag/tail. Compare Q2 against Q1 explicitly, not silently against v1.1.0.
Afterward interpret it alongside Q1's published-baseline comparison without
claiming cross-run ratios are a matched direct measurement.

No source edit is warranted merely to reduce the number of tiny requests
before pricing the larger representation change. Existing trace leads remain
open and full tracing stays disabled. No commits, pushes, tags or release.
