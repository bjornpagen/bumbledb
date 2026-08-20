# 32 — `cold_containment_walk_delete` 3.1–3.4×: trace the untraced twin, then fix

- **Status:** **fixed this pass** — traced twin lit (`trace_cold_containment_walk_delete`,
  `traced_cold_solo`); test `cold_containment_walk_delete_traced_twin_lands`.
  Re-ruled with the current-tree flame: the campaign suspects do not rank.
- **Severity:** performance debt, attribution-first.

## The recorded facts (TODO.md)

- `cold_containment_walk_delete` 3.1–3.4× in all six reps.
- Traced suspects from b100: `apply_deletes` self-time 2.1× under the
  cursor-fold applier; `judgment_source` +70% under the T8 walk at small
  batch (the sweep only priced 1k–4k parents).
- **The delete lane has no traced twin** — the reps' write set is untraced
  by protocol.

## Protocol (house rule: data first, no intuition fixes)

1. Light the delete lane's traced twin (same protocol as the existing
   flamediffs; the close's diffs live in git history:
   `writes.durable.delete_b100.diff.svg`).
2. Rank by trace-reader; fix the top attribution only; re-run the six reps.
3. Note: the T8 walker is now the `SortedGets` trait impl and the applier
   sits under `MutationCore` — re-trace on the CURRENT tree before
   believing the campaign-close suspects.

## The fix

The twin is one captured cold sample after the timed window: seed a
swap, then `traced_cold_sample` (harness `touch` + `sample`) writes
`<out>/trace/cold_containment_walk_delete.{json,folded}`. Wired from
`driver/write_families` when `--trace` selects the family. Own seed
page `cfg.seed ^ 0x0115_0005` so the twin never collides with the
timed stream.

## Current-tree traces (2026-08-20, Apple M2 Max, obs release, NosyncLane)

**`cold_containment_walk_delete`** — scale S, seed 1, COLD protocol
(2+16), `--trace` `--i-am-lying` `--nosync`:

| family | ours p50 | sqlite p50 |
| --- | ---: | ---: |
| `cold_containment_walk_delete` | 10678.9 µs | 52.4 µs |
| `cold_containment_walk` (insert-touch sibling, same corpus) | 903.0 µs | 84.8 µs |

Delete / insert-touch = **11.8×**. Flame top-5 self (one cold sample,
wall 11852.8 µs):

| span | self_us |
| --- | ---: |
| `image_distincts` | 7577.0 |
| `decode_batch` | 2598.5 |
| `selections` | 927.3 |
| `image_build` | 214.3 |
| `join` | 67.0 |
| `apply_deletes` | 19.3 |
| `judgment_source` | (absent) |

**`writes --trace` `delete_b100`** — scale S, seed 1, 2 samples,
NosyncLane (the historical flamediff's cell, re-traced):

| cell | ours p50 | sqlite p50 | ratio |
| --- | ---: | ---: | ---: |
| `delete_b100` | 1539.1 µs | 1736.9 µs | **0.89×** |
| `commit_b100` | 1561.7 µs | 2108.5 µs | 0.74× |

`delete_b100` flame top self: `apply_deletes` 750.7 µs, `lmdb_commit`
731.9 µs, `write_txn` 176.8 µs. `judgment_source` 0.3 µs — does not
rank. The T8/`SortedGets` walk is no longer the cell's cost.

## Re-rule

The campaign suspects predate `MutationCore` / `SortedGets` / I1's
append-only incremental images. On this tree:

1. The **cold delete family's** top attribution is `image_distincts` on
   a full rebuild of the walked relation — the I1 discriminator's
   expected shape (`writebench.rs`: insert-touch collapses, delete-touch
   must not). That is essential complexity (REQUIRED-READING Insight 16),
   not a ranked applier defect.
2. `apply_deletes` is 19 µs on the cold family's one-row swap; it does
   not rank.
3. `judgment_source` does not appear on the cold family's flame and is
   0.3 µs on `delete_b100`.
4. `delete_b100` itself is **0.89×** vs SQLite — the write-ladder cell
   is not a 3× regression.

No engine edit. The 3.1–3.4× row closes as the I1 delete-rebuild
discriminator, now traced.

## Acceptance

- The lane's traced twin exists and is repeatable.
- The 3.1–3.4× either closes to a stated target or is re-ruled with the
  trace attached; TODO.md row closed either way.
