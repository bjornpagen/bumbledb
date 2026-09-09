# Saved-trace exhaustion ledger

The user prohibits another full trace until existing evidence has been milked
for supported improvements. Both `diagnostics.py trace-*` phases refuse to run.
This ledger is **not** an exhaustion declaration. P1 has an unaccepted local
candidate; all remaining candidates are open. No new full CPU capture launched.

## Current checkpoint — September 9, Q3 implemented and correctness-gated

This continuation completed Q2's ordinary review/closeout and arity-two static
audit, then implemented the supported Q3 cleanup in q3-source, leaving Q2,
Q1, all six prior identities and the primary tree unchanged. The user asked
for a recap and to continue; the no-new-full-trace instruction still applies.

Q3-DENSE-PUBLICATION.md is the current authority. One values.len replaces the
separate map row count; internal entry returns an ordinal, so seen sets do
not borrow discarded values; value reserve precedes a single key extension.
No growth/probe/load law, allocator policy, public API or disk layout change.
Generic mutable payloads remain for ResolveMemo and hashed groups.

Gate 1 failed compilation because a new sibling test could not access the
private entry helper; preserved. Gate 2 completed 12:16:17 UTC: 28 map tests,
67 sink tests, strict lint, 1,329 ordinary and 1,342 allocation-enabled library
tests, fresh optimized build and 2,879 independent oracle cases pass. Source
6bc03e29d2e426a50995b5945efe2d1761966d816601f06a046edb183d39ff3b;
executable f1899d6607176e3c2968c9955a34cccda0b975a5fafa3ee0ac89cb6f5c5a608a.
Both Cargo caches are fresh and separately isolated.

q3-codegen-1 exports 21 functions from the Q2/Q3 ordinary GATE binaries.
Arity-two scalar/bulk duplicate-value bounds checks, second row-count stores,
and duplicate key reservations are gone. Scalar's copy helper call and both
paths' ZST overflow checks remain. Hot function sizes shrink but a separate
unit-value grow body appears; total executable file size rises 128 bytes.
This is static evidence of the intended deletion, NOT proof of speedup.

NEXT: exact Q3 allocation-owner/counter comparison and targeted ordinary
affected/negative controls. Use the existing common comparison checkout and
driver; gate-binary timings cannot substitute for same-driver timings. Keep
Q2 frozen and preserve adverse results. Q2's owner helper accesses self.len;
use a separately frozen common helper calling len() for Q2/Q3 rather than
editing the old evidence. Text memo coverage remains open and can reuse the
saved Ledger/SQLite data. No full trace/benchmark, commit/push or release.
Goal active, incomplete, not blocked; saved evidence remains unexhausted.

## Previous checkpoint — September 9, Q2 ordinary timing reviewed

The preceding turn completed the Q2 numeric ordinary panel and began static
inspection. This continuation finishes the interpretation and follows the
exact two-word scalar/bulk insertion branches. Authorities:
Q2-ORDINARY-TIMING-REVIEW.md, Q2-INSERTION-CODE-REVIEW.md and
Q2-CONSUMER-AUDIT.md. No new full trace or identical panel rerun occurred.

q2-timing-1 completes 12:00:12 UTC: 72 fixed ABBA processes, all SQL checks
pass. Review 1 independently verifies 432 distributions, 50,688 raw values
and 216 clock brackets; all 46 flags and slow samples remain. Same-path fresh
Q1/Q2 builds pass format/lint/build with independent target AND build caches.
Q1 executable 44593bfb044e0330120229828f9c22b78ca987724721a3928d2ff587c691e8fc;
Q2 f9369dda9fbd15b5485e4eb040f3407f92adfb72532f43166798b1b3378a1e85.
Comparison source is clean at published HEAD; Q2 and all prior identities
remain unchanged. Rebuilt Q1 is not bit-identical to its older timing binary.

Q2 remains NOT performance-accepted. Written-union500 combined p50/mean
improve 15.14%/14.23%, all cold brackets unflagged. But DNF500 same-target warm
loses 7.00%/4.59%, written-union500 alternating loses 12.66%/10.67%, and
computed100k alternating loses 5.10%/4.09%: all three lose both directions
with no clock flags. Do not hide these behind cold/memory wins. Union100k's
apparent 34% cold win and DNF alternating apparent 28% win are contaminated.
Triangle is roughly flat; range warm improvements bypass the changed map.

Static arity-two review establishes unused ZST value-reference bounds checks,
duplicate row-count stores and redundant key-capacity checks. Scalar calls
an out-of-line copy helper; bulk already inlines its two-word copy. These are
leads, not dynamic cycle attribution. Warm allocation tuples are unchanged.
ResolveMemo is a third real WordMap consumer with mutable text ranges; the
numeric panel does not cover its owners/performance. Saved text corpus can
close that gap without a new corpus or full trace.

NEXT: freeze this Q2 checkpoint, then isolate Q3 single-source dense row
publication (one cardinality, ordinal-returning internal entry, one key
reservation). Preserve generic payloads and safe access. Gate correctness,
inspect fresh ordinary code, and only then price targeted affected/negative
controls and text outputs. No commits, pushes, releases, full benchmark or
trace. Goal active/incomplete; the saved evidence is not exhausted.

## Previous checkpoint — September 9, Q2 correctness and allocation complete

The preceding turn implemented Q2 and started its correctness gates after
deriving Q1 growth costs from saved owners. This continuation completes its
gates and the matched allocation comparison. Q2-ALLOCATION-REVIEW.md and
Q2-DENSE-WORDMAP-LEAD.md are the live authorities. No new full trace, benchmark,
commit, push or release occurred. The primary tree and all prior candidates
remain unchanged; Q2 stays isolated and NOT performance-accepted.

Q2 packs key/value payload in insertion order; sparse slots hold row ordinals.
It preserves the probe/generation/load/doubling laws. Rehash rebuilds only
the index, leaving payload pointers and capacities untouched. Payload access
is safe Rust, deleting WordMap's MaybeUninit/unsafe handling. No SDK, storage
format, quota, allocator mode or fallback was introduced.

Gate 1's test-conversion lint failure remains preserved. Gate 2 completes
11:47:20 UTC: focused tests, strict lint, 1,327 ordinary and 1,340 allocation
library tests, ordinary release build and all 2,879 independent oracle cases
pass. Exact source fingerprint is
`dc78c0d7123b10edc7f9df4751a9d3a4792b65e4a428458f639c8a9d6e936f3c`.

Matched Q1/Q2 controls finish 11:48:23/11:49:52. Both use the same comparison
source path with fresh target AND build caches, and distinct fresh executables.
A shared representation regression fails on Q1 as intended and passes Q2.
Review 1 completes 11:49:58: all 41 cases, 328 windows and 488 map records per
variant reconcile. Q1 reproduces its previous saved observations exactly.
The fifth-owner schema transition is explicit; all other recorded owners and
all warm/changed-parameter request tuples match. No residual byte difference.

100,000-row result-map backing falls 9,961,479 -> 5,242,887 bytes, with
9,437,104 fewer first-use requested bytes but one additional request. DNF-500
maps fall 10,000,398 -> 5,263,374 bytes; groups-8192 622,599 -> 327,687.
29 cases use fewer retained/requested bytes and 12 match; request counts grow
in 17 cases, fall in three, match in 21. No RSS/peak/Pi/speed claim follows.

NEXT: the predeclared ordinary 18-case Q2-vs-Q1 panel, including DNF/union/
groups-8192 losses and range/point/tiny/empty controls. Scripts are prepared,
but no Q2 ordinary timing build or panel has run. Use the clean comparison
tree, unchanged ordinary driver, separate fresh Cargo caches, all clock flags
and tails. Do not rerun the identical allocation controls or older Q1 timing
panel. Goal active/incomplete, evidence not exhausted; full tracing remains
guarded. The comparison's temporary mounts are removed; frozen copies remain.

## Previous checkpoint — September 9, Q1 ordinary timing complete

The preceding turn made progress on broader Q1 allocation controls. This
continuation completes the fixed ordinary panel without a new engine edit,
full benchmark or trace. Q1-ORDINARY-TIMING-REVIEW.md is the live authority.

Baseline timing build 2 and candidate build 1 (61664) both pass format, strict
bin lint and ordinary release build. They use the same checkout path/driver,
fresh target AND build directories, freshly compiled ordinary engine artifacts
and distinct executable hashes. Baseline build 1's fixture-lint failure stays
preserved. The temporary comparison edits are removed BEFORE timing; Q1 remains
exact gate-2 source and all six prior candidate fingerprints are unchanged.

q1-timing-1 (59889) completes 11:27:39 UTC, exit 0: 28 fixed case/draw ABBA
comparisons, 112 processes, all exact numeric SQL checks passing. Review 1
completes 11:27:44, independently checking 672 distributions, 78,848 raw values
and 336 clock brackets. All 26 flagged brackets and every slow sample remain.
No retries, normalization, sample removal or repeat-until-win.

Triangle preparation medians fall from about 1.00–1.10 ms to 21.7–38.8 us;
combined prepare+first centers improve 7.8–10.3%. This prices the saved ~80 MiB
backing deletion, not a similarly large warm gain or RSS/Pi claim. Adverse
unflagged controls are real: DNF-500 combined p50/mean +16.51%/+10.38%, union
100000 +4.52%/+5.62%, written-union groups 500 +7.86%/+12.09% with order drift.
Groups-8192 same-target warm +5.58%/+5.96%; every range draw has a small adverse
warm median even though execution allocation tuples are unchanged. Q1 remains
isolated and NOT performance-accepted. Do not hide losses in one overall score.

Warm samples cross the predeclared generation-rollover index 248. Baseline
triangle there is 2.36–8.93% over its median, not its dominant maximum;
candidate and unrelated control tails remain visible. No automatic event
attribution from index coincidence. All 224 warm tail-index summaries survive.

NEXT: derive actual extra growth/rehash work from saved DNF-500, union-100000
and groups-8192 map owners/counts, then discriminate final geometry/history
and removed runtime PlanNode/code-layout effects on warmed controls. A minimal
count-only diagnostic is permitted if needed; no full trace/corpus or identical
timing/allocation-panel rerun. Do not restore speculative join/batch hints.
Keep other measured allocation/CPU leads open. Goal active, incomplete, not
blocked; no commit, push, release, tag or version bump.

## Previous checkpoint — September 9, Q1 broader controls complete

The preceding goal turn was progress: Q1 passed correctness gates and the
saved 32-window allocation audit. This turn completes its broader allocation
controls without ordinary engine changes or another full trace. The current
authority is Q1-BROADER-CONTROLS.md.

Two initial harness problems were caught and preserved. Baseline-1's identical
union arms normalized to a single proven-distinct rule, leaving a coverage
gap. Candidate-1 reused the byte-identical baseline executable from the shared
Cargo cache and is INVALID-BASELINE-EXECUTABLE-REUSED. Neither is a valid
matched comparison. New runs use distinct overlapping union arms plus runtime
shape assertions, isolated CARGO_TARGET_DIR AND CARGO_BUILD_BUILD_DIR, fresh
engine artifacts, distinct binary hashes and candidate empty-map assertions.

Baseline-2 completes 11:05:52 UTC (28582), candidate-2 11:07:03 (82651).
q1-controls-review-1 verifies 41 case/draw controls, 328 paired windows and
488 map records per variant, plus all other recorded owners. Every numeric
answer matches handwritten read-only SQLite, including after changed params
and release/refill. Cumulative net bytes reconcile with observed maps plus
constant small plan metadata. Prior invalid evidence is explicitly marked.

Small-result capacity benefits survive, but 15 cases request MORE bytes in
prepare+cold. For 100,000 outputs, computed/union/interior add about 2.36 MB
of requested bytes while ending with the same table capacities; hashed groups
add about 295 KB. A one-group hot-account DNF/union still dedups 50,365 input
pairs and adds about 2.06 MB despite returning one row. Do not confuse result
groups with input dedup cardinality or hide these adverse growth costs.

Warm/changed-param/return/refill tuples match baseline exactly, not universally
zero: dense has an existing 8-byte request, interior+reach 1,534 bytes/six
requests per warm call. Exact dense radix [3]/12-byte table is unchanged.
These small pre-existing leads are below pricing the measured large tradeoff.
No speed/RSS/peak/Pi claim follows from these allocation results.

NEXT: ordinary matched timing INCLUDING preparation, cold, second, same-target
and alternating-target warm behavior. Sampling must cross the old u8 result
map generation rollover; short pre-rollover windows could miss a large control
array clear. Use frozen shared cases and exact saved triangle/point inputs,
fresh per-build target AND build directories, all raw tails/clock flags and
verified QoS. No identical allocation rerun, full benchmark/trace, commit/push
or release. Q1 remains isolated, research active and evidence unexhausted.

q1-controls-closeout-1 completes 11:11:36 UTC (27351): exact unmounted Q1
gate-2 identity restored; new baseline worktree clean at published HEAD;
both format/diff checks pass; all six prior identities and matched helpers/
binaries verified. All sessions terminal. Timing has not started yet.

## Previous checkpoint — September 9, Q1 implementation and saved audit complete

Q1-DEMAND-GROWTH-REVIEW.md is the current authority. The previous continuation
implemented deletion of speculative allocation hints and passed 67 focused
tests; this continuation resolved the strict-lint failure by deleting the
now-unused estimate accessor/runtime fields (planning estimates preserved).
q1-gates-1 remains preserved as failed, not green.

q1-gates-2 completes 10:54:58 UTC (45625): format, 67 focused tests, strict
engine/benchmark lint, 1,324 ordinary and 1,337 allocation-enabled library
tests pass (18 ignored each), along with ordinary release build and all
2,879 independent oracle cases. Exact dense radixes/dedup proofs, existing
WordMap growth/reset/release, SDKs and persisted layout are unchanged.

q1-audit-1 completes 10:55:38 (29469). q1-audit-review-1 checks all 32 saved
SQL execution windows, 100 active/parked views, 64 spares and two independent
preparation windows. Result-map backing falls 83,886,119 → 199 bytes for
five rows; fresh empty/unexecuted maps own zero backing. Other measured
owners match baseline exactly. All warm/second-rotation allocations stay zero.

Account for relocated work: each nonempty first execution adds six requests,
254 requested and 87 freed bytes. Prepare PLUS first execution adds two
requests overall while saving 83,885,881 requested bytes. Range preparation
saves 589,847 requested bytes; its execution is unchanged and that saving is
mostly transient, not retained. These are capacities/layouts, not RSS or Pi
results, and no ordinary speed comparison has run for Q1 yet.

The candidate preparation observer omits only two post-snapshot printing
lines for deleted runtime estimates. All original helpers/hashes are preserved
and all other observation/input files match. Q1 remains isolated/unaccepted;
all six prior candidates remain unchanged. No commits/pushes/releases/tags.

NEXT: price high-cardinality/union/recursive and hashed/dense aggregate controls,
then matched ordinary timings INCLUDING preparation, cold, second and warmed
costs. The old e1_timing driver excludes preparation and is insufficient on
its own. Do not rerun G2 for prettier timing. Full-trace guards remain enabled;
the saved evidence is not exhausted and the goal is active/incomplete.

q1-candidate-closeout-1 completes 10:56:04 UTC (56182): all five temporary
observer mounts removed; exact gate-2 source fingerprint restored; format
and diff checks pass; all six prior identities and frozen helpers verified.
All tool sessions are terminal. Nothing committed, pushed or released.

## Previous checkpoint — September 9, Q1 preparation owner discovered

G2 ordinary timing is fully documented; g2-closeout-3 verifies all six exact
candidate identities and unmounted gate-3 source, format/diff checks passing.
No identical timing panel was rerun. No full trace or full benchmark launched.

Q1-OWNERSHIP-REVIEW.md is the new evidence authority. An isolated baseline
q1-source with read-only test mounts completed 32 exact saved SQL windows
(session 85028, 10:38:26 UTC), then a separate two-query preparation-only
audit (43856, 10:40:06). Review checks every answer/sink owner, all 100
active/parked views and 64 spares. All 16 overlapping triangle allocation
tuples exactly reproduce the frozen published baseline. Ordinary engine code
has not changed in Q1; these are owner/counter diagnostics, not speed results.

The largest discovery is **83,886,087 bytes of result-hash backing allocated
during preparation**, before any execution, for the saved five-result/empty
triangle. The last join estimate is 2,400,000; its capped hint causes 2^23
table slots. The range query allocates then discards its hinted backing when
the valid distinctness proof selects dense output. Q1-HINTED-ALLOCATION-LEAD.md
selects removal of this speculative allocation route using existing ordinary
insertion-driven WordMap growth. This is not yet implemented. Preserve exact
dedup proofs, recursive deltas and dense-group radixes; price prepare PLUS
cold execution, including high-cardinality controls and possible extra growth.

Smaller leads are now measured: Cell is 24 bytes; range retains 96,000 decoded
cell bytes plus 32,768 dense sink bytes. Filtered bindings retain 400,000
position bytes each (1,600,000 across four cached draws). First-predicate
survivors are often much larger than final survivors, so incremental reserve
alone may not help. Do not double-count positions already in COLT totals.
None of these requested layouts/capacities are physical RSS or Pi results.

No commits, pushes, releases, tags or version bumps. Existing evidence remains
unexhausted. NEXT: the Q1 hinted-allocation experiment, ahead of speculative
output-Cell refactoring or a small filter-reserve tweak. Full tracing disabled.

q1-closeout-1 completes 10:43:32 (63304): all observer mounts removed,
q1-source clean at published HEAD, format/diff checks pass, frozen observation
dependencies verified and P2/P3/M1/E1/G1/G2 fingerprints unchanged. Every
session is terminal; the goal remains active, incomplete and not blocked.

## Previous checkpoint — September 9, G2 ordinary timing complete

`G2-PER-DRAW-TIMING.md` records the completed ordinary panel and full review.
Both builds used the same path/frozen driver and only ordinary grow.rs differs.
Exact gate-3 source and all prior candidates were restored/verified before
timing (g2-closeout-2). Panel 1 completes 10:27:02 UTC (session 66246); review
completes 10:27:08. All 128 distributions and 96 clocks were checked, with
25 flags, zero retries and no discarded samples. No new full trace/benchmark.

Nonempty cold triangle median changes -1.29%, -0.58%, -5.30% do not establish
a clean causal speed win. Triangle 0 second-execution mean/max are +2.14%/
+13.35%; point 1 cold and same-target have unflagged adverse controls. Empty
triangle apparent warmed wins are confounded by slow/flagged baseline A1.
G2 measured memory savings remain valid; ordinary timing is mixed/noisy and
the candidate stays isolated/not performance-accepted. Do not rerun the same
panel for prettier data. No commits, pushes, releases, tags or version bumps.

NEXT: keep mining saved evidence. The filter kernel reserves one u32 per
input row even for sparse/empty survivors; separately observe its ownership
before choosing a refactor, and price dense controls. The historical range
trace also points to output gathering/finalization; audit against current
source before ranking it, since earlier result-path rewrites already landed.
Neither lead is implemented or exhausted. Both full-trace entry points stay
disabled; Mac payload/timing results are not RSS/peak/Pi qualification.

## Previous checkpoint — September 9, G2 implementation and allocation audit complete

This goal turn made progress: the isolated staged-entry growth candidate now
passes correctness gates and a matched saved-corpus allocation/owner audit.
`G2-STAGED-GROWTH-REVIEW.md` is the latest authority. No new full trace or full
benchmark was run, and both full-trace entry points remain disabled.

G2 replaces append/rehash with one unpublished-tail growth algorithm using
the existing scratch owner. It checks all growth sizes first, stages keys and
packed children in dense order, reserves before mutation, initializes/clears
cooperatively, rewrites dense ordinals in place and restores cleared scratch
on every Result path. force.rs, duplicate policy, public SDKs and disk layout
are unchanged. Real force rollback preserves older published maps/children.

Gate 3 completes 10:15:52 UTC (session 75303): 47 focused tests, strict lint,
1,329 ordinary and 1,342 allocation-enabled library tests (18 ignored each),
release build and 2,879 independent oracle cases pass. New failure tests cover
140 checkpoint refusals/retries across cold/retained pools, partial large
control clearing, packed children, old tokens, clones, reset/release and
overflow ordering. Initial fixture/compiler/lint failures remain preserved.

The matched saved audit completes for G2 at 10:17:20 (83794), baseline at
10:18:05 (69070), G1 at 10:18:52 (80302), and its full review at 10:19:30.
All 20 SQL windows and 78 active/parked owners in each variant are checked,
including scratch. Baseline/G1 reproduce every original allocation and owner
record exactly. All map shapes/bytes are equal and every other owner is
unchanged after subtracting arenas plus scratch. Every G2 retired arena
content disappears. All clones and warm/repeat/second-rotation requests are zero.

Each nonempty cold draw saves one request, 2,194,384 requested bytes and
1,549,248 retained bytes (~22.5%) versus published. Four cached draws save
1,619,840 retained bytes. The large root matches the projected four-owner
capacity exactly: 2,909,792 bytes including 419,424 bytes of scratch.
G2 has three MORE cold requests than G1. Its small index retains 1,568 more
bytes than G1, although whole cold-query retention is 693,184 lower.
Do not hide these adverse tradeoffs or add isolated E1/G2 savings.

g2-closeout-1 completes 10:19:33 (77984): all temporary observer mounts removed,
exact gate-3 fingerprint c759483862ce7a957dcda1a39e4246b36fe47fd1e1eb5614dfa82bd725f70e04,
format/diff checks pass, and all five prior candidates are unchanged. All
sessions terminal. No commits, pushes, releases, tags or version bumps.

NEXT: ordinary matched per-draw timings using the existing external driver
and exact saved inputs. The measured memory savings are not a speed result,
RSS/peak measurement or Pi qualification. G2 remains isolated/unaccepted.
Saved evidence is not exhausted; no full trace is authorized.

## Previous checkpoint — September 9, G1 timing complete; G2 source lead

The preceding goal turn was progress: G1 retention regression/correctness
gates and actual allocation savings completed. This turn revalidated all
five candidates and priced G1 with matched ordinary binaries on the exact
saved corpus. No new full trace or full benchmark was needed or run.

G1-PER-DRAW-TIMING.md is the timing authority. Comparison 1 completed
09:54:12 UTC (session 25892); review at 09:54:24 recomputed all 128
distributions and checked all 96 independent clock brackets. Four flags
remain, no retries or dropped samples. Nonempty cold p50 changes -4.49%,
+1.42%, +1.71% do not prove a consistent speed win. Warm means/tails and
point controls remain mixed, including unflagged adverse windows. G1's
actual memory saving remains valid, but it stays isolated/unaccepted.
Do not repeat the identical panel hoping for acceptance.

G2-STAGED-GROWTH-LEAD.md records the next source-supported experiment:
existing rehash scratch is used only in grow.rs, so it may stage dense
occupied key/child records while growing the unpublished table at fixed
bases. One construction-only growth algorithm; no new owner, allocator,
mode, quota or SDK/layout change. Its synthetic published-map growth test
must be reframed around the actual rollback/published-prefix contract,
not silently retained to justify unused duplicate production paths.

g2-geometry-1 counts all three arenas PLUS the enlarged scratch. For the
saved root only, it projects 2,909,792 combined retained bytes versus G1's
3,604,544, with one additional request but fewer requested bytes and no
whole-table relocation. This is arithmetic, NOT implemented/measured G2.
Next create a separate red/green staging/rollback/ownership experiment,
preserving all frozen candidates and failed/noisy evidence.

g1-closeout-2 verifies the exact restored gate-2 fingerprint and unchanged
P2/P3/M1/E1, with format/diff checks passing. No timing/observer Cargo hooks
remain. All sessions terminal. No commit, push, release or version bump.
Saved evidence is still unexhausted; full-trace entry points stay disabled.

## Previous checkpoint — September 9, G1 saved allocation audit complete

G1-GROWTH-REVIEW.md is the latest continuation authority. The existing saved
root's growth model exactly matches its measured lengths/capacities. An
isolated G1 candidate now reclaims only unpublished construction tails after
each growth, with bounded cooperative copying. Generic grow_map, load factor,
allocator, published maps, public SDKs and persisted layout are unchanged.

Baseline retention discriminator fails as intended; multi-map/order/clone/
reset controls pass. Gate 2 completes 09:45:04 UTC (session 24937): format,
strict engine/benchmark lint, 1,325 ordinary and 1,338 allocation-enabled
library tests (18 ignored each), release build, and 2,879 independent oracle
cases. Failed lint attempt 1 is preserved, not counted as a correctness win.

Actual saved audit 1 completes 09:46:09 UTC (session 91139), its review at
09:46:15. All 20 SQL windows and 78 owner pairs checked. All live map shapes
are unchanged; all retired map contents disappear. Each nonempty cold draw
saves 1,155,072 requested bytes and 856,064 retained COLT-pool bytes. Four
cached draws save 929,792 retained bytes. Warm/repeat and second-rotation
windows still allocate nothing; all clone counters are zero. These are actual
requested layouts/payload capacities, NOT RSS, peak memory or a speed result.

G1 remains isolated and not performance-accepted. Its additional copying must
be priced with bounded ordinary per-draw timings, not inferred from smaller
owners. Do not add isolated E1/G1 savings together. Temporary observer hooks
are removed; g1-closeout-1 verifies the exact gate-2 fingerprint and unchanged
P2/P3/M1/E1, with diff/format checks passing. All sessions are terminal.
No new full trace, full benchmark, commit, push or release. Saved
leads are still unexhausted; both full-trace entry points remain disabled.

## Previous checkpoint — September 9, completed E1 per-draw timing

E1-PER-DRAW-TIMING.md is the latest timing authority. Ordinary ABBA comparison
1 completed 09:28:21 UTC (session 63397), raw-sample review 09:28:39. All 128
distributions and 96 independent clock brackets checked; eight flags remain,
no retries or dropped samples. Fresh empty medians fall 8.56–9.10 to
4.18–4.20 ms; warmed empty executions fall from about 1.49 ms to below 1 us.
Nonempty triangle centers are close, but adverse point controls and some tails
prevent a universal performance-green claim. Do not rerun the same panel merely
to get cleaner results. "Memoized" is warmed query state, NOT cached answers.

E1 gate-1 source is restored exactly, with no observer/temporary Cargo hook.
P2/P3/M1 remain separate and unchanged. All sessions terminal. No full trace,
full benchmark, commit, push or release. The next open ownership question is
growth-time retention of obsolete construction tables: final tail compaction
alone was already shown insufficient to shrink capacities. Read grow_map and
its rollback/published-map contracts before choosing an experiment. Existing
trace/allocation evidence is NOT exhausted.

## Previous checkpoint — September 9, 09:19 UTC

E1-EMPTY-INPUT-REVIEW.md is the latest continuation authority. The isolated
entry-first regression reproduces unnecessary maps on both empty filtered
inputs and empty relations. A common post-bind positive-empty check now avoids
join execution, preserving negation, discharged roles and select's cursor
contract. Both it and the existing selection-miss return flush cancellation;
the latter has its own red/green discriminator at the internal join seam.

Gate 1 completes (session 96873): seven focused tests, format, strict engine/
benchmark lint, 1,328 ordinary library tests, 1,341 allocation-enabled tests
(18 ignored each), release build and 2,879 independent query-oracle cases pass.
New test-fixture mistakes and the passing non-discriminating fixtures were
compared against baseline and retained as failed evidence, not product wins.

Saved ownership audit 1 completes (session 54064). It uses the exact same
saved corpus, test, observer and schema as the M1 published baseline. All 20
SQL windows and 78 owner pairs were reviewed. For the fresh cold empty draw,
requests fall 95 -> 18, requested bytes 16,414,780 -> 5,300,324, and retained
COLT-pool bytes 6,795,880 -> 400,048. Every nonempty allocation tuple remains
identical. Warm/repeat and second-rotation requests are still zero. In the
mixed rotating workload the large root is already legitimately built; its
empty draw saves just 4 requests / 2,568 bytes. Do not generalize cold savings.

All E1 hooks removed; ordinary source matches gate 1 exactly:
e44e04132d30992ab094d61c93d620207175e1390006046e53733c16fb52cd55.
P2/P3/M1 identities revalidated unchanged. All sessions terminal. No ordinary
E1 timings have run, so no speed acceptance. Next use per-draw ordinary timings
with matched nonempty/point controls, not pooled curves or another full trace.
The allocation savings are not RSS or Pi qualification. No commits, pushes,
release, version bump or new full trace. Saved evidence remains unexhausted.

## Previous checkpoint — September 9, 08:55 UTC

M1-CONSTRUCTION-REVIEW.md remains the live authority. Gate 1 and bounded
ordinary construction/probe ABBA (session 22822) complete. All 64 timing
distributions and 16 clock brackets were reviewed; two retry flags and two
contaminated brackets remain visible. Cold triangle centers improve about
7% but warm controls are mixed: no universal speed acceptance or rerun loop.

Actual saved-corpus audits now complete for candidate (session 52231) and
baseline (60350): all 20 exact SQL windows and 78 active/parked snapshots per
variant pass. The entire ownership/allocation/clone records are IDENTICAL.
Thus the cold timing signal is not an allocation-saving result on this corpus.
All clones count zero; warmed and second-rotation executions allocate nothing.
Synthetic boundary savings remain valid, distinct evidence. Failed diagnostic
attempt 1 (missing import) is preserved separately, never counted as a test win.

The actual dominant retired root contains 1,828,452 dead arena bytes but has
no cloning here; tail compaction alone would retain its large capacities.
More promising general lead: an already-empty positive view still allows a
100,000-row table to be forced elsewhere in the rule. EMPTY-INPUT-LEAD.md
records a narrow common-seam early-exit experiment and correctness controls;
it is not implemented, and measured current cost is not promised savings.

All temporary M1 hooks are removed. Unhooked source matches gate 1, format
passes, P2/P3 remain separate and unchanged. All sessions terminal. No new
full trace/full benchmark, commit, push, release or version bump. No lead is
declared exhausted merely because a candidate is deferred or timing is noisy.

## Previous checkpoint — September 9, 08:47 UTC

M1-CONSTRUCTION-REVIEW.md is the latest continuation authority. The isolated
duplicate-growth fix has now reproduced the old-policy failure, passed all
28 synthetic ownership/model cells and completed gate 1 (session 45189).
Format, strict engine+benchmark lint, 1,323 library tests, 1,336
allocation-enabled tests, six displaced tests and 2,879 independent oracle
cases pass. Both library invocations retain 18 ignored tests. No timing claim.

In the affected 25-key synthetic cells, groups halve (16 to 8), cold requests
fall 24 to 20, and clone requested bytes fall without changing its request
count. All zero-width/26-key negative controls are exactly unchanged; all
reset/reforce windows allocate nothing. These are requested/payload bytes,
not RSS or actual workload prevalence. The source/model fix remains isolated
in m1-source; P2/P3 are untouched. Retired construction ownership is still a
separate open experiment. All sessions are terminal. No new full trace,
full suite, commit, push or release occurred; saved evidence is not exhausted.

## Previous checkpoint — September 9, 08:33 UTC

P3-FILTER-REVIEW.md is the latest continuation authority. Ordinary comparison
2 (ABC/CBA, session 95023) and actual ownership/path audit 5 (session 99035)
are complete. All six ordinary rounds, 72 distributions and 84 read clock
boundaries were reviewed without dropping tails or flags. Block8 does not
establish a t2 gain over compact: -0.32% p50, +0.57% mean, +22.50% max; t3
has a material adverse mean/tail signal. Keep it unaccepted and isolated.

Audit 5 confirms exactly the compact cache ownership: 2,812,160 fewer retained
payload bytes than the prior tree representation (44.64%), no additional probe
requests, and zero warm requests. Exact SQL pair checks show t3 key 0 returns
105,992 pairs through 3,081 large-tree calls and ZERO flat calls; keys 1/5 use
the flat arm. This separates paths, not timing causation or acceptance. Every
temporary hook is removed and P3/P2 source fingerprints revalidate unchanged.

No full trace, release, tag, publication, version bump, commit or push occurred.
Do not repeat an identical broad comparison seeking a win. Any t3 follow-up
must retain per-parameter samples and matched interleaved controls. M1's
duplicate-growth/retired-construction ownership discriminators are also still
open; no trace-exhaustion declaration is justified. All sessions are terminal.

## Earlier checkpoints

Latest continuation: the complete ordinary ABC/CBA comparison is reviewed in
`P1-VARIANTS-REVIEW.md`. Neither probe form cleared all controls; preserve and
defer them rather than repeat identical broad loops. Only the experimental
production probe hunk was restored, retaining its regression test. P2 now
starts standalone on the baseline kernel. Its first experiment borrows
witnessed aggregate inputs without new routing metadata; the ownership
accounting in `P2-IMPLEMENTATION-SKETCH.md` explains why the larger dedup
scratch consolidation is not bundled. No lead is marked exhausted by deferral.

P2 update: initial borrowed inputs pass correctness but the focused wide-row
test proves linear source lookup can regress 128-word grouping by about 5.1x.
The compact slot-indexed routing refinement passes gate 4, including the 2,879
independent oracle cases, and is now in focused mechanism validation. See
P2-COMPACT-REVIEW.md for exact source/binary identity and current sessions.
It remains unaccepted; no additional full trace is authorized or needed.

Latest P2 update: the compact lookup removes the wide cliff, but the completed
ABC/CBA ordinary comparison gives back about 11% versus the initial getter on
o4 and leaves noisy/losing controls. Neither form is accepted. The in-place
route-reader refinement removes per-batch cache take/restore without a width
heuristic, and gate 5 is in progress. P2-INPLACE-REVIEW.md is the live authority;
P2-THREE-VARIANTS-REVIEW.md records every control and both candidates' disposition.

In-place follow-up: gate 5 passes 1,328 library tests, 293 allocation-enabled
executor tests and 2,879 oracle cases. Focused attempt 4 completes with the wide
cliff absent, but singleton comparisons confound cache ownership with batch
dispatch. The matched-dispatch four-arm discriminator now completes with all
144 sample blocks retained (16 boundary flags). Cache movement is mostly a
few percent; production dispatch adds roughly 9-17% in several narrow singleton
cases. Static ordinary gate-5 inspection shows a 112-byte/six-register-pair
rebuild frame on cache hits. Gate 6 separates the exact hot check from that
body and adds a same-address changed-layout test. See
P2-INPLACE-MECHANISM-REVIEW.md; no temporary hook remains. No form is
performance-accepted, and none of the other saved leads is exhausted.

Gate 6 update: all correctness/lint/build gates and 2,879 independent oracle
cases pass. Static inspection confirms both cache callers skip the separate
112-byte rebuild frame on hits, with emit_batch's own frame unchanged. A
bounded ordinary published/gate-5/gate-6 OLAP-and-control comparison follows;
P2-SPLIT-REVIEW.md is the latest authority. Full tracing remains disabled.

Comparison 3 completes, but the split does not demonstrate an o4 benefit over
gate 5 (+0.63% p50 / -0.94% mean), and P2's scan/tail controls remain unresolved.
The exact published scan control varies from 87.587 to 131.779 ms median,
with no boundary flags; flags on three other read windows are retained. Do not
repeat the same broad loop or infer a clean win from mixed central statistics.
Gate 6 remains frozen and unaccepted; all sessions are terminal and no hook
is mounted. See P2-SPLIT-REVIEW.md for the complete result/disposition.

The read-only saved temporal corpus count now measures 1,999 flat-size groups
(46-106 rows) plus one 3,082-row group. The unused small-tree representation
projects 2,611,136 live payload bytes of avoidable internal nodes/padding if all
full groups are built. This is NOT observed cache capacity or RSS. The next
discriminator must measure actual executor/cache ownership and build coverage;
p3-saved-structure-1/ and OVERLAP-LEAD.md record the inputs and caveats. No P3
production optimization is bundled into unaccepted P2, and all leads stay open.

## Shared CPU leaders

P3 update: actual owner audit 3 confirmed the saved temporal query builds all
2,000 projected groups. A standalone worktree now stores only real ends for
<=128 groups, preserving the existing larger trees. Two permanent regressions
cover mixed slab sizes/order/reset/rehash and equal/reversed constraint windows.
Compact gate 2 passes strict format/lint, 1,323 library tests, 1,336 allocation
tests and 2,879 oracle cases. Audit 4 measures 2,812,160 fewer retained payload
bytes (44.64% of these cache owners), unchanged request count and zero warm
requests. See P3-OWNERSHIP-REVIEW.md for exact ownership scope and source hashes.
No temporary hooks remain. Ordinary standalone ABBA is running in
p3-comparison-1/; the candidate is NOT yet performance-accepted. P2 remains
unchanged and unaccepted in the primary workspace. Flat-filter CPU work and
all other saved leads remain open; no full trace is authorized.

P3 ABBA has now completed, all sessions terminal. Actual cache capacity is
44.64% lower, but ordinary t2 centers are +3.36% median/+1.27% mean and range
remains a losing control. No clock boundaries were flagged; no records were
dropped. P3-COMPACT-REVIEW.md records the complete result, hashes and limits.
Compact P3 is still isolated and not performance-accepted. Next discriminate
the separate saved flat-filter CPU lead; do not repeat identical broad loops
or treat a storage saving as a proven speedup. Primary P2 remains unchanged.

Next P3 turn made progress: actual demand capture/replay proves 5,613,018
eligible positions but only 424,209 emitted across 144,953 flat queries.
Prefix copying raises cold output capacity to 6016 from 4096 entries and loses
its matched native mechanism comparison. Binary-search prefix alone also loses
the real-input mechanism. Eight-row sorted-start checks with ordinary append
show the stronger signal; the mask variant loses dense-hit controls. Every
block/clock flag is retained in p3-filter-mechanism-1/ and -2/. These are
bounded targeted diagnostics, not full traces or engine speed acceptance.
P3-FILTER-REVIEW.md is now the live authority. P3 worktree block8 gate 2 is in
progress with additional block-boundary/slab-size regressions; primary P2 is
unchanged. No new owner, allocator policy, public API, storage layout or unsafe
code. All other saved leads remain open and tracing stays disabled.

### P1 — repeated prepared-probe setup in adjacent carried-cursor runs

Full newer r4 caller paths were reviewed again, not just the top-symbol list:
`native-reads/r4_bomb_t2/whale-analyzed.summary.json` under
`bench-out/autoresearch-87-df737a6f`.
The expensive path is `run_pipeline -> pump -> probe_pass -> pump -> probe_pass
-> probe_sibling_batch<1,true> -> probe_sibling_keys`, not final result output.
Nearest exclusive owners include callback 16.69%, probe walk 16.09%, routing
12.95%, gather/hash 11.57%, copy-map 8.13%, and already-forced `force` 3.14%.
These shares are historical attribution, not predicted speedups.

`probe_pass.rs`, `colt/probe.rs`, and `colt/prefetch.rs` are unchanged since that
capture. Force/grow changed allocation-policy plumbing, but still perform the
same hash-table construction. The current ordinary r4 median is 1.525 s.
Its source documents about 57 million closing probes at m=384. This is a
general batched-join routing candidate, not a workload-specific triangle rule.

First experiment should retain the exact existing whole-batch prefetch pass
and reuse a prepared borrowed probe only across adjacent equal **tagged**
cursors. That isolates setup amortization from prefetch-policy changes.
Drop the borrow before forcing the next cursor; retain survivor order, masks,
original child indices, exact refusal prefix, empty/dynamic/wide widths and
presence-only no-child-load behavior. See `PROBE-LEAD.md` for discriminators.
Only consider further Row/Map dispatch hoisting after measuring that step.

2026-09-09: the 54-window symbolized allocation census completed, reviewed in
`CENSUS-REVIEW.md`. P1 still ranks first for CPU. The new differential test and
existing sibling/refusal tests passed against unchanged engine source (four
selected tests). Its first draft failed compilation because View deliberately
has no Clone; corrected to the existing explicit clone_in API. The candidate
now reuses a prepared probe across adjacent equal tagged cursors, preserving
the complete prefetch pass. Whole-library correctness is running before any
timing acceptance. No speedup or allocation improvement is claimed yet.

Later September 9 update: the complete P1 A0/A1/B0/A2/B1/A3 scenario/read
comparison finished, including 2,879 oracle cases per binary, library and
allocation gates. r3/r4 repeat roughly 7–10% candidate gains, but P1 remains
unaccepted pending its noisy controls. `P1-REVIEW.md` preserves the full
central/tail/clock review, including contaminated A2 controls rather than
crediting their large apparent wins. The large B0 j4 loss did not repeat.

The saved-corpus structural audit now proves undisplaced probe executes
524,288 root sibling keys and **zero carried keys** on each cold/warm pass.
Its 1,024 outputs agree with a direct independent sum of the corpus generator.
The width-one per-key callbacks' normal lookup instructions are unchanged in
the frozen binaries; containing frames/register choices changed. Do not
attribute root timing variation to executing the new carried-run loop.
A bounded ordinary ABBA of undisplaced probe plus four cheap controls is in
`p1-focused-1/`; it is now terminal and did not clear the candidate's signals.
The current worktree is a subsequent iterator-based P1 refinement, with full
library/allocation/lint/build gates passed but no performance acceptance.
`P1-ITERATOR-REVIEW.md` is the latest continuation state and explicitly records
the mixed code-generation result. All measurement/build sessions are terminal.
No full trace is needed or authorized by these investigations.

### P2 — grouped fold routing/materialization

Reviewed the newer o4 caller stacks through pinned-leaf emission, not just
aggregate symbol names. Its main exclusive owners include fold_batch_rows
9.91%, load_group_key 6.64%, refresh_shape_cache 6.23%, fold_scratch_row 5.98%
and probe_group 3.46%. The key groups and batch-fold implementation are still
present; sink setup/reset and old budget pressure checks have changed.
Therefore historical maybe_spill_groups time is not a present-day estimate.
The current o4 median is 27.213 ms over a three-way, 64-group rollup.
Check one-row leaf batches, constant-group dispatch, unnecessary binding copies
and cached shape lookup before introducing any new planner or storage policy.

The next continuation read the current scalar/batch/scan fold paths and the
full pinned-leaf callers. `AGGREGATE-LEAD.md` records a falsifiable borrowed
row-input experiment plus dedup/float/Pack/shape-reset obligations. No aggregate
change is bundled into P1's frozen evaluation.

### P3 — interval enumeration

The newer t2 caller/source review separates OverlapCache::query_into (25.86%),
lookup (7.33%), Walk::report (6.78%) and overlap_gather (6.29%). Its
`interval/overlap.rs` and `run/overlap_leaf.rs` are unchanged from the capture.
The current t2 median is 41.386 ms. Cache directories and result hits are rebuilt
within each execution, reusing vector capacity. Candidate questions: repeated
key hashing, index build/flat-sweep representation, and full-hit staging versus
bounded iteration. Preserve start-order, exact residual recheck, multiple Allen
constraints, finite/infinite ends, abutting intervals, cancellation and source
view identity. Do not call a query with one count row a one-row operation.

Line-level review now narrows 19.62% of total saved t2 CPU to the **flat**
len<=128 filter's cutoff/predicate/push lines (177/180/181), not the max-end
tree. `OVERLAP-LEAD.md` records branchy append versus safe compaction as an
isolated experiment, including additional-prefix-capacity and sparse-query
tradeoffs. No overlap change or new capture was made.

## Memory/ownership leaders

### M1 — dead construction tables and duplicate-triggered growth

Source confirms growth before key-existence check and appended retired control,
bucket and dense tables. `clone_bound_from` copies their lengths, so this can
also amplify same-shape occurrence copies. Current warm read windows do not
allocate those tables per operation (`ALLOCATION-REVIEW.md`). Quantify actual
cold/rebuild ownership and bytes with the census and a discriminating pool
fixture before selecting a representation change. Avoid claiming lower RSS
from shorter Vec lengths or treating spare capacity as live map contents.

Tail-only construction may permit reclaiming construction space without a new
indirection, but extra copying can trade memory for CPU. Existing published maps
and their cursors must survive a later force, failed growth and partial rehash.
The current tests' direct grow_map calls can target a published map; don't assume
all test/diagnostic uses are at the pool tail without an explicit contract.

First isolate duplicate-triggered growth from any arena-layout redesign. A
store-free 50-row, 25-distinct-key root starts at eight groups. Current code
grows on the final duplicate despite unchanged distinct count. Compare against
an otherwise identical 26th-distinct-key fixture, and separately report map
capacity, live pool lengths and retained capacities across widths 1/2/3/4/8.
Ingest should probe first and grow/re-probe only for a missing new key that
crosses the load threshold. Check child multiplicity, old readable maps,
clone_bound_from, cancellation and reset reuse. This is a hypothesis for the
next isolated experiment, not a code change bundled into P1.

Further source audit found a required downstream update:
`displaced.rs::forced_spoke_map_bytes` encodes the duplicate-triggered threshold.
Add boundary discriminators when changing it; the stock 453,241-key fixture
selects the same final table under both policies. `MAP-OWNERSHIP-LEAD.md` now
separates that fix from a later one-time force-completion tail compaction,
including why generic grow_map cannot assume it owns the arena tails.

### M2 — dense-group key allocation at finalization

The current stats window has a small unexplained-by-parameters eight-byte request
in addition to cancellation. `for_each_ram_group` owns a new key vector on each
dense iteration. Verify site attribution and other dense-group consumers, then
reuse an existing appropriate owner if it removes allocation without duplicating
state or changing output/error semantics. A useful general cleanup, lower CPU
priority than P1/P2/P3 unless the full site census changes that ranking.

### M3 — storage range-bound ownership (new census lead)

The completed census attributes two 18-byte requests per inserted row in both
10k insert fixtures to DataTree::prefix_iter -> heed::Database::range. Current
DataTree already uses a stack array for its own prefix; the dependency converts
borrowed range bounds into owned Vecs. Heed 0.22.1's cursor module and cursor
constructor are private; its public RoIter/RoRange do not expose seeking.
Removing this allocation generically would need a dependency API improvement,
not another local buffer abstraction or a repeated seek per enumerated row.
Keep the exact-prefix point paths in view for a later source audit, but do not
trade safe dependency encapsulation or linear scans for a tiny request count.
No dependency change or unsafe cursor escape has been made.

## Obsolete evidence, not future work

Old native deadline polling and older ResidentRows enum-per-next/result-carrier
costs have already been changed. Do not reintroduce them for a comparison or
recount their former shares as current opportunities. The newer range capture
was audited against current finalization/projection source too; it does not
justify blindly repeating the previous ownership rewrite.

## Acceptance and next full-trace gate

The 32+34-family allocation run is complete and reviewed in full. The original
site census failed before measurement because its fixture omitted Account.id's
required functionality declaration; preserve `census/` as failed evidence.
Retry the census serially with the test-only fixture repair explicitly recorded,
while keeping engine source frozen at the published baseline. The retry is now
complete with zero dropped events; engine source identity was verified at both
ends. P1 was selected using
both CPU and allocation evidence. Compare baseline/baseline and alternating
candidate controls; retain correctness failures and noisy/losing measurements.
Keep broad affected and negative-control families visible. New full tracing
remains disabled until the supported open leads have been implemented and
verified, disproved experimentally, or shown obsolete with concrete evidence.
