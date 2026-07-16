# Phase R: the ramdisk measurements

The numbers the ephemeral-store decision stands on (the shipping law:
measure before Phase 2). Produced by the ignored harness
`crates/bumbledb/tests/ramdisk_phase_r.rs`:

```sh
cargo test -p bumbledb --release --test ramdisk_phase_r -- --ignored --nocapture
```

## Stamp

| | |
|---|---|
| Machine | Mac14,5 — Apple M2 Max, 96 GiB (the pinned fact-ledger machine) |
| macOS | 15.7.7 (build 24G720) |
| Toolchain | rustc 1.99.0-nightly (be8e82435 2026-07-11), release profile |
| Date | 2026-07-15 (re-stamped the same day with the interleaved R4 harness — the fixit record) |
| Runs | three back-to-back runs of the interleaved harness; the table records the third, and the two prior runs agree within their spreads (the R4 trigger ratio measured 1.10x, 1.14x, 1.12x). Recorded quiet-machine band across ALL banked runs, widest observed: **1.09x–1.26x** (the original sequential harness stamped 1.12–1.16x; independent quiet re-runs measured 1.09x, 1.12x, 1.15x, and once 1.26x). One co-tenant-contaminated sequential run printed 2.27x and is *excluded* — that incident is why the harness now interleaves its cells and carries the quiet-machine guard (see R4) |
| Ram disks | `hdiutil attach -nomount ram://4194304` (2 GiB), formatted `diskutil erasevolume HFS+` (non-journaled by name) and `APFS`; every disk detached by the harness's drop guard — `hdiutil info` shows none after each run |

Harness placement (a recorded decision): an engine integration test, not
a bench-crate bin — R4 opens scratch heed environments with the flags
the engine forbids, and the bench crate's dependency quarantine
(`docs/architecture/00-product.md`) is rusqlite and nothing else. `heed`
as an engine dev-dependency adds no node to the dependency graph.

## R1 — the F_FULLFSYNC smoke test

LMDB's data sync on Darwin is `fcntl(F_FULLFSYNC)` with no fallback
(`mdb.c`: `MDB_FDATASYNC(fd)` is `fcntl(fd, F_FULLFSYNC)` under
`__APPLE__`), so a device that refuses fullfsync surfaces as a typed
`CommitSync` on the first committing write. The named integration risk
does not materialize:

| Volume | Committing write |
|---|---|
| HFS+ (non-journaled) ram disk | **PASS** — no `CommitSync` |
| APFS ram disk | **PASS** — no `CommitSync` |

Phase 2 consideration stays open on this axis (it closes on R4's
trigger below).

## R2 — commit latency

Median per-commit wall time (min–max spread), engine `Db::write`
through the typed insert path. Small = 16 facts/commit, n=64; bulk =
4096 facts/commit (the `Db::bulk_load` chunk shape), n=8. SSD = the
internal SSD (APFS, the system temp dir).

| Cell | small (16 facts) | bulk (4096 facts) |
|---|---|---|
| internal SSD | 4.916 ms (3.868–9.169) | 17.476 ms (16.057–25.984) |
| HFS+ ramdisk | 0.236 ms (0.209–0.294) | 10.426 ms (9.601–13.355) |
| APFS ramdisk | 0.656 ms (0.262–0.797) | 9.993 ms (9.712–10.814) |

The SSD small-commit median matches the ~4–5 ms fullfsync baseline
expectation. The ramdisk removes the fullfsync floor: ~21x on small
commits (HFS+; ~21–24x across banked runs), ~1.7x on bulk chunks
(where insert work, not sync, dominates). APFS pays ~0.4 ms more than
HFS+ per small commit (journal/container overhead) and slightly less
on bulk.

## R3 — the DVFS dividend

128 back-to-back single-fact commits, aggregate wall time:

| Cell | 128 commits | ratio |
|---|---|---|
| internal SSD | 634.074 ms (~5.0 ms/commit) | — |
| HFS+ ramdisk | 6.341 ms (~0.05 ms/commit) | **100.0x** |

The per-commit arithmetic from R2 predicts ~21x; the measured aggregate
ratio is ~94–100x (all banked runs) — the fsync DVFS floor is real: a
back-to-back fullfsync loop holds the cores at low frequency, so the
ramdisk buys more than the subtraction of the sync itself.

## R4 — LMDB flag deltas on the ramdisk (the Phase-2 trigger)

Scratch heed environments on the HFS+ ramdisk (never the engine's env —
the engine's flags are law), 24-byte keys / 16-byte values, same two
commit shapes. The three flag cells are **interleaved per repetition**
(the fact ledger's co-tenancy remedy: interleaved same-session A/B
stays valid under ambient load) — the original sequential per-config
blocks absorbed a co-tenant load spike asymmetrically, and one
contaminated rerun printed a spurious 2.27x trigger against a
quiet-machine ~1.1x. The harness also carries a **quiet-machine
guard**: it prints a warning whenever any bulk cell's max/min spread
exceeds 2x (quiet runs measure ~1.1–1.3x; the contaminated run
measured 2.9x), and a warned run's trigger ratios are branded not
decision-grade. Medians (min–max), n=64 small / n=8 bulk:

| Flags | small (16 puts) | bulk (4096 puts) |
|---|---|---|
| default | 0.049 ms (0.037–0.070) | 0.811 ms (0.695–0.865) |
| `MDB_NOSYNC` | 0.007 ms (0.006–0.015) | 0.743 ms (0.633–0.788) |
| `MDB_WRITEMAP\|MDB_NOSYNC` | 0.003 ms (0.003–0.014) | 0.726 ms (0.600–0.745) |

**The trigger does not fire.** The Phase-2 trigger is
`WRITEMAP|NOSYNC >= 2x` over plain-ramdisk-default on the bulk-load
shape; measured **1.12x** (1.10x and 1.14x on the two prior interleaved
runs; the recorded quiet-machine band across all banked runs is
**1.09x–1.26x, widest observed** — nowhere near the bar). The flags win
big only on the smallest commits (~16x on 16-put commits, where
per-commit sync/write overhead is the whole cost), but small commits on
the plain ramdisk are already at 0.05 ms — a per-commit saving of
~46 µs that no adversarial lane is bounded by. On the shape that moves
wall-clock time (bulk load), the plain ramdisk already captures the
win.

## R5 — memory reality

Method: `vm_stat` sampled before/after loading ~190 MiB of facts
(128 x 4096-fact commits) into a store on the HFS+ ramdisk; deltas are
signed and rough — the whole machine moves under the sample.

| Store on disk (`du`) | wired delta | file-backed delta | free delta |
|---|---|---|---|
| 186,564 KiB | +0 KiB | +186,256 KiB | -367,936 KiB |

What reproduces, and what does not (honesty over coverage — the fixit
record): the **wired delta is ~0 in every banked run** (+0, -576, +0
here; -3,136, -496, +0 earlier) — ram-disk pages are not wired kernel
memory. The **file-backed delta does not reproduce**: recorded runs
measured +172,432 KiB, +708,304 KiB, and **-661,968 KiB** for the same
186,564 KiB store — ambient machine activity dominates the single
vm_stat sample, so no budgeting rule may stand on it (the earlier
"roughly 1:1 with store bytes" rule is withdrawn). What stands:
ram-disk pages surface as reclaimable UBC pressure, not a wired floor,
and the worst-case RAM cost is bounded by the **attach size** (the
`ram://` device is the ceiling). Budget at attach size.

## Verdict carried into Phase 2

- R1: fullfsync is accepted on both ram-disk filesystems — no stop.
- R4: the trigger (>=2x on bulk-shaped commits) did **not** fire.

## R6 — the ephemeral constructor, priced through the real surface

Appended 2026-07-15, after the owner ruling admitted `Db::ephemeral`
(see the amended Phase-2 record below). The R4 cells priced raw
scratch heed environments; R6 prices the SHIPPED constructor — the
full engine write path (typed inserts, judgment, counters) on the four
cells the decision names, interleaved per repetition, under the R4
quiet-machine guard on the bulk shape (the sub-100 µs small cells
absorb single-commit outliers that make a max/min band meaningless;
the bulk cells are the steady co-tenancy witness). Harness:
`ramdisk_phase_r_ephemeral` in the same file; the HFS+ ram disk is
**6 GiB for this test** — a recorded consequence: an ephemeral store's
`MDB_WRITEMAP` ftruncates the data file to the full 4 GiB map at open,
and HFS+ has no sparse files, so ephemeral-on-HFS+ needs map size +
slack (the SSD cells sit on APFS, which is sparse — `du` stays small
there; a 2 GiB HFS+ disk refuses with `StorageFull`, typed).
*Amended 2026-07-16:* the sparse-filesystem overcommit fix put an
`fcntl(F_PREALLOCATE)` in every ephemeral open
(`storage/env/open_env.rs::preallocate`), so the map's blocks are now
reserved on APFS too — `du` shows the full map on EVERY filesystem,
and an undersized volume refuses typed at open uniformly, never a
silent overcommit. The sparse-`du` sentence above is historical.

Three back-to-back unwarned runs; the table records the third; the
two prior runs agree (dividend ratios 4.5x/4.2x/4.4x on the ramdisk,
device tax 1.1x/1.0x/1.0x). Medians (min–max), n=64 small / n=8 bulk:

| Cell | small (16 facts) | bulk (4096 facts) |
|---|---|---|
| `Db::create` @ SSD | 4.864 ms (3.696–7.835) | 14.220 ms (13.136–15.216) |
| `Db::ephemeral` @ SSD | 0.054 ms (0.037–0.098) | 9.090 ms (8.543–9.792) |
| `Db::create` @ HFS+ ramdisk | 0.228 ms (0.121–0.345) | 9.809 ms (8.844–11.567) |
| `Db::ephemeral` @ HFS+ ramdisk | 0.052 ms (0.035–0.121) | 9.135 ms (8.396–9.909) |

What the cells say (run-1 create@ssd small printed a 9.1 ms median
under momentary fullfsync pressure — recorded, not averaged in):

- **The flags dividend on the ramdisk, small commits: ~4.4x**
  (228 µs → 52 µs; 4.5x/4.2x/4.4x across runs). Smaller than R4's ~16x
  scratch-side because the engine's per-commit work (encode, judgment,
  counters) now shares the bill with the sync — the saving is real
  (~180 µs/commit) and the shape is the staging cadence.
- **The flags dividend against durable-on-SSD, small commits:
  ~90x** (83x–158x banked; the fullfsync floor plus DVFS, cf. R3).
  This is the two-store staging pattern's actual before/after.
- **Ephemeral is device-independent in practice: the SSD/ramdisk tax
  on ephemeral stores measured 1.0–1.1x** on small commits and ~1.0x
  bulk — with the kind's flags, the device stops mattering, which is
  what makes ephemeral-on-SSD a legitimate (and zero-setup) cell.
- **The bulk shape re-confirms the R4 refusal's arithmetic through
  the real constructor: 1.07x–1.12x** (create vs ephemeral on the
  ramdisk) — the plain ramdisk path needs no flags on bulk loads,
  exactly as the 1.14x-vs-2x-bar record said. On the SSD, ephemeral
  bulk is 1.56x over durable bulk (the per-chunk fullfsync).

### R6 re-earned (2026-07-16, the 1.0.0-candidate tree)

The zero-known-bugs pass changed the write path (the schema-bound
witness's per-transaction re-check, the determinant map's
probe-first overwrite, the cancel-overlay fix) and the ephemeral
open (the full probe battery, `F_PREALLOCATE`), so the cells were
re-earned under the same protocol — three back-to-back runs, the
third recorded. Medians (min–max), n=64 small / n=8 bulk:

| Cell | small (16 facts) | run-3 dividend |
|---|---|---|
| `Db::create` @ SSD | 4.708 ms (4.027–5.724) | — |
| `Db::ephemeral` @ SSD | 0.064 ms (0.032–0.204) | 74.1x |
| `Db::create` @ HFS+ ramdisk | 0.255 ms (0.113–0.367) | — |
| `Db::ephemeral` @ HFS+ ramdisk | 0.061 ms (0.030–0.167) | 4.2x |

Device tax 1.0x. Per-session dividend bands: SSD 90x (07-15) /
74.1x (07-16) — quote "~75–90x", never the scalar; ramdisk
4.4x / 4.2x. The cold first runs of the session printed 0.10–0.14 ms
small cells (45x/39x) — the warm third is the comparable protocol,
and the drift against 07-15's 0.054 ms sits inside the small-cell
band. Verdict: **the fixes are perf-neutral on the commit path and
the admission decision's arithmetic stands.**

## The Phase-2 record (amended 2026-07-15: refusal → admission)

**The original decision (2026-07-15, superseded the same day, kept
verbatim in history):** Phase 2 refused — no `Db::ephemeral` — because
the R4 trigger (>=2x on the bulk-load shape) measured 1.12x
(1.09x–1.26x band), the plain ramdisk already captured the bulk win,
and the proposed RAM-backedness contract plus the unanswered WRITEMAP
crash-atomicity question were complexity the numbers refused to pay
for. **Its reversal clause was:** "a workload sights ephemeral stores
where the flag delta matters — a lane bounded by sub-100 µs commit
cadence on RAM (the small-commit shape shows ~16x there: 49 µs → 3 µs,
R4)."

**The reversal fired (the owner ruling, 2026-07-15):** the sighting is
the ephemeral relational engine — staging stores judged before
ETL-to-durable, analysis working sets, scratch stores — a lane bounded
by exactly the small-commit cadence the clause names. The owner's
doctrine, recorded verbatim: **"everything we can do to make dogfooding
easier is upgraded to a feature."**

**Decision (standing): `Db::ephemeral` lands first-class as a store
KIND** — a distinct constructor, an on-disk `_meta` kind marker, and
`MDB_WRITEMAP|MDB_NOSYNC` derived from the kind
(`docs/architecture/50-storage.md` § the ephemeral store kind).
**Alternative (the refusal's shape):** an ephemeral constructor
preconditioned on RAM-backed paths. **Why it lost:** the kind carries
the no-machine-crash-durability claim, not the device — R6 measured
the device tax on ephemeral stores at 1.0–1.1x, so ephemeral-on-SSD is
both legitimate and nearly free, and a device precondition would have
refused the pattern's cheapest deployment for nothing. **What
dissolved the refusal's two costs:** the WRITEMAP crash-atomicity
question is now answered empirically (the deterministic crashpoint
sweep runs against ephemeral stores — every point × prefix cell
recovers all-or-nothing; `fuzz/tests/crash.rs`), and the RAM-backedness
contract is gone (no device precondition exists). **What the refusal
got right and keeps:** the bulk-shape trigger still does not fire —
R6 re-measured it at 1.07x–1.12x through the real constructor — so the
plain-ramdisk path needs no flags and the DURABLE constructors remain
exactly as they were; the durability law stands whole on the durable
kind (`00-product.md`). **Reverses if:** the crash sweep ever convicts
a crashpoint on an ephemeral store (drop WRITEMAP, keep the kind), or
the staging sighting itself is retired.
