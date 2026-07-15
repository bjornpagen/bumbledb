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

## The Phase-2 refusal (the decision record)

**Decision: Phase 2 refused — no `Db::ephemeral`, no engine code
lands.** **Alternative:** an ephemeral constructor setting
`MDB_WRITEMAP|MDB_NOSYNC` on RAM-backed paths (durable `create`/`open`
untouched). **Why it lost:** the trigger measured 1.12x on the
bulk-load shape against the >=2x bar (1.09x–1.26x, the widest band
observed across every banked quiet-machine run) — the plain ramdisk
already captures the win (~21x small-commit latency, ~94–100x on
back-to-back commit loops, ~1.7x bulk vs the SSD), and an engine
surface that buys a further ~1.1x while adding a second flag regime, a
RAM-backedness contract inside the dependency law, and a WRITEMAP
crash-atomicity question is complexity the numbers refuse to pay for.
The no-sync-mode law stands whole: durability is LMDB defaults, and
`NOSYNC`/`WRITEMAP`/`MAPASYNC` remain inexpressible through the
engine's types (`docs/architecture/50-storage.md`). **Reverses if:** a
workload sights ephemeral stores where the flag delta matters — a lane
bounded by sub-100 µs commit cadence on RAM (the small-commit shape
shows ~16x there: 49 µs → 3 µs, R4), or a bulk-shaped run whose
re-measured trigger ratio reaches 2x. **The re-run protocol (the
quiet-machine rule):** run the harness above on an otherwise-quiet
machine — a co-tenant load once inflated a sequential run to a
spurious 2.27x. The harness interleaves the flag cells per repetition
and prints a warning when any bulk cell's max/min spread leaves the
2x quiet-machine band; a warned run's ratios are not decision-grade
and neither reopen nor re-close this decision. Reopen only on an
unwarned run whose trigger ratio reaches 2x.
