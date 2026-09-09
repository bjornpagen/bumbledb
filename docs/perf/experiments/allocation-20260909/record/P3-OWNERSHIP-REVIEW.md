# P3 saved-query ownership audit — complete; compact representation gating

Previous goal turn was progress: gate-6 source and the same-address layout
regression passed 1,329 library/294 allocator-enabled executor tests and 2,879
oracle cases; code generation and ordinary comparisons determined that the
cache split did not establish the main-query win. Read-only actual corpus
counts established the next structural question. P2 is still unaccepted.

No full trace is authorized. All prior sessions were terminal and gate-6
fingerprint was revalidated before mounting this diagnostic:
3bc43f3f04a09fa8983abeaf183eb54152fc8159a4a5b26fd73cbceb9f26958c.

Three temporary test-only modules are now mounted: overlap.rs observes
allocations scoped to cache.probe and exposes an out-of-window ownership
snapshot; run.rs exposes the executor's overlap owner; prepared/tests.rs runs
the actual saved query against a private byte-identical copy of the temporal
database. Hooks require both tests and alloc-counter (prepared/tests already
has test context). No ordinary release logic or on-disk format is changed.

The fixture executes cold, warm and after prepared.release_memory(). It checks
the independent SQLite overlap Count result, prints the actual plan and every
directory's complete key/length/tree offset, and compares actual per-key cache
groups with SQL counts. It records all nine owner lengths/capacities/element
sizes and scoped Rust allocation requests, outside timing. Warm storage must
reuse the same contents/capacities with zero cache-probe allocation. The release
snapshot must drop every overlap owner before refilling.

Whole-query allocation counters include public API setup and possible test
harness activity. Cache-probe counters include its feed closure (the current
suffix enumeration is borrowed). Counting instrumentation is NOT a timing
authority, allocator usable-size census, RSS, mmap or C allocation measure.

Attempt 1 (session 36213) is preserved in p3-owner-1/, terminal exit 1 at
07:33:09 UTC. Independent SQLite oracle/group extraction passed; the test
fixture did not compile because Allen is a struct variant, not a tuple variant.
No query execution or ownership result exists for this attempt. The fixture
was corrected to the source's CmpOp::Allen { mask } form; retry uses a fresh
phase. Do not overwrite or discard the failed build or its frozen fixture.

Attempt 2 (session 18374) is likewise preserved and terminal, exit 1 at
07:33:40 UTC. The remaining fixture compile error was the ambiguous empty
parameter array (BindValue and ParamArg both implement BindArgs). It now
uses the existing explicit &[] as &[BindValue] form. Attempt 3 will be fresh;
these failures are diagnostic fixture errors, not observed engine failures.

Remove the exact three module hooks and the allocation guard after terminal
completion, then revalidate the full gate-6 source fingerprint. Do not bundle
an unaccepted P3 production change into P2. Actual ownership evidence will
decide the representation experiment; the flat-filter CPU experiment remains
separate. The 128/129, same-start-order, rays and equal/reversed two-inequality
boundary matrix is still required before changing production overlap logic.

## Attempt 3 complete — actual ownership, not a projection

Session 30860 completed at 07:37:53 UTC, exit 0. Frozen instrumented binary
SHA256: f66d024adc4c2b4553bba96eeb1eae191f396374283285a4f3aaf79545c3b349.
The SQLite oracle and all three real-query executions return Count=267603.
Actual plan: cover occurrence 0 has trie levels [key] then [id, span]; the
other occurrence drives full [id, key, span] rows. Directory keys are [0,key].
Exactly 150,034 cache probes build all 2,000 groups, with no remaining tallies.
Every actual directory length agrees with the independent SQL count. All
saved original/private data.mdb and oracle file hashes remain unchanged.

Actual cold/warm/released-refill owner payloads (headers and allocator overhead
excluded; elements are shown as length/capacity):

| Owner | Elements | Element bytes | Retained payload bytes |
|---|---:|---:|---:|
| directory table | 4096/4096 | 4 | 16,384 |
| directories | 2000/2048 | 24 | 49,152 |
| directory keys | 4000/4096 | 8 | 32,768 |
| starts | 150034/157696 | 8 | 1,261,568 |
| positions | 150034/157696 | 4 | 630,784 |
| end tree | 481536/524288 | 8 | 4,194,304 |
| build triples | 80/4096 | 24 | 98,304 |
| last query hits | 74/4096 | 4 | 16,384 |
| lookup key | 2/4 | 8 | 32 |

Total: 5,751,312 live payload bytes, 6,299,680 retained-capacity payload bytes.
The 1,999 flat groups account for the previously projected 326,392 unused
internal/padding words (2,611,136 bytes). Their actual presence is now verified.
This does not predict the candidate's final Vec capacity: its growth sizes may
not be powers of two after using exact group lengths.

Cold AND released-refill cache.probe windows: 75 allocation/reallocation
requests, 12,563,076 requested bytes, 6,279,812 old bytes relinquished through
reallocations; no separate dealloc calls. Warm: zero requests/frees/bytes for
both the cache and the whole public read call. Reset retains equal cache
contents/capacities across executions. prepared.release_memory() drops all nine
cache-related owners to zero lengths/capacities; the next execution rebuilds
the same correct result and ownership. Peak/lifetime counters include the
out-of-window inspection clones and must not be labeled query peak/RSS.

All three module hooks and the probe allocation guard have been REMOVED from
the primary workspace. Its full gate-6 fingerprint revalidated exactly. No
ordinary production logic changed in this audit; no full trace ran.

## Independent P3 worktree and next experiment

Created a detached clean worktree from b02a641e at
bench-out/autoresearch-20260908.pMXtzv/p3-source. It contains no P1/P2 production
changes; the primary workspace retains the unaccepted gate-6 P2 unchanged.
Two permanent candidate regression files are added there: the cache boundary
matrix and a fixed equal/reversed-constraint executor case. Both pass against
unchanged production code, among 15 selected overlap tests (1 manual benchmark
ignored); session 6851 exited 0. The executor regression verifies not only
answers but that the second cover uses the combined cache filter (56 examined
rows rather than 80). The matrix checks exact emitted order, independent input
positions, empty/1/16/127/128/129/256/511 groups, growth and reset.

Session 66378 completed the unchanged standalone baseline via p3-gate.py
baseline 1, exit 0 at 07:46:08 UTC. Formatting, strict all-target clippy,
1,323 library tests, 1,336 allocation-enabled library tests, ordinary release
build and 2,879 independent oracle cases passed. Full evidence lives in
p3-gates-baseline-1/; scripts use the shared target directory SERIALly.
Source fingerprint: 9eb992649743755f9e19d57b714e3b944d8ebc6ab523c70863b7c5554bd85336.
Ordinary binary SHA256: 84b920460fe27b49181a8e8a444e6557971f559e96972fb5e5780f305d47cc74.

Selected isolated production change, now implemented in the P3 worktree only: the existing <=128
arm stores only its end words in the existing tree Vec; large groups retain
the exact padded max-end tree. The small query arm reads directly from that
group's tree_base rather than tree_base+p. Keep the cutoff, directory metadata,
ordering, exact two-inequality predicate and large walk unchanged. No new owner,
policy, unsafe code, public API or persisted-layout change. Compare actual
capacity/allocation requests and ordinary temporal/related controls; fewer live
words do not automatically prove fewer retained bytes or faster queries. Keep
the distinct flat-filter/compaction CPU idea separate from this representation.

Compact gate attempt 1 is preserved in p3-gates-compact-1/, session 77589,
terminal exit 1: strict clippy rejected missing Markdown backticks in the new
query documentation. No engine test failed; the gate stopped before tests.
The comment is corrected and fresh compact gate attempt 2 follows.

Compact gate 2 completed at 07:52:19 UTC, session 24723 exit 0. All the same
gates passed: formatting, strict all-target clippy, 1,323 ordinary library
tests, 1,336 allocation-enabled library tests, ordinary release build, and
2,879 independent oracle cases. Source fingerprint:
bed447ab25161b2ca64954468ce5a135df5fa08d2f2be6f27881d88591aafb7f.
Ordinary binary SHA256: 0fbe16bd472018555ff63c6ac3eadaba4dbe67cf9317a69b78504d6cdd980e63.
This is correctness acceptance only, not a memory/timing result. The same
temporary test-only ownership hooks are now mounted in the P3 worktree for
fresh candidate ownership audit 4; remove them and revalidate the unhooked
source fingerprint after its terminal completion. The primary P2 stays untouched.

## Compact audit 4 complete — actual retained capacity falls 44.64%

Session 73593 completed at 07:55:09 UTC, exit 0. All three passes returned the
independent Count=267603, with exactly the same 2,000 directory keys/lengths
and 150,034 probes. Frozen instrumented executable SHA256:
318e27615780ac1770360fcf062cb6d217d3b21c52637dd51697abed0351a475.
All original/private data.mdb and original oracle hashes remain unchanged.
The three temporary module hooks and probe guard have been removed; rustfmt
and the exact unhooked compact gate-2 fingerprint were revalidated. No hooks
remain in either worktree.

All eight other owner lengths/capacities/element sizes are identical to audit 3.
The end/tree Vec now has len=155144, capacity=172768, element size=8: exactly
1,241,152 live and 1,382,144 retained payload bytes. That capacity is observed,
not rounded or assumed from power-of-two growth.

| Cache measure | Prior actual | Compact actual | Reduction |
|---|---:|---:|---:|
| Total live payload bytes | 5,751,312 | 3,140,176 | 2,611,136 (45.40%) |
| Total retained payload bytes | 6,299,680 | 3,487,520 | 2,812,160 (44.64%) |
| Cold/refill probe allocation requests | 75 | 75 | 0 |
| Cold/refill probe requested bytes | 12,563,076 | 6,932,652 | 5,630,424 |
| Cold/refill old realloc bytes relinquished | 6,279,812 | 3,461,548 | 2,818,264 |
| Warm probe requests / bytes | 0 / 0 | 0 / 0 | unchanged |

No separate dealloc calls in the cold/refill probe windows. Release drops all
nine owners to zero; refill recreates identical compact ownership and answers.
The 2,812,160 retained-byte saving is about 2.68 MiB for this query's cache,
not whole-database memory, RSS, mmap residency, or allocator physical traffic.
Lifetime peaks include out-of-window observer clones, not a query-peak measure.

p3-ownership-compare.py independently verifies every measured flat/tree slab
is contiguous in physical build order, its final length matches the observed
owner, all directory keys/lengths/padded sizes match, and all non-tree owners
are unchanged. Its first analysis assertion incorrectly assumed directory
order equals slab-build order; directories are assigned on FIRST touch, slabs
on SECOND touch. The corrected checker sorts observed slab offsets before
checking contiguous physical coverage. Neither engine execution nor the raw
ownership evidence failed; both logs remain unchanged.

Comparison scope: audit 3 includes P2 aggregate changes outside the unchanged
cache, while audit 4 is standalone P3. Compare only the actual identical cache
owners/groups and scoped cache.probe requests, not whole-query counters across
those binaries. Ordinary CPU comparison instead uses the independent baseline
and compact gate-2 executables, with no P1/P2 changes on either side.

Session 42138 now runs p3-compare.py 2 1: bounded ABBA, all five temporal
queries (24 samples/8 warmups), plus point/range/stats and mandate_overlap,
conflict_pairs, conflict_free, slot_booking_overlap read controls (32 samples,
batch one). Private, binary-bound oracle-verified read corpora; fresh temporal
scenario corpora per round. Keep source frozen. This is not a full benchmark
or any trace. Memory improvement is established; timing acceptance is pending.

ABBA session 42138 is now terminal, exit 0 at 07:59:35 UTC. All oracle gates,
rounds and source checks passed. P3-COMPACT-REVIEW.md is the latest authority:
retained cache capacity is down 44.64%, but t2 is +3.36% median/+1.27% mean
in the descriptive round centers, with a slower second candidate round and
an unresolved range control. No clock boundary was flagged; do not infer that
all within-window behavior is clean. No speed acceptance or identical broad
rerun is justified. All hooks remain removed and primary P2 is unchanged.
