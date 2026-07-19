# The incremental-images packet — the rebuild punt retired, the 4 GiB axiom retracted

One direction, honestly forked: on an insert-only commit the next reader's image
is built by **copying columns and decoding only the tail** instead of rescanning
the world; on a delete-bearing commit today's evict-and-rebuild stays, priced
and measurable instead of invisible; and the map-size ceiling rises from 4 GiB
to 32 GiB as a documented retraction, not a silent edit.

This directory is an execution-style PRD packet (ordered, strict passing
criteria). Per house convention it is DELETED once shipped. It distills the
design corpus at `/tmp/bumbledb-feature-research/` (incremental-images-facts,
-prd1, -deletes — session artifacts, not repo citations) plus the owner rulings;
this packet is the plan of record — the /tmp docs do not survive it.

Branch: `worktree-incremental-images` · PR #10 — **stays OPEN; nobody merges
anything**. A serial committer owns git; PRD workers never commit/push.

## The rulings (no PRD re-litigates these)

1. **Copy-on-append is APPROVED** as designed (I1): fresh frame at the new row
   count, per-column `copy_from_slice` prefix, tail-only decode via a suffix
   range scan from the base image's recorded row-id high-water; zero-copy
   carry-forward (same `Arc`, re-keyed) for relations the commit never touched.
   The soundness spine is the **row-id high-water's** committed monotonicity
   (`storage/commit/applier.rs:276-294`, flushed `commit/write.rs:337-340`,
   sweeper-enforced `verify_store.rs:185`) — NOT the `Q` fresh never-reissue
   law (`storage/delta/alloc.rs`), which governs host-visible fresh *field
   values*; cite the right law or not at all.
2. **The delete fork is GATED.** The tombstone/validity-mask route is NOT built
   in this wave. The honest price is recorded: under the only structurally sane
   variant (route (b), view-build-time masking) the mask is one AND at view
   build **plus an engine-wide conversion of the dense case into the gathered
   case** for any tombstoned relation (`View::All` ceases to exist; the
   `fold_*_dense` family, `gather_identity`'s contiguous copy, and
   count-as-arithmetic stop firing — the dense→gathered gap is already measured
   at 8.8 vs 4.0–4.6 rows/ns, `exec/kernel.rs:30-33`), plus the ray-poison
   fusion in `filter_duration_range_u64` and every measure surface. The re-earn
   bill: ~26 published wall-clock claims + 10 report rows + ~8 kernel pins;
   the hard gate-enforced part is the 22 ALL-WIN wins + 2 geomeans.
   **Recorded reopen triggers:** (a) a real delete-heavy, latency-sensitive
   workload arrives (large relation + high delete rate + read latency that a
   single-digit-ms-per-100 MB rebuild actually ceilings — today none exists:
   the bench never times a delete-induced rebuild, and primer's write paths are
   LLM-bound so rebuilds vanish in its wall clock); (b) I2's lane shows the
   delete-rebuild cost as a real ceiling for a workload someone runs; the fork
   reopens with I3's twin number already in hand and the writer-side
   compaction-threshold policy (`dead_fraction`/`S_min`/`G_max`/measure-bearing
   escape hatch) as the frame.
3. **The decider twin runs anyway (I3).** The filter-mask twin is an afternoon
   and one of its two outcomes permanently retires the fork's kernel-tax half.
   It ships `#[ignore]`d, masked kernel test-local — nothing enters the product
   kernel surface while the fork is gated.
   **RECORDED (Wave M, 2026-07-19): the CONFIRM branch fired — the fork's
   kernel-tax half is dead.** B/A ≥ 1.10 at BOTH tiers and both selectivities
   (L2 1%: 1.2443; L2 50%: 1.2248; DRAM 1%: 1.2355; DRAM 50%: 1.1986), and the
   codegen-isolated B/A′ also clears 1.10 everywhere (1.13–1.18; A′/A read
   1.04–1.06, above the noise band, so B/A′ is the load-bearing ratio). C ≈ B:
   the tax is the mask plumbing, not the holes. Conditions + full table:
   prd-I3 § the recorded verdict. **Consequence for ruling 2:** the mask is
   NOT free even at its cheapest surface — a reopened delete fork starts from
   compact-on-delete (the writer-side compaction-threshold policy), and the
   mask route needs a workload so delete-heavy it eats a ≥ 1.13× filter-surface
   tax PLUS the dense→gathered conversion before it re-enters design.
4. **THE 32 GiB CEILING RULING (owner):** *4 GiB is too low as a hard limit;
   32 GiB is the new hard limit, with the engineering problems that follow.*
   This FLIPS `storage/env.rs:168`'s `MAP_SIZE = 4 << 30` and every sentence
   downstream of it (`50-storage.md:13-14,442`, `README.md:430`, the container
   /ramdisk/test sizing). A flip is a documented retraction with the new truth
   written — never a silent edit. G1 owned it; its work list was a stub pending
   the 32G scout's report. **SUPERSEDED (cleanup-0.5.0 ruling 1):** G1's
   per-kind split (durable 32 GiB / ephemeral 4 GiB, `StoreKind::map_size()`)
   is retired in favor of ONE lazy 32 GiB `MAP_SIZE` for both kinds — no
   `WRITEMAP`, no eager capacity contract, no preallocation
   (`docs/prds/cleanup-0.5.0/prd-U1-ephemeral-lazy.md`). The prd-G1 file is
   deleted with the split; the ceiling ruling itself (32 GiB, never a knob)
   stands.
5. **The epistemic retractions** (each lands WITH the code that replaces it):
   - *"writes are bursty and rare"* (`image/cache.rs:37-39`,
     `docs/architecture/50-storage.md:524-528`) — RETRACTED. A workload
     assumption, never a measurement; steady-write hosts are real and are
     served by the copy-on-append path, not by an assumption about frequency.
   - *"ceiling-hitting workloads are append-shaped"* — bench-true by
     construction (zero deletes in any timed family), workload-FALSE: the
     cookbook's taught write idioms (recipes 9, 13, 20, 21, 27) and primer's
     hottest recurring commits (attemptText swap, derive diffs, settles) are
     delete-bearing. What saves the day is the denominator (primer is
     LLM-bound), not the shape claim. I2 exists so this stops being invisible.
   - *"row ids exist only in LMDB keys"* — imprecise: they are also M/U values
     and the R key tail (`storage/commit.rs:165-167`, `keys.rs:326-340`);
     never in images or exec. Prose swept where touched.
   - *"single-digit milliseconds per 100 MB"* (`50-storage.md:496-497`) —
     bandwidth arithmetic, not a measurement of the decode-bound build; I1's
     measurement re-trues or retracts it with the baseline it establishes.

## The dependency graph

```
I1 copy-on-append (engine + docs + fuzz) ──→ I1's measurement (Wave M: idle machine, owner go)
I2 delete-bearing cold-read lane (bench)  ──→ feeds Wave M (the delete cost becomes measurable;
                                               also the negative witness: I1 must NOT move it)
I3 the decider twin (#[ignore]d kernel falsifier) — independent; verdict filed either way
G1 the 32 GiB ceiling (constant + docs + sizing) — independent; scout-refined, LANDED,
   then SUPERSEDED by cleanup-0.5.0 ruling 1 (one lazy 32 GiB map, both kinds)
```

| PRD | Title | Depends on |
| --- | --- | --- |
| I1 | Copy-on-append image maintenance | — |
| I2 | The delete-bearing cold-read bench lane | — (lands before Wave M) |
| I3 | The filter-mask decider twin | — |
| G1 | The 32 GiB ceiling | the 32G scout's report (refined + landed; the per-kind split since superseded — cleanup-0.5.0 ruling 1, prd-G1 deleted) |

## The gates (every PRD proves its own)

- `scripts/check.sh` exit 0 (fmt, clippy -D, workspace tests, alloc gate,
  crashpoint + kill sweeps, feature matrices) and `scripts/lean.sh` exit 0
  (images are below the model — no lean change expected anywhere in this packet).
- Engine law: no new unsafe outside the sanctioned modules; never weaken a
  test, a lean theorem, a pinned margin, or a probe to pass; errors carry
  facts, never row ids; renames sweep citations (`scripts/spec-census.sh`).
- The landing bar governs every perf claim: a win lands WITH `scripts/measure.sh`
  numbers (the machine-wide mutex; interleaved A/B; tier stated); unmeasured
  claims are marked pending-measurement, never asserted. Bench families are
  Report-class unless already gated — nothing in this packet adds a gate.
- Measurement is its own wave: idle machine only, owner go, and the Machine
  discipline holds (a foreign bench repin may own the box — check before any
  build).
