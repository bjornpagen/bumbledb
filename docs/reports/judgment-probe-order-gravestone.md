# T8 gravestone: judgment probes in key-sorted order — refuted at every real size

The commit path's source-side judgment (`storage/commit/judgment.rs:
check_source`) runs one LMDB B-tree descent per containment edge, serially
dependent, in the delta's fact-hash order. The LMDB API is serial — no
memory-level parallelism can be manufactured through it — so the one
honest lever left was probe ORDER: sort the probe worklist by the probed
`U` key (`target relation ‖ key statement ‖ key bytes`, exactly the
stored byte order) so successive descents share upper pages and walk the
leaf level monotonically instead of random-walking the tree.

**Verdict: refuted — do not land.** The sort wins only in a narrow
constructed band (probes covering ≥ ~1/16 of a beyond-L2 target tree)
and is a measured 2–9% judgment-span LOSS at every realistic commit
size, exactly the instruction-count law of the cached tier
(`m2max.mem.l2-resident-retire-bound`, `m2max.method.layer-law`: below
the memory wall, added instructions are the only surviving cost — and
the sort is added instructions).

## The experiment

The twin and its pin harness are commit `b3ea9bfe` on
`perf/judgment-probe-order` (this gravestone's parent): the sorted
worklist in `check_source`, a `trace`-gated thread-local off switch
(`with_probe_sort_disabled`, the `ground-off` precedent), and the bench
example `judgment_probe_order` — interleaved A/B inside ONE process
(`m2max.method.interleaved-ab`), fresh random parent draws per
repetition (`m2max.predict.tage-memorizes-benchmarks`), ABBA pair order,
60 pairs per cell, the measured span the engine's own `judgment_source`
trace span divided by its traced probe count
(`m2max.method.attribution-count-error`). Ephemeral store on the
ramdisk scratch, so no fsync floor drowns the µs-scale phase. World:
`Child(parent) <= Parent(id)` — one scalar probe per inserted child.
Every measured commit's children were deleted untimed before the next
repetition (constant mass across arms).

Disassembly gate: `ipnsort` monomorphized on the worklist tuple
`(RelationId, StatementId, &FactOp, &EdgeOp)` with the `check_source`
closure is present and called in BOTH the measured (trace) and the
production (non-trace) release binaries — the codegen is the claimed
shape.

Semantics evidence (probe order is observably invisible, so the twin
was legal to try): `Violations::seal` sorts and dedups by citation
(`lean/Main.lean` `RVerdict`: ascending statement indices, order-free);
the differential and fuzz oracles compare `cited()` identities only
(witness fact bytes are engine-side detail the model never derives);
`cargo test --workspace` green with the sort in.

## Stamp

| | |
|---|---|
| Machine | Mac14,5 — Apple M2 Max (the pinned fact-ledger machine) |
| macOS | 15.7.7 (build 24G720) |
| Toolchain | rustc 1.99.0-nightly (be8e82435 2026-07-11), release profile |
| Date | 2026-07-16 |
| Protocol | `scripts/measure.sh` (the machine-wide mutex); interleaved same-session ratios only — absolute ns are co-tenant ambient and NOT comparable across sessions |

## The regime sweep

`ratio` = arrival-order ns/probe ÷ sorted-order ns/probe on the
`judgment_source` span (> 1 = sorted wins). Medians of 60 interleaved
pairs; p10/p50/p90 of the per-pair ratio distribution.

| parents | tier (U tree) | k=16 | k=64 | k=256 | k=1024 | k=4096 |
|---|---|---|---|---|---|---|
| 4,096 | cached (bench world) | 0.936 | 0.913 | 0.926 | 0.979 | 0.967 |
| 65,536 | ~L2/SLC | 0.988 | 0.943 | 0.943 | 1.020 | **1.214** |
| 1,048,576 | DRAM | 0.961 | 0.989 | 1.004 | 1.012 | **1.088** |
| 4,194,304 | DRAM, past L2-TLB reach | 0.971 | 0.979 | 0.994 | 1.025 | 1.034 |

Raw rows (sorted_med / arrival_med ns per probe): 4096-tier
250/232, 236/215, 220/204, 211/206, 217/209; 65536-tier 508/526,
400/378, 382/375, 409/429, 336/405; 1M-tier 1297/1198, 966/928,
810/810, 911/915, 808/887; 4M-tier 1164/1117, 1096/1082, 1054/1044,
1051/1072, 981/1030.

Family-shape guard (k=1 commits, whole-commit wall, ephemeral,
40 blocks × 256 commits): ratio p50 = **0.9984** — dead neutral, the
sort of a one-element worklist is invisible at family level.

## Where the win lives, and why it does not matter

The crossover is a DENSITY, not a commit size: the sorted order pays off
only when the commit's probe count is a large fraction of the target
tree's key count — ~1/16 density on a beyond-L2 tree gives +21%
(65,536 keys, k=4096); 1/256 density gives +8.8% (1M keys, k=4096);
1/1024 density gives +3.4% (4M keys). At every density real commits
have, the ratio is 0.91–1.03:

- **Interactive commits** (every write family, k=1–16): neutral to −6%
  span, 0.998 whole-commit — the worklist is one element.
- **Bulk chunks** (`bulk_load`, k=4096 by construction): against the
  bench worlds' own parent trees (4,096 keys, fully cached) the sort is
  a −3.3% span loss; the +21% cell needs a parent tree 16× the chunk
  AND beyond L2 — a store shape no current family has, and even there
  the judgment span is a minority of the chunk commit.
- **The cached tier's loss is structural**, not noise: p90 < 1 across
  k=16–256 at 4,096 parents. The tree is L1/L2-resident, descents cost
  no memory stalls, and the sort's compares, the worklist's cache
  pollution, and the extra per-edge key-statement lookups are pure added
  retire pressure (`m2max.mem.l2-resident-retire-bound`).
- At 4M keys the win at k=4096 SHRINKS to +3.4% (from +8.8% at 1M):
  probes-per-leaf falls below one, so sorting no longer dedups leaf
  misses, and each descent still pays its own walk
  (`m2max.cache.tlb-miss-cost` — the 48 MiB L2-TLB reach is exceeded,
  and sorted order cannot merge distinct leaves).

The predicted mechanism (upper-page sharing) is real but already free:
a hot tree's top levels are cache-resident under EITHER order; the only
sharable misses are leaf-level, and those only merge at pathological
probe density. This is `m2max.probe.window-vs-branchy`'s lesson wearing
a B-tree: a locality transform's sign is a property of the tier and the
density, and the regime where it wins is not the regime the engine
runs in.

## Standing falsifier

Re-run the experiment from commit `b3ea9bfe`:

```sh
git checkout b3ea9bfe
BUMBLEDB_SCRATCH_DIR=/Volumes/bumbledb-scratch \
  scripts/measure.sh cargo run --release -p bumbledb-bench \
  --features obs --example judgment_probe_order
```

The refutation reverses only if a REAL family's commits reach ≥ ~1/16
probe density on a beyond-L2 target tree — at which point the twin is
one `git revert` away, its harness already written.
