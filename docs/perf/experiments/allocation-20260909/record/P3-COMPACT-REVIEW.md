# P3 compact flat-end storage — memory benefit verified, speed acceptance open

All sessions are terminal. No temporary test hook remains; no full trace,
commit, push, release, tag or version change occurred. Primary P2 gate 6 was
revalidated unchanged at fingerprint
3bc43f3f04a09fa8983abeaf183eb54152fc8159a4a5b26fd73cbceb9f26958c.

## Isolated change and correctness

Only the detached p3-source worktree contains P3. <=128 interval groups now
append their real end words to the existing tree Vec, and the flat query arm
reads them from tree_base. Larger groups retain the same padded max-end tree.
No new owner, cutoff, allocator policy, public API or persisted layout. Filter
logic and output order are unchanged. Comments explicitly retain the two
inequalities even when combined query bounds are equal/reversed.

Both standalone baseline and compact gate 2 passed formatting, strict
all-target clippy, 1,323 library tests, 1,336 allocation-enabled library tests,
ordinary release build and 2,879 independent oracle cases. Both include the
new mixed-size/rehash/reset/order cache matrix and the real executor regression
for abutting/disjoint query constraints with spanning covers. The latter
checks examined rows as well as results, proving use of the index.

| Identity | Baseline | Compact |
|---|---|---|
| Gate | p3-gates-baseline-1 | p3-gates-compact-2 |
| Source fingerprint | 9eb992649743755f9e19d57b714e3b944d8ebc6ab523c70863b7c5554bd85336 | bed447ab25161b2ca64954468ce5a135df5fa08d2f2be6f27881d88591aafb7f |
| Ordinary binary SHA256 | 84b920460fe27b49181a8e8a444e6557971f559e96972fb5e5780f305d47cc74 | 0fbe16bd472018555ff63c6ac3eadaba4dbe67cf9317a69b78504d6cdd980e63 |

Compact gate 1's documentation-lint failure remains preserved. Gate 2 is the
successful authority; no failed attempt is relabeled as a pass.

## Actual memory result

See P3-OWNERSHIP-REVIEW.md and raw p3-owner-3/ and p3-owner-4/ logs. Cold,
warm and released/refill executions against byte-identical saved data all
return the independent SQLite Count=267603. Actual keys/group lengths match
every SQL count: 1,999 flat groups plus one large group. No store/oracle changed.

- The end/tree Vec falls from len/capacity 481536/524288 to 155144/172768
  u64 words. Capacity is measured, not inferred from powers of two.
- All nine cache-related owners together retain 6,299,680 -> 3,487,520 payload
  bytes: **2,812,160 fewer bytes (44.64%, about 2.68 MiB)**. All other owner
  lengths/capacities are identical. Live payload falls by 2,611,136 bytes.
- Cold/refill cache.probe request count is unchanged at 75, but requested
  bytes fall from 12,563,076 to 6,932,652. Old realloc layouts relinquished
  fall from 6,279,812 to 3,461,548 bytes. No separate dealloc calls.
- Warm requests/frees/bytes remain zero. Release drops all nine owners to
  zero and refill restores correct answers and compact ownership.

These are Rust request sizes and Vec payload capacity, not allocator usable
sizes, RSS, mmap residency, query peak, whole-database memory or Pi results.
Baseline audit 3 includes P2 outside the unchanged cache; only exactly matched
actual cache owners/groups and scoped cache.probe requests are compared with
standalone audit 4. Do not compare their whole-query counters. The ordinary
timing binaries below both exclude P1/P2.

## Ordinary ABBA result

Session 42138, p3-comparison-1/, completed at 07:59:35 UTC September 9, exit 0.
24 samples/8 warmups for each of five temporal queries, and 32 samples/batch 1
for seven read controls, in A0/B0/B1/A1 order. Scenario answers are oracle-gated;
read corpora are private and verified against their exact binaries. No full
suite, profiler, full trace or allocation instrumentation ran in this phase.
Source identity was revalidated after completion.

Geometric centers of the two round summaries, B/A (lower is better). This is
descriptive, not independent-sample significance. All rounds and tails remain.

| Family | p50 | Mean | p90 | Max |
|---|---:|---:|---:|---:|
| t1_stab | 1.0000 | 1.0131 | 1.0278 | 0.9500 |
| t2_overlap_join | 1.0336 | 1.0127 | 1.0116 | 0.9987 |
| t3_mixed_mask | 0.8930 | 1.0245 | 1.0337 | 0.9980 |
| t4_ray_stab | 0.9936 | 0.9760 | 0.9358 | 0.9373 |
| t5_pack_key | 1.0094 | 1.0183 | 1.0161 | 1.0048 |
| point | 0.9989 | 0.9906 | 0.9581 | 1.0009 |
| range | 1.0332 | 1.0431 | 1.0737 | 1.1480 |
| stats | 0.9886 | 1.0159 | 1.0548 | 1.2908 |
| mandate_overlap | 1.0033 | 0.9998 | 0.9998 | 0.9997 |
| conflict_pairs | 0.9863 | 0.9863 | 0.9861 | 0.9385 |
| conflict_free | 1.0268 | 1.0277 | 1.0492 | 1.0480 |
| slot_booking_overlap | 1.0100 | 1.0127 | 1.0115 | 1.0153 |

t2 medians (ms): A0=34.526542, B0=34.479458, B1=36.875750, A1=34.472250.
Means (ms): 35.095998, 35.263206, 36.576904, 35.832802. The first candidate
round is near parity; the second is slower. Aggregate median is +3.36%, mean
+1.27%. Similar minima are not proof that the slower candidate round was only
host noise. No scenario per-query clock stamps exist. This does not establish
a speedup or rule out a regression.

Range medians (us): A0/A1=4.959/4.917, B0/B1=5.250/4.958. The candidate is
not uniformly slower, but its +3.32% median/+4.31% mean control signal remains.
Stats B1 includes a 605.250 us max versus reference maxima 360.333/384.792 us;
its lower median must not conceal the tail. t3's favorable median is over
mixed parameters, while its mean/p90 worsen. Do not market it as a 10.7% win.
The other related control centers are near parity or modestly mixed.

All 28 read-engine and all 28 SQLite clock boundaries are unflagged, including
their existing harness retry behavior. No record was dropped or frequency-
normalized. Unflagged boundaries do not exclude pauses. QoS was verified;
macOS QoS is not hard P-core affinity and this does not qualify the Pi.

## Disposition and next discriminator

Keep compact P3 isolated and **not performance-accepted**. The memory reduction
is real and correctly scoped, but smaller storage did not establish faster
execution. Do not repeat an identical broad comparison looking for a win.

The saved t2 trace still has a separate 19.62% at flat cutoff/predicate/push
sites, directly reread this turn along with the full caller stacks. Next study
actual eligible-prefix versus emitted-hit patterns and ordinary code generation,
then test a bounded safe flat-filter compaction mechanism separately. Preserve
no-hit/early-cutoff/sparse/dense cases, exact order and equal/reversed bounds;
measure its additional writes and possible output-capacity growth, not just
branches removed in source. Keep current compact gate 2 as an immutable
reference if testing a subsequent filter variant. A targeted discrimination
must address the unresolved timing signal; no repeat full trace is needed.

P1/P2/M1/M2/M3 remain open. No exhaustion declaration or new full-trace authority.
