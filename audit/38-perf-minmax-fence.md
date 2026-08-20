# 38 — Deferred finding 009 step 2: the per-forced-map min/max fence

- **Status:** **fixed this pass** — closed: premise still true under
  `GroupState::Folds` (`Acc::Min`/`Acc::Max`), but the kernelized
  column fold is the coordinate change and Force does not rank on the
  fold lanes. o5 692 µs / 175 ms (0.00×); o3 344 µs / 116 ms.
- **Severity:** performance debt, small.

## The recorded fact

Finding 009's step 2 — a per-forced-map min/max fence — was deferred at the
campaign close (the R5 Arg/CountDistinct family it neighbored is killed;
the fence itself survived the kill as still-applicable to the surviving
`Min`/`Max` folds).

## Protocol

Confirm the fence's premise still holds on the current tree (the fold sink
became `GroupState` this pass); if it holds, land the fence; if the
`GroupState` restructure absorbed it, close by inspection note plus the
fold-lane numbers.

## Premise under `GroupState`

`GroupState::Folds { accs, n_aggs }` still holds `Acc::Min`/`Acc::Max`
(`exec/sink.rs`). A forced-map whose column extrema cannot beat the
running acc could still be skipped. That skip is **not** implemented —
GroupState did not grow a fence.

What the restructure *did* absorb is the walk: `fold_batch` /
`aggregate/sink.rs` dispatch Min/Max through
`kernel::fold_min_max_u64` over columns (or the idx twin), not a
per-row branch. o5 (`Min+Max` per store, 200 groups) pays
`jp_descend_n1` 664 µs exclusive and **no `jp_force`** — the fence's
forced-map surface is not on the fold lanes.

Fold-lane numbers (2026-08-20, same olap run as 36, 8 samples):

| query | ours p50 | sqlite p50 | ratio |
| --- | ---: | ---: | ---: |
| `o5_store_extremes` (Min+Max) | 692 µs | 174_954 µs | 0.00× |
| `o3_promo_split` | 344 µs | 115_831 µs | 0.00× |

No fence landed. Fold lanes are not worse — they win by two orders.

## Acceptance

- Fence landed or closed-with-reason; fold lanes not worse; TODO.md row
  closed.
