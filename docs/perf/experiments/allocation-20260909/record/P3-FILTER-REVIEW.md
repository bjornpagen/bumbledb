# P3 flat-filter demand and block experiment

Previous goal turn was progress: compact storage passed correctness and reduced
actual retained cache payload by 44.64%, while ordinary timing failed to prove
a speedup. This turn does not rerun a full trace or the same broad comparison.
Primary P2 remains separate and unaccepted. No release/commit/push/version bump.

## Actual query demand, not a projected distribution

p3-demand-1/ completed 08:06:01 UTC September 9, session 55009 exit 0. It is
a DEBUG, untimed, test-only structural observation followed by exact replay,
not a CPU trace, timing benchmark, allocation counter window or RSS census.
Independent SQLite Count=267603 matches both cold and warm actual queries.
Original/private data.mdb and original oracle hashes remain unchanged.

Source before/after the temporary hooks:
bed447ab25161b2ca64954468ce5a135df5fa08d2f2be6f27881d88591aafb7f.
Instrumented debug executable SHA256:
a4152cbe00d365b4738e5d5f020e78dd37b9ab2b2611de53ab2afe32a6d9ed28.
Exported flat-input.bin SHA256:
3f7f8e24b3cf120dbfe27cbe8723c89b4bb827cdadf5817e542bff5ce726f60f.

The observer preallocates its event vector outside the operation and refuses
to grow it during recording. It records actual indexed-query arguments, hit
counts and output capacities AFTER the executor calls query_into. After the
query it replays the stable, fully built cache and checks exact ordered results
and every actual baseline output capacity, not just total rows.

Both passes execute 148,034 indexed queries: 144,953 flat and 3,081 large.
The 2,000 first-probe declines are intentionally not indexed-query samples.
Cold/warm argument/order/hit schedules match exactly.

| Flat demand per actual execution | Observed |
|---|---:|
| Sum of full group lengths across calls | 10,802,768 positions |
| Sum of eligible start-prefix lengths | 5,613,018 positions |
| Emitted positions before downstream exact residuals | 424,209 |
| Calls with early start cutoff | 139,681 |
| Calls with no hits | 0 |
| Calls where every eligible position hits | 2,402 |
| Adjacent end-predicate transitions within eligible prefixes | 349,002 |

Thus about 92.44% of eligible positions are rejected. The ~19.62% saved trace
at cutoff/predicate/push sites does not imply random 50/50 hit branches, or
justify copying all prefixes. These counts are specific to this real workload;
synthetic no-hit/dense/early-exit controls remain necessary for other callers.

Direct safe prefix-copy/compaction replay, with identical ordered output:

- Cold actual append capacity finishes at 4096 u32 entries; compaction at
  6016. Each replay has six growth events, but maximum extra candidate capacity
  is 1920 entries (7680 payload bytes). Twenty-five flat calls have a prefix
  longer than the actual append owner's capacity at that time. This is measured
  Vec capacity evolution, not allocator request counts or physical traffic.
- The warm replay starts BOTH outputs at the ACTUAL baseline's 4096 entries,
  and neither grows. This is an equal-starting-capacity replay, NOT proof that
  a candidate warmed from its own cold run would forget its 6016-entry capacity.
- Replay includes large queries to preserve their effect on output growth;
  the native micro input export intentionally contains only flat groups/calls.

All three module hooks and the executor observation statement were removed
after the terminal result; exact compact source identity was revalidated.

## Matched native mechanism 1 — search and copying do not earn promotion

p3-filter-mechanism-1/, session 38830 exit 0, completed 08:09:27 UTC. Three
non-inlined kernels, with equal slice-length guards, opaque function-pointer
dispatch and equal 4096-entry warm output capacity. Guards prevent independent
slice arguments from artificially adding per-position bounds checks only to
the baseline. Actual 144,953-call order is repeated eight times per block;
controls use 131,072 calls per block. Exact naive ordered results are checked
outside timing on real inputs and 216 synthetic edge cases per three kernels.

A=existing branchy loop; B=binary-search prefix then append-only end filter;
C=binary-search prefix, copy it, safely compact and truncate.

| Case | B/A | C/A |
|---|---:|---:|
| Actual saved flat order | 1.1157 | 1.5788 |
| Empty | 1.0387 | 1.0226 |
| Singleton | 1.0780 | 1.6770 |
| Early none | 1.7830 | 1.8315 |
| Early two | 1.5070 | 1.8927 |
| No hits | 0.9563 | 1.5089 |
| All hits | 1.0124 | 1.0643 |
| Alternating | 0.9278 | 1.2874 |
| Sparse | 0.9770 | 1.4929 |
| Reversed bounds | 1.2011 | 2.4660 |

All 60 blocks retained, including 18 clock flags (four of the six actual-input
blocks). These are descriptive geometric centers of ns/call per block, not
independent-sample significance or production speed estimates. No dropped,
retried or frequency-normalized windows. Neither form earns a production
change. Copying also has the explicit cold output-capacity penalty above.

## Mechanism 2 — skip redundant start checks in full eight-row blocks

p3-filter-mechanism-2/, session 40806 exit 0, completed 08:11:24 UTC. Same
input, matched guards, protocol and controls, with a new hypothesis:

A=same branchy loop; B=check the last sorted start of an eight-row block, then
keep ordinary conditional append for its ends; C=same block check, but form
an eight-bit end mask and emit set bits in order. Both finish any partial
block/tail with the existing scalar two-inequality test. Neither copies a
prefix or adds an output owner. No unsafe code or architecture intrinsics.

| Case | B/A | C/A |
|---|---:|---:|
| Actual saved flat order | 0.8963 | 0.9732 |
| Empty | 1.1674 | 1.2355 |
| Singleton | 1.1594 | 1.1525 |
| Early none | 1.3808 | 1.5157 |
| Early two | 1.1867 | 1.2860 |
| No hits | 0.6573 | 0.2719 |
| All hits | 0.7525 | 1.2387 |
| Alternating | 0.6015 | 1.0081 |
| Sparse | 0.7121 | 0.3699 |
| Reversed bounds | 0.9461 | 0.9331 |

All 60 blocks retained. Three flags: actual B0 (3.070912/3.352736 GHz proxy),
no-hits C1 (3.087373/3.271493), all-hits C1 (3.056780/3.358601). Actual B1
is unflagged and 45.8688 ns/call versus A1 49.5623; A0/B0 are 53.2716/46.2389.
The two baseline blocks differ, so a 10.37% geometric-center mechanism signal
is not a precision production promise. Early-none B adds about 1 ns per
standalone call; other tiny controls also lose. These regressions are retained
and require real caller controls, not dismissal because the main input wins.
The mask form loses densely matching calls and has a weaker real-input signal;
do not select it just because its synthetic all-reject case is fast.

## Selected production candidate: block8, correctness gating

Only the P3 worktree now contains the B mechanism on top of compact storage.
The large-tree arm, cutoff=128, directory metadata, keying and output owner are
unchanged. Starts are sorted: last_start < q_end proves all eight starts pass.
When that test fails, the scalar tail begins at the SAME block base, so no
partial block is skipped. End tests and ascending-position emission remain
identical, including equal/reversed bounds. There is no new allocation path.

Permanent regression lengths now include 7/8/9, 15/16/17 and 31/32/33 as well
as 0/1/127/128/129/256/511, both growth orders. They also assert exact end-slab
length so flat groups cannot silently regain padding. The existing real
multi-constraint executor regression remains.

Block8 gate 1 (session 59864) failed strict clippy before tests: this repository
requires as_chunks::<8>() for a constant chunk size. The saved failure remains
in p3-gates-block8-1/. Production now uses the equivalent fixed-array iterator;
the mechanism binary used chunks_exact(8), so ordinary built-code measurements,
not its exact ratio, must decide the final implementation.

Gate 2 (session 86477) also stopped before tests: the expanded regression was
102 lines against the repository's 100-line lint. The end-slab assertion now
lives in its build helper, keeping that invariant next to construction instead
of suppressing the lint. Both failed artifacts are preserved unchanged.
Fresh gate 3 follows in p3-gates-block8-3/. Keep source frozen while running.
No P3 variant is performance-accepted yet; no full trace is authorized.

## Gate 3 complete; ordinary comparison running

Gate 3, session 5622, completed 08:20:39 UTC, exit 0. Formatting, strict
all-target clippy, 1,323 library tests, 1,336 allocation-enabled library tests,
ordinary release build and 2,879 independent oracle cases all pass.
Source fingerprint: 3f89f14dfea19b2ae5a58bc3244e8022b0ea20131a39e5e7b87786902163f7a6.
Ordinary binary SHA256: 3fd1ed105d37974ba1e8e1c4dad449442fc07d6faed9d1c2c2669bd6e910d390.

Static p3-codegen-1/ completed 08:21:42 UTC without running any workload.
query_into is inlined into overlap_enumerate in BOTH compact and block builds.
The containing function's frame remains 0x170 (368) bytes, with the same six
saved register pairs. The block build visibly checks the eighth start once,
then has eight unrolled end tests and ordered conditional pushes; growth calls
remain on capacity-miss paths. A per-block start-index bounds check remains,
while individual position bounds checks have been eliminated in this ordinary
build. Its containing function grows from 5212 to 5976 text bytes, so code-size
effects are not assumed free. Source/micro idiom differences do not stand in
for ordinary measurements. All raw symbols and disassembly are preserved.

p3-comparison-2/ now runs the extended p3-compare.py with block8 gate 3 and
compact gate 2 in ABC/CBA order: A=standalone published baseline,
B=compact storage, C=compact storage plus block filtering. Same ordinary
five-query temporal lane and seven ledger/calendar controls as comparison 1.
This is a new production mechanism comparison, not an identical broad rerun.
Keep all three frozen executables, all round/tail/clock results and the current
source intact until terminal. No full suite, sampling trace or profiler runs.

## Ordinary comparison 2 complete — block8 does not earn promotion

Session 95023 completed 08:28:52 UTC September 9, exit 0. All six rounds,
scenario oracles, private read-corpus verification and terminal source checks
passed. Source fingerprint revalidated as
3f89f14dfea19b2ae5a58bc3244e8022b0ea20131a39e5e7b87786902163f7a6.
p3-timing-review.py now reads the recorded order rather than assuming ABBA;
it validated/reviewed all 72 ordinary distributions and all 84 read clock
boundaries, including both engine and SQLite, with all retries retained.

A=published baseline, B=compact, C=compact+block8. Ratios are geometric centers
of the two round summaries, not independent-sample confidence or significance.
Lower is better. Every round, tail and flag remains in p3-comparison-2/.

| Family | C/A p50 | C/A mean | C/A p90 | C/A max | C/B p50 | C/B mean | C/B max |
|---|---:|---:|---:|---:|---:|---:|---:|
| t1_stab | 1.0426 | 0.9915 | 0.9894 | 1.0084 | 1.0437 | 0.9882 | 0.9580 |
| t2_overlap_join | 0.9685 | 0.9733 | 0.9346 | 1.1303 | 0.9968 | 1.0057 | 1.2250 |
| t3_mixed_mask | 1.0140 | 1.6484 | 1.2336 | 3.7157 | 1.2097 | 1.6642 | 3.8326 |
| t4_ray_stab | 0.9844 | 0.8887 | 0.7439 | 0.7375 | 0.9922 | 0.9913 | 0.9388 |
| t5_pack_key | 1.0284 | 1.0051 | 0.9957 | 1.0373 | 0.9753 | 0.9821 | 0.9592 |
| point | 1.0000 | 0.9741 | 1.0000 | 0.8944 | 0.9203 | 0.9404 | 0.9014 |
| range | 1.0124 | 1.0134 | 1.0165 | 0.9842 | 0.9916 | 0.9864 | 0.9765 |
| stats | 1.0161 | 1.0193 | 0.9522 | 1.0876 | 1.0017 | 1.0638 | 1.6694 |
| mandate_overlap | 0.9829 | 1.0090 | 1.0087 | 1.1266 | 1.0272 | 1.0441 | 1.1736 |
| conflict_pairs | 1.3244 | 1.0099 | 0.9850 | 1.1120 | 1.3545 | 1.0718 | 1.1812 |
| conflict_free | 1.0000 | 0.9906 | 0.9768 | 0.9349 | 1.0000 | 1.0122 | 1.0232 |
| slot_booking_overlap | 1.1235 | 1.0935 | 0.9821 | 1.6984 | 1.0674 | 1.1170 | 1.8399 |

The block mechanism's t2 medians are 35.123208/35.739958 ms, versus compact's
35.767542/35.324708 ms. The directions reverse; C/B is -0.32% median,
+0.57% mean and +22.50% max. C1 reaches 60.610459 ms. This does not establish
the standalone micro's ~10% benefit as an engine benefit. C/A's -3.15% median
is not an isolated block-filter saving: B/A is already -2.84%. Published A
itself moves from 38.036750 to 35.184750 ms. Compact was +3.36% versus A in
comparison 1; the sign reversal does not license selecting only this run.

Mixed-mask t3 is the strongest adverse signal. C0/C1 means are
1.804718/1.601270 ms versus B0/B1 1.023350/1.019637 ms and A0/A1
1.040892/1.021718 ms. C0 has a 22.859791 ms max; C1 has a 10.817959 ms max
and 6.203459 ms p90. Its roughly unchanged C/A median hides the problem,
since the scenario rotates keys 0/1/5/1000000 of very different costs. There
are no per-query scenario clock stamps or per-parameter raw samples. Do not
attribute these tails to the block loop, the host, or a specific key yet.

Other non-neutral controls remain: C0 conflict_pairs median 27.750 us versus
C1 15.458 us; slot_booking_overlap C0 max 128.625 us versus C1 36.416 us;
stats C1 max 786.250 us. C's mandate_overlap mean is higher than B in both
rounds (10.997/10.971 versus 10.502/10.539 us). The apparently favorable t4
mean/tail ratio is amplified by A1's 32.750 us max and 17.265 us mean. t1's
C0 500 ns median versus the other rounds' 458/459 ns is visible even though
mean/tails are mixed. Range remains slightly slower versus A, not resolved by
a favorable C/B ratio. No center substitutes for these individual observations.

Three engine clock boundaries are flagged after the harness's existing retry:
B0 point (3.098/3.198), B0 range (3.264/3.191), C0 stats (3.257/3.073).
Two SQLite boundaries are flagged: B0 range (3.188/3.254), B0 mandate_overlap
(3.408/3.176). All 12 retry-marked boundaries, including seven subsequently
unflagged boundaries, are reported by the reviewer. No new retry, dropped
record, frequency normalization, profiler or full benchmark was introduced.

Disposition: keep block8 frozen and UNACCEPTED. Do not repeat this broad
comparison or substitute the microbenchmark for the failed engine discriminator.
Compact storage's measured cache-memory benefit remains separate; CPU acceptance
of either P3 variant is still open. Primary P2 is untouched and unaccepted.

Next bounded discriminator: ownership audit 5 on the exact block8 source, plus
an untimed actual mixed-mask branch census on the existing saved corpus. The
latter runs the original four-key parameter order twice, compares every pair
against independent SQL, and counts actual flat/tree calls without collecting
CPU samples. This separates executed flat-filter demand from the large-group
path before attributing the mixed-parameter tail. These temporary hooks are
mounted only after session 95023 completed. Remove them and revalidate the
ordinary source fingerprint when the diagnostic becomes terminal.

## Audit 5 complete — identical compact ownership; t3 paths separated

p3-owner-5/, session 99035, completed 08:32:58 UTC, exit 0. Both the existing
cold/warm/release-refill audit and the new mixed-mask exact-pair test pass.
Instrumented executable SHA256:
75c17a636177f7eebe2cc98b3333e287eaab4a99a646f9c75e2b29bce96f82e9.
Independent mixed-pair CSV SHA256:
109ee2762e20dbff495431c10f67478805b537480b33db54a7da6985eccb3133.
Original/private data.mdb and original oracle hashes remain unchanged.

Actual block8 cache ownership is identical to compact audit 4 in all three
passes, including every owner's length/capacity, all groups, and scoped probe
allocation costs. p3-ownership-compare.py 4 5 --baseline-compact now enforces
that equality. Comparing audit 3 with 5 preserves the previously scoped
2,812,160-byte / 44.64% retained cache-payload reduction, with no P2/P3
whole-query comparison. The tree owner remains 155144/172768 u64 words,
hits remain capacity 4096, cold/refill probes request 6,932,652 bytes in 75
requests, warm requests/frees/bytes remain zero, and release empties all nine
owners before correct refill. Count=267603 in every actual t2 pass.
These are requested sizes and Vec payload capacities, not RSS, physical mmap
residency, allocator usable sizes, whole-query peak or Pi qualification.

The mixed-mask test uses the original query, one prepared owner, and the same
parameter order twice. Every actual ordered pair (sorted outside execution)
matches the independent SQL DURING-or-MEETS query; duplicate/lost pairs would
fail. Fixed-size thread-local counters observe query_into's actual flat/tree
arm selection. No event vectors, backtraces, sampling, CPU timing or profiler.

| Parameter key | Answer pairs | Flat calls | Large-tree calls | Cache probes | Built group length |
|---|---:|---:|---:|---:|---:|
| 0 | 105,992 | 0 | 3,081 | 3,082 | 3,082 |
| 1 | 82 | 73 | 0 | 74 | 74 |
| 5 | 99 | 77 | 0 | 78 | 78 |
| 1,000,000 | 0 | 0 | 0 | 0 | not rebuilt |

Both parameter cycles have identical counts. For the absent key, the snapshot
still contains the preceding key-5 scratch group, but there are ZERO probes
or indexed queries; this is retained scratch, not evidence of querying stale
data. Do not infer current execution from the last owner's contents alone.

This disproves attributing key 0's work to executing the new flat loop. It
does not identify which key owned each original timing tail (those samples
were not retained by parameter), rule out binary-layout/code-size effects,
or prove host pauses. The containing function grew from 5212 to 5976 text
bytes despite its unchanged 368-byte frame. Source and arm selection cannot
turn an unexplained timing regression into acceptance.

All three module hooks, the probe guard and the two-word query counter hook
have been REMOVED. Rustfmt and full P3 source fingerprint revalidation pass:
3f89f14dfea19b2ae5a58bc3244e8022b0ea20131a39e5e7b87786902163f7a6.
Primary P2 also revalidates unchanged at
3bc43f3f04a09fa8983abeaf183eb54152fc8159a4a5b26fd73cbceb9f26958c.
Every build/measurement/diagnostic session is terminal. No production changes
were made in this continuation; the isolated block8 remains unaccepted.

Next work must not rerun the same broad protocol or tweak block size until it
wins. Any timing follow-up on t3 needs explicit per-parameter samples and
interleaved controls with matched ordinary engines; the current scenario
report pools the four parameter sets. Another independent supported lead is
M1 construction-table ownership: first quantify duplicate-triggered growth,
then separately evaluate retained retired tables and same-shape copying per
MAP-OWNERSHIP-LEAD.md. Do not bundle P1/P2/P3 candidates into that experiment.
The existing evidence remains unexhausted; full tracing stays disabled.
