# PRD-U1 — Ephemeral-lazy: one map, no contract, no WRITEMAP

Wave 1 · Repo: bumbledb (`crates/`, `fuzz/`, tests) · depends on: — ·
executes ruling 1 (ratified; do not re-litigate)

## Objective

One `MAP_SIZE = 32 << 30` for both store kinds; the eager capacity contract
retired; the ephemeral flag set reduced to `NO_SYNC` (WRITEMAP dropped — the
recorded fallback, `docs/architecture/50-storage.md` § the ephemeral kind).
The kind system itself is untouched law: on-disk identity, probe-first open,
the constructor × kind cross-open matrix, exhume reading both kinds.

**Tree note (differs from the census).** The census's §1.2 inventory describes
the PR #10 branch's per-kind split. THIS tree never merged it: there is one
`MAP_SIZE = 4 << 30` (`crates/bumbledb/src/storage/env.rs:168`) and no
`StoreKind::map_size()`. The change here is therefore simpler — raise the one
constant, delete the contract, drop the flag — with no prd-G1 retraction to
write (that packet does not exist on main). The census inventory remains the
site map; re-locate every line number.

## Work

1. **The constant.** `MAP_SIZE` 4 GiB → `32 << 30`. Rewrite the doc comment
   above it (`env.rs:~150-168`): the container-filesystem materialization
   paragraph describes WRITEMAP's ftruncate, which this PRD removes for the
   ephemeral kind — re-derive what is still true per kind after the change,
   and record the 4 GiB → 32 GiB flip as a documented retraction in the
   comment (the flip discipline: a retraction, never a silent edit).
2. **The contract dies.** In `crates/bumbledb/src/storage/env/open_env.rs`:
   delete the `preallocate` call in the ephemeral arm (`:69`), `preallocate`
   (`:88-99`), and all three `preallocate_blocks` platform variants
   (`:101-165` — the two sanctioned-unsafe sites die with them). If `libc`
   has no other consumer in the crate (check before cutting), drop the
   dependency and its justification comment (`Cargo.toml:48-52`); if the SysV
   semaphore note or another site still needs it, rewrite the justification.
3. **WRITEMAP dies.** `open_env.rs` ephemeral flags: `WRITE_MAP | NO_SYNC` →
   `NO_SYNC`. Rewrite the SAFETY comment and the module-doc flag story
   (`open_env.rs:6-8,37,54-62`) — the NOMEMINIT interaction paragraph and the
   "durable paths structurally cannot reach WRITE_MAP" sentence both change
   shape. The flag remains DERIVED from the kind; no caller passes flags.
4. **The pins.**
   - `crates/bumbledb/tests/ephemeral.rs` `ephemeral_open_allocates_the_full_map_eagerly`
     (`:129-150` in this tree) — the contract's pin. **DELETE with a
     gravestone comment** naming ruling 1 and this packet; never retarget the
     bound (32 GiB eager cannot run on a ~14 GB CI runner, and weakening a
     bound is forbidden).
   - `ephemeral_refusal_...byte_identical` and the env/tests.rs refusal twins:
     assertions SURVIVE (byte-identity and the probe bound get stronger with
     no WRITEMAP ftruncate at all); resweep their prose ("full 4 GiB map").
   - Probe-first open (`env/ephemeral.rs`): the mechanism STAYS (refusal never
     mutates is a law regardless of flags); resweep its doc prose that
     explains the danger via WRITEMAP's ftruncate — the residual reason is the
     reopen path itself, restate honestly.
5. **The dependents.**
   - `fuzz/tests/kill.rs` — the WRITEMAP commit-window kill sweep: the sweep
     survives as the NOSYNC kill sweep (the crash-safety claim it referees is
     about NO_SYNC ephemeral commits, which still exist); resweep names and
     prose. Never weaken the kill counts.
   - `fuzz/src/lib.rs` `StoreDir::drop` truncate-before-unlink and its 4 GiB
     prose — re-derive: with no WRITEMAP the dirty-page reclamation rationale
     may be void; keep the mechanism only if a stated reason survives.
   - `crates/bumbledb/tests/ramdisk_phase_r.rs` — the ephemeral cells sized to
     "map + slack": with the ftruncate gone the sizing story changes; resweep
     the `attach_sized` docs and cell sizes. The R-lane numbers in the docs
     were measured under WRITEMAP — M re-earns them; mark stale numbers as
     pending-measurement, do not invent new ones.
   - `crates/bumbledb-bench` `--ephemeral` / `ephemeral_twin`: mechanism
     unaffected (kind identity forces the dual load); its recorded numbers go
     stale — pending-measurement marks, M re-measures.
6. **Doc resweep** of the capacity-contract and flag sentences is U6's job;
   this PRD leaves a complete site list in its commit message (the census
   §1.2 doc inventory, re-verified against this tree).

## Technical direction

- **Resolve the ftruncate-ownership disagreement FIRST.** The census reads
  mdb.c as ftruncating the data file to the full map only under WRITEMAP;
  this tree's `README.md:438-439` and `storage/env.rs`'s constant doc claim
  EVERY open ftruncates the full map. One of them is wrong — read mdb.c and
  test on a real store. If durable (non-WRITEMAP) opens also materialize the
  map on non-sparse filesystems, the 32 GiB raise has a durable-side disk
  consequence (containers, small volumes) that must go to the owner as a
  finding BEFORE the constant changes; if WRITEMAP-only, this PRD removes the
  last ftruncate and the docs get simpler. Record the verdict in the commit.
- The false friends stay false: the u32 byte-heap 4 GiB in `error.rs` /
  `resolve_memo.rs` / `error/display.rs` is a different 4 GiB — do not sweep.
  "The 32 GiB map bounds live rows" comments become TRUE at the unified size.
- `docs/architecture/50-storage.md` records NOSYNC-only as the fallback the
  design already priced ("had any point shown partial state") — cite it; the
  differential oracle, crashpoint sweep, and cross-open matrix are the
  standing referees that the reduced flag set still satisfies.

## Passing criteria

- `grep -rn "WRITE_MAP\|preallocate\|F_PREALLOCATE\|posix_fallocate" crates/`
  is empty (modulo gravestone/retraction prose that names them historically).
- One `MAP_SIZE`, no per-kind size anywhere; `StoreKind` itself unchanged.
- The eager-alloc test is deleted with a gravestone naming ruling 1; every
  other ephemeral/refusal/crash/kill test green UNWEAKENED (same assertions
  or stronger; kill counts intact).
- `scripts/check.sh` green, including the full ephemeral differential oracle
  and the crashpoint sweeps; alloc gate green.
- Unsafe count in `storage/env/open_env.rs` reduced by exactly the two
  preallocation sites; no new unsafe anywhere.
- Every stale perf number touched carries a pending-measurement mark pointing
  at M; no number invented.
