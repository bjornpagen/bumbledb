# Q1: current output and survivor ownership, including preparation

This document records the BASELINE discovery before implementation, not the
current candidate status. See Q1-DEMAND-GROWTH-REVIEW.md for the subsequent
gated implementation and matched allocation results. This was saved-evidence
mining, not a new CPU trace. The ordinary source for these baseline audits was
exactly published
b02a641e; five cfg(test) mounts expose read-only owner metadata. The query,
schema and input database reproduce the saved S/seed-1 range and triangle.
The q1-owners driver verifies each mounted file becomes byte-identical to HEAD
when its single test hook is removed, and rejects all other source changes.

## Completed diagnostics

- q1-owners-1 (session 85028): build completed 10:38:23 UTC, all 32 exact SQL
  execution windows completed 10:38:26, exit 0. Every window validates all
  returned rows against handwritten read-only SQLite queries. The four range
  draws each contain 2,000 rows; triangle draws contain 5/5/5/0 rows.
- q1-owners-review-1 checks all 32 counter/answer/sink records, all 100 active
  and parked views, and all 64 spare buffers. All 16 overlapping triangle
  allocation tuples exactly reproduce the frozen M1 published baseline.
  The main agent read the entire ownership.log, not just selected totals.
- q1-prepare-1 (session 43856): build completed 10:40:06, two preparation-only
  observations completed 10:40:06, exit 0. It reuses the byte-identical saved
  query/schema/owner helpers, but measures preparation BEFORE any execution.
  This is a new observation window, not a repeat of the completed timing panel.

All baseline input database/oracle hashes were checked before and after.
Counter windows end before SQL-result conversion, printing, owner inspection,
or separate first-predicate replay. Those replays are diagnostics, not timing.
The imported standalone schema emits unused-item warnings; this diagnostic
build is not advertised as a strict-lint or whole-library correctness gate.

## Largest discovery: speculative result-table preparation

The triangle sink owns **83,886,087 bytes** before any execution: 8,388,615
control bytes, 8,388,608 u64 keys (67,108,864 bytes), and 8,388,608 stamps.
Its Vec<()> values have usize::MAX capacity but ZERO payload bytes; they are
not an exabyte owner. With five results the dense list adds 32 capacity bytes,
giving **83,886,119 bytes**, about 80 MiB, for the result hash table alone.
Empty fresh queries still retain the 83,886,087-byte backing.

Actual preparation counters:

| Query | Requests / frees | Requested / freed bytes | Plan estimates | Result table retained before execution |
| --- | --- | --- | --- | ---: |
| range | 173 / 103 | 613,519 / 597,279 | `[6250]` | 0 |
| triangle | 299 / 146 | 83,938,163 / 15,840 | `[100000,200000,2400000]` | 83,886,087 |

The source chain is exact:

1. `binary2fj` copies join-order step estimates into Free Join nodes.
2. `build.rs::free_join_hint` takes the LAST estimate, clamped to 2^21;
   `output_hint` forwards it without executing/binding query parameters or
   measuring the head's projected distinct rows.
3. `make_plain_sink` → ProjectionSink → SpillSet → WordMap::with_capacity_hint
   eagerly allocates `(hint.clamp(2,2^21) * 3).next_power_of_two()` slots.
4. The actual triangle hint reaches 2^21, hence 2^23 slots and the observed
   three buffers. Five projected account rows do not justify that backing.

Range is also relevant: it is proven distinct, so `elide_output_hashing`
immediately drops the just-allocated hash backing. Source arithmetic for its
6,250 hint gives 32,768 slots and **589,831 bytes** allocated then dropped
before preparation returns. That attribution is source arithmetic consistent
with counters, not a new stack-attributed allocator event.

Previous execution-only windows deliberately began after prepare. They still
correctly report zero warm requests, but did not account for this owner.
This result-map owner is separate from COLT-pool retention. Do not relabel
the earlier COLT savings as whole-query or whole-process memory savings.
These bytes are requested layouts/capacities, NOT physical pages/RSS: ordinary
zeroed allocation and Linux/macOS overcommit may leave much of this backing
nonresident. No claim that 80 MiB was touched/resident or saved has been made.

## Smaller leads, now measured rather than guessed

Cell is **24 bytes**, alignment 8, on this actual host/compiler. The range
answers retain 4,000 cells = 96,000 bytes; the distinct sink separately owns
4,000 u64 words in capacity 4,096 = 32,768 retained bytes. No hash backing,
scan-row staging, text or blob capacity remains in that query. The current
source still gathers row-strided words, then fills row-strided decoded cells,
but direct unique-row gather already avoids the old intermediate row copy.

The saved historical range capture at df737a6f attributes exclusive 31.64%
to Cell writes, 16.43% to u64 writes, 14.33% to gather_scan_rows and 11.47%
to slice splitting. Current finalization has new cooperative chunks and
removed old charging; these historical percentages are not present latency
or predicted savings. Output ownership survives prepared/DB lifetimes and
must not be replaced with borrowed image words. Any compact representation
must price complete consumer reads, not move decoding outside the timer.

The survivor vector retains 100,000 u32 slots = 400,000 bytes per filtered
binding in BOTH queries. Four cached bindings retain 1,600,000 bytes; all
spare capacities are zero. This owner is already included in COLT retention,
so do not add it again to those totals.

| Draw | Range first-predicate survivors → final | Triangle first-predicate survivors → final |
| --- | --- | --- |
| 0 | 93,750 → 2,000 | 49,508 → 529 |
| 1 | 81,250 → 2,000 | 32,937 → 495 |
| 2 | 68,750 → 2,000 | 16,517 → 500 |
| 3 | 56,250 → 2,000 | 0 → 0 |

The first Ge scan reserves for the whole input, then scalar Lt refinement
shrinks only length. Merely changing reserve to incremental growth would still
retain the large intermediate prefix; geometric overshoot might even worsen
it. Fused or bounded conjunction evaluation needs separate reasoning and
dense all-match controls. It is not yet implemented.

## Ranking and continuation

Q1 speculative output-hint allocation is the next general memory experiment:
it is substantially larger than the survivor/output-layout owners and needs
no new allocator, limit or public API. `Q1-HINTED-ALLOCATION-LEAD.md` records
the intended narrow change and correctness/performance obligations. P1 remains
the historical CPU leader; existing deferred CPU candidates remain open.
Nothing here accepts G2 timing or declares the saved traces exhausted.

## Closeout

q1-closeout-1 completes 10:43:32 UTC (session 63304), exit 0: all temporary
test mounts removed, q1-source clean at published HEAD, format/diff checks
pass, all frozen observation dependencies verified, and all six prior
candidate identities exactly match g2-closeout-3. All sessions are terminal.
No production implementation, commit, push, release or full trace this turn.
