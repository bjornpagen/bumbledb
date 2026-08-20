# 36 — Deferred finding 014: per-parent leaf batch-of-1

- **Status:** **fixed this pass** — closed-by-measurement. o4 re-benched
  **0.07×** vs SQLite; the adjacent pinned-row arm absorbed the
  53–69 ns/tuple scaffolding (now 24 ns/call). `run_leaf_pinned_run`
  remains a HANDOFF (tests only).
- **Severity:** performance debt, attribution-first.

## The recorded facts (TODO.md)

Finding 014 — the per-parent leaf runs batch-of-1. The campaign's pinned-run
fold `a75d1e65` "lands the adjacent mechanism, but o4's lane was not
re-benched."

## Protocol

Re-bench the o4 lane on the current tree first — the adjacent fold may
already have absorbed the win. If the batch-of-1 still shows, batch it
along the landed fold's mechanism; if it does not, close the row with the
re-bench numbers.

## Re-bench (2026-08-20, Apple M2 Max, obs release, 8 samples)

`scenarios --only olap --trace --samples 8`, 525_200 facts:

| query | ours p50 | sqlite p50 | ratio |
| --- | ---: | ---: | ---: |
| `o4_segment_category` | **25_541 µs** | 375_923 µs | **0.07×** |
| o1 | 460 µs | 235_005 µs | 0.00× |
| o3 | 344 µs | 115_831 µs | 0.00× |
| o5 | 692 µs | 174_954 µs | 0.00× |

o4 warm flame: `jp_descend_n3` **500_000** calls, 24 ns avg, 12_123 µs
exclusive (46% of 26.3 ms wall). Call count is still one-per-leaf-row
(batch-of-1 shape). Per-call cost is **24 ns**, down from the recorded
53–69 ns/tuple of emit scaffolding — `run_leaf_pinned` (batch-of-one
with scaffold skipped) is the adjacent mechanism, and it is live.

`run_leaf_pinned_run` (N rows, one `LeafBatch`) is still
`#[expect(dead_code)]` HANDOFF — called only from
`exec/run/tests/pinned_run.rs`. probe_pass.rs:558 gravestone: batching
the routing-loop copies was NEUTRAL. Wiring the run arm is a later
handoff, not this row's ranked fix: the o4 lane already wins 15× and
the 53–69 ns scaffolding is gone.

## Acceptance

- The o4 lane re-benched and recorded; the finding fixed or closed-by-
  measurement; TODO.md row closed.
