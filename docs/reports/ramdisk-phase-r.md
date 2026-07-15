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
| Date | 2026-07-15 |
| Runs | three back-to-back runs; the table records the third, and the two prior runs agree within their spreads (the R4 trigger ratio measured 1.16x, 1.12x, 1.14x) |
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
| internal SSD | 4.979 ms (3.899–7.132) | 17.198 ms (16.101–25.283) |
| HFS+ ramdisk | 0.207 ms (0.167–0.277) | 10.942 ms (10.019–12.468) |
| APFS ramdisk | 0.676 ms (0.278–0.996) | 9.995 ms (9.837–10.630) |

The SSD small-commit median matches the ~4 ms fullfsync baseline
expectation. The ramdisk removes the fullfsync floor: ~24x on small
commits (HFS+), ~1.6–1.7x on bulk chunks (where insert work, not sync,
dominates). APFS pays ~0.4 ms more than HFS+ per small commit
(journal/container overhead) and slightly less on bulk.

## R3 — the DVFS dividend

128 back-to-back single-fact commits, aggregate wall time:

| Cell | 128 commits | ratio |
|---|---|---|
| internal SSD | 617.986 ms (~4.8 ms/commit) | — |
| HFS+ ramdisk | 6.550 ms (~0.05 ms/commit) | **94.3x** |

The per-commit arithmetic from R2 predicts ~24x; the measured aggregate
ratio is ~90–95x (all three runs) — the fsync DVFS floor is real: a
back-to-back fullfsync loop holds the cores at low frequency, so the
ramdisk buys more than the subtraction of the sync itself.

## R4 — LMDB flag deltas on the ramdisk (the Phase-2 trigger)

Scratch heed environments on the HFS+ ramdisk (never the engine's env —
the engine's flags are law), 24-byte keys / 16-byte values, same two
commit shapes. Medians (min–max), n=64 small / n=8 bulk:

| Flags | small (16 puts) | bulk (4096 puts) |
|---|---|---|
| default | 0.051 ms (0.039–0.074) | 0.833 ms (0.720–0.893) |
| `MDB_NOSYNC` | 0.006 ms (0.005–0.015) | 0.732 ms (0.635–0.784) |
| `MDB_WRITEMAP\|MDB_NOSYNC` | 0.003 ms (0.002–0.015) | 0.731 ms (0.599–0.744) |

**The trigger does not fire.** The Phase-2 trigger is
`WRITEMAP|NOSYNC >= 2x` over plain-ramdisk-default on the bulk-load
shape; measured **1.14x** (1.16x and 1.12x on the two prior runs). The
flags win big only on the smallest commits (~17x on 16-put commits,
where per-commit sync/write overhead is the whole cost), but small
commits on the plain ramdisk are already at 0.05 ms — a per-commit
saving of ~48 µs that no adversarial lane is bounded by. On the shape
that moves wall-clock time (bulk load), the plain ramdisk already
captures the win.

## R5 — memory reality

Method: `vm_stat` sampled before/after loading ~190 MiB of facts
(128 x 4096-fact commits) into a store on the HFS+ ramdisk; deltas are
signed and rough — the whole machine moves under the sample.

| Store on disk (`du`) | wired delta | file-backed delta | free delta |
|---|---|---|---|
| 186,564 KiB | -3,136 KiB | +172,432 KiB | -409,584 KiB |

Ram-disk pages surface as **file-backed UBC pages, roughly 1:1 with
store bytes**, not as wired kernel memory. Practical rule: budget
ramdisk RAM at attach size worst-case, at store size typical; the pages
are reclaimable pressure on the UBC, not a wired floor.

## Verdict carried into Phase 2

- R1: fullfsync is accepted on both ram-disk filesystems — no stop.
- R4: the trigger (>=2x on bulk-shaped commits) did **not** fire.

## The Phase-2 refusal (the decision record)

**Decision: Phase 2 refused — no `Db::ephemeral`, no engine code
lands.** **Alternative:** an ephemeral constructor setting
`MDB_WRITEMAP|MDB_NOSYNC` on RAM-backed paths (durable `create`/`open`
untouched). **Why it lost:** the trigger measured 1.14x
(1.12–1.16x across three runs) on the bulk-load shape against the
>=2x bar — the plain ramdisk already captures the win (24x
small-commit latency, ~94x on back-to-back commit loops, 1.6x bulk vs
the SSD), and an engine surface that buys a further 1.1x while
adding a second flag regime, a RAM-backedness contract inside the
dependency law, and a WRITEMAP crash-atomicity question is complexity
the numbers refuse to pay for. The no-sync-mode law stands whole:
durability is LMDB defaults, and `NOSYNC`/`WRITEMAP`/`MAPASYNC` remain
inexpressible through the engine's types
(`docs/architecture/50-storage.md`). **Reverses if:** a workload
sights ephemeral stores where the flag delta matters — a lane bounded
by sub-100 µs commit cadence on RAM (the small-commit shape shows
~17x there: 51 µs → 3 µs, R4), or a bulk-shaped run whose re-measured
trigger ratio reaches 2x. The harness above prints the trigger
arithmetic; re-run it before reopening the decision.
