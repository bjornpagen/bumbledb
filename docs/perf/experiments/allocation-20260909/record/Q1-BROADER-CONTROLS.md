# Q1: broader allocation controls and the first-use tradeoff

Status: **allocation comparison verified; ordinary timing now complete**.
Q1-ORDINARY-TIMING-REVIEW.md is the latest authority: all 672 distributions
checked, substantial preparation savings and adverse DNF/union/warm controls
preserved. Q1 remains isolated, not performance-accepted. The timing plan below
is the pre-run protocol, not an instruction to repeat the completed panel.
This continues Q1-DEMAND-GROWTH-REVIEW.md. No ordinary engine edit in this
turn. The isolated candidate remains the exact gate-2 source; no combination
with earlier candidates, no full trace/benchmark, commit, push or release.

## Protocol and verified coverage

The frozen S/seed-1 database and read-only SQLite oracle are reused. The
shared q1_control_cases.rs supplies explicit query ASTs and independently
written numeric SQL, not SQL derived from the engine's rendering. Forty-one
case/draw combinations cover eleven shapes:

- Computed identity projection, actual overlapping written union, a named
  interior and high-cardinality hashed aggregate groups: thresholds
  0/5/513/8192/100000 on the existing 100,000 Posting rows.
- Distinct (account, amount) aggregation, written union aggregation and
  DNF aggregation: account thresholds 0/5/500. The latter two use `< n` and
  `<= n`; n=0 is therefore NOT empty. Hot account 0 alone has 50,365 distinct
  input pairs, even though the aggregate returns one group.
- Exact dense currency groups with account thresholds 0/5/500, point hits
  and a miss, and ancestor recursion with/without an interior on OrgParent.

Each case measures prepare, cold, warm, changed parameter zero, return to
the original parameter, release, refill and warm-refill: **328 counter/owner
windows per variant**. All six executions per case check every numeric
result against sorted independent SQL answers. Release also checks that
previous owned answers survive. The answer owner is intentionally retained
through prepared.release_memory(); refill is not mislabeled fresh cold.

All SQL checks, result conversion, labels and owner printing happen outside
the allocation windows. Diagnostic builds/times are not speed evidence.
No new input corpus or CPU profile is collected.

## Invalid attempts are preserved

q1-controls-baseline-1 ran the original identical-arm union, which normalized
into a single proven-distinct rule. Its data is preserved, but that fixture
did not exercise the required hashed-union path. q1-controls-candidate-1 is
**invalid**: the shared Cargo target/build cache returned `fresh=true` and
the byte-identical baseline executable despite candidate source. The runtime
still reported speculative backing. That attempt is explicitly marked
INVALID-BASELINE-EXECUTABLE-REUSED, not a candidate measurement.

The shared binary SHA was
f3f37350d2cce54cdf1326ec00dc7a6ee82c79d2aebe283d28d94a7e3c9f7e1a.
This was caught by artifact identity and actual map ownership before any
comparison was accepted. Do not rely on source fingerprints or the artifact's
reported package path alone to prove compiled-code identity.

Attempt 2 uses distinct overlapping union arms (id threshold versus entry
threshold) and asserts that the 100,000-row union owns a hashed result map.
Both CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR are phase-isolated. Both engine
artifacts must be freshly compiled, originate from the expected source path,
and reside within their own phase directory. Candidate execution additionally
asserts every unexecuted hash map has zero backing.

- Baseline 2 (28582): fresh build and all controls complete 11:05:52 UTC.
  Binary SHA: 454a2912143f5cbc909afc6976123ec3bd2e9772286e017bdb3a1c9758e89433.
- Candidate 2 (82651): fresh build and all controls complete 11:07:03 UTC.
  Binary SHA: 358438005e81d506a1d48f76c0039ae4545de6aaac7fab9b65ed98d91d2c36e8.
- q1-controls-review-1 verifies both helper/input identities, all 328 paired
  counters and all 488 map records per variant. Every answer, projection,
  dense-set, aggregate regime, dense table, fold and other recorded aggregate
  owner matches exactly outside the changed hash backing.

The review reconciles cumulative requested-minus-freed differences at every
window against the complete observed map-capacity differences plus a constant
small preparation metadata difference. Nothing large disappears from the
accounting at release or gets moved into an uncounted first execution.

## The adverse result matters

Smaller outputs retain much less map backing. Large real outputs still grow
to the same final power-of-two capacity, but now traverse more intermediate
sizes. The extra allocations/rehashes are a real cost to price, not noise.

Selected prepare PLUS cold deltas (candidate minus baseline):

| Case | Actual output rows | Requests delta | Requested-byte delta | Map capacity baseline → candidate |
| --- | ---: | ---: | ---: | ---: |
| computed, threshold 5 | 5 | +2 | -2,358,873 | 2,359,335 → 327 |
| computed, threshold 513 | 513 | +23 | -2,285,672 | 2,363,399 → 40,967 |
| computed, threshold 100000 | 100,000 | +41 | **+2,359,234** | 9,961,479 → 9,961,479 |
| written union, threshold 8192 | 23,306 | +40 | **+2,359,218** | 2,490,375 → 2,490,375 |
| written union, threshold 100000 | 100,000 | +40 | **+2,359,218** | 9,961,479 → 9,961,479 |
| named interior, threshold 100000 | 100,000 | +49 | **+2,360,247** | 19,922,958 → 19,922,958 |
| hashed groups, threshold 8192 | 8,192 | +43 | **+294,813** | 622,599 → 622,599 |
| hashed groups, threshold 100000 | 100,000 | +43 | **+294,813** | 9,961,479 → 9,961,479 |
| distinct pairs, threshold 500 | 500 | +28 | -2,580,607 | 2,656,270 → 38,919 |
| DNF or written union groups, threshold 0 | 1 | +40 | **+2,064,418** | 5,275,678 → 4,980,910 |
| DNF or written union groups, threshold 500 | 500 | +72 | **+2,137,914** | 10,258,446 → 10,000,398 |

All 41 separate prepare/cold deltas are retained in the review STATE, including
15 cases with MORE joint requested bytes. No favorable-only selection for
acceptance. Interior+reach adds 3,125–4,287 joint requested bytes depending on
the source node; plain reach saves 130–798. These small figures are reported,
not promoted ahead of the large allocation owners.

The pairs case has an important observed distinction: its binding seen-map
has zero live rows in both variants after execution. The existing physical
distinctness path does not use that backing, but baseline retains it anyway.
Q1 removes that unused reservation. By contrast, DNF/written-union aggregate
seen-maps contain 50,365–99,872 exact input pairs. A small number of result
groups is NOT evidence that the input-dedup table should also be small.

All warm, changed-parameter, return, refill and warm-refill allocation tuples
are **equal between variants**, not universally zero. Dense controls retain
an existing one-request/eight-byte warm allocation; interior+reach retains
six requests/1,534 bytes per warm call. Those pre-existing tiny leads are not
Q1 regressions and are lower priority than pricing the growth tradeoff.
Exact dense radix [3], the 12-byte table and its ordinals are unchanged.
Point preparation avoids three transient requests/151 bytes; execution and
retained owners remain unchanged. All released maps own zero backing.

These are requested Rust layouts and retained capacities, **not RSS, peak
resident memory, speed or qualification on the 512 MB Pi Zero 2**. More total
requested bytes does not establish a higher peak; smaller retained capacity
does not establish fewer touched OS pages.

## Ordinary timing protocol (subsequently completed)

The broad allocation tradeoffs are now known. Do not repeat these controls
for nicer numbers or restore a speculative join hint to hide request counts.
Keep Q1 isolated until its time cost is measured with ordinary builds.

Use the exact saved triangle/point inputs plus representative small and large
cases above. Include the single-group/high-input-cardinality DNF case, not
just high output cardinality. Measure paired preparation, first execution,
their joint elapsed interval, second execution, same-target warm and
alternating-parameter warm operation. The old e1_timing driver excludes
preparation and is insufficient without modification.

Warm sampling must actually cross the old u8 generation rollover: tiny-result
baseline tables can physically clear their large control arrays after 255
resets. Either retain enough individual calls or timed batches to expose that
tail. Do not silently report only a short pre-rollover window. Retain all
raw distributions, tails and non-retrying clock flags, with verified macOS
QoS steering (not a claim of hard P-core affinity). No full suite or trace.

Build matched variants from the same source path where practical and isolate
BOTH Cargo output/cache directories per frozen build. Assert a fresh artifact
and distinct binary identity, not just a correct source tree. Reuse the
frozen shared query/SQL file without editing it; put any added timing cases
in a separate helper. All prior candidate/evidence identities must survive.

## Closeout

q1-controls-closeout-1 (27351), completed September 9 at 11:11:36 UTC,
verifies candidate source exactly matches gate 2, the separate baseline
worktree is clean at published HEAD, all six prior candidates are unchanged,
all observation mounts are removed, both source trees pass format/diff
checks, and matched helpers/binaries retain their verified identities.
All sessions are terminal. No commit, push, release or full trace occurred.
