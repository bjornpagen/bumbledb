# PRD-G1 — The 32 GiB ceiling

Repo: bumbledb (`crates/`, `docs/`, `scripts/`, README, CI) · refined by: the
32G scout's report (landed; this PRD now carries everything from it that must
survive the packet — the /tmp corpus does not) · gates: `scripts/check.sh` +
`scripts/lean.sh`.

## The ruling (owner, verbatim in substance)

> 4 GiB is too low as a hard limit; 32 GiB is the new hard limit, with the
> engineering problems that follow.

This is a doctrine FLIP: the constant is a decision, not a knob
(`storage/env.rs` — "path-only public surface"), and the decision changes. A
flip is a documented retraction with the new truth written, never a silent
edit: every sentence that states or leans on 4 GiB is rewritten to state the
new truth and WHY the old number fell, and the engineering problems the ruling
names are solved or explicitly recorded as accepted costs — never silently
inherited.

## The structural discovery that reorganized the work (the scout's finding)

The map is materialized eagerly for EPHEMERAL stores only — and for those on
EVERY filesystem, deliberately (the capacity contract,
`storage/env/open_env.rs`: `WRITEMAP` ftruncate on non-sparse filesystems,
explicit `F_PREALLOCATE`/`posix_fallocate` on sparse ones). Durable stores
carry no `WRITEMAP`, so LMDB never extends `data.mdb` at open (the
ftruncate-to-map lives inside mdb.c's `WRITEMAP` branch only) — the durable
map is a pure virtual reservation and `data.mdb` grows with data on every
filesystem, containers included. Consequences:

- the durable 32 GiB flip is nearly free (address space only);
- the ephemeral flip would be 32 GiB of REAL disk (or wired ramdisk RAM) per
  open — one open exceeds a GitHub `macos-latest` runner's ~14 GB total disk,
  a ≥33 GiB HFS+ ramdisk wires a third of the canonical 96 GB machine, and a
  32 GB contributor Mac could not run the sanctioned scratch wiring at all;
- the old container/overlayfs-materialization prose attributed the ftruncate
  to every open — asserted, not observed, and wrong for durable stores;
  corrected everywhere it was copied (a Linux spot-check closes the provenance
  gap — follow-up F4).

## THE DECISION TAKEN — per-KIND map size (the scout's Design A)

**`MAP_SIZE_DURABLE = 32 << 30`; `MAP_SIZE_EPHEMERAL = 4 << 30`**
(`storage/env.rs`, consumed via `StoreKind::map_size()`). Still a decision,
not a knob: no public surface, no env var, no feature; the kind is on-disk
identity so each store's ceiling stays parseable. The resize-gone gravestone
(`storage/commit/write.rs`) survives verbatim — per-kind constants are set
once at open and no resize call exists to race. Rationale: the ruling's
motivation is the durable DATA ceiling; the ephemeral kind is scratch/staging
whose eager full-map allocation makes big maps expensive BY CONTRACT. This
dissolves the CI/ramdisk/contributor-disk walls with zero new surface and zero
test weakening.

**The named trade, accepted:** a 32 GiB ephemeral staging store is impossible.
If the workload inversion ever wants huge staging stores, follow-up F3 (Design
B below) reopens it.

**OWNER SIGN-OFF FLAG:** the scout marked the per-kind split "needs owner
ruling — do not land silently". It is landed here as the only design under
which the ruling, the CI reality (one 32 GiB ephemeral open structurally
exceeds the runner's disk — every ephemeral lane red, not flaky), and the
never-weaken-a-test law are simultaneously satisfiable; this paragraph IS the
loud record. If the owner instead rules ephemeral must also be 32 GiB, the
recorded options are: pay 32 GiB of disk/RAM per ephemeral open everywhere, or
the scout's Design C (a test-scoped compile-time small map via cargo feature —
rejected this wave because test builds would create stores whose on-disk
ceiling silently differs from production's, and feature unification can leak
it downstream; recorded as the least-bad CI mitigation with that dishonesty
window named).

## What landed (the scout's this-wave list, executed)

| # | Work | Where |
|---|---|---|
| G1-b | Constant split + doc comment rewritten as the documented retraction (kind-split materialization correction, hard-ceiling paragraph number-swept, ceiling/axiom decoupling) | `storage/env.rs`, `storage/env/open_env.rs`, `storage/commit/write.rs` |
| G1-c | The mis-scoped materialization prose corrected everywhere it was copied | `README.md` (disk requirements), `50-storage.md` §§ env constants + ephemeral kind, `70-api.md` (probe note), `scripts/ramdisk.sh` |
| G1-d | Scale-axiom decoupling: the map ceiling no longer tracks the axiom — it is the never-resize wall, headroom above the unchanged validated envelope | `00-product.md` § scale axiom, `50-storage.md`, `env.rs` |
| G1-e | Test prose + literal sweep; assertions stay loud and track `MAP_SIZE_EPHEMERAL` via pointer comments (never weakened); the u32 byte-heap 4 GiB named as a false friend and NOT swept | `tests/ephemeral.rs`, `storage/env/tests.rs`, `tests/ramdisk_phase_r.rs`, `error.rs`, `api/prepared/resolve_memo.rs`, `error/display.rs` (untouched — correct as is) |
| G1-f | ramdisk sizing: default stays 5 GiB and stays CORRECT under the per-kind split; prose names the ephemeral constant | `scripts/ramdisk.sh`, `60-validation.md` sizing note |
| G1-g | Ephemeral-open preallocate cost at the shipped (unchanged 4 GiB) size: no change of regime, so no new number owed; the 32 GiB-scale open cost is folded into F1 pending-measurement | this record |
| G1-h | Gates + the execution-time `grep -rn '4 GiB\|4 GB\|4 << 30'` — every remaining hit is a deliberate ephemeral-map or byte-heap sentence | grep-proven at landing |

The image-memory story at the ceiling (what 32 GiB stores mean for the
no-budget cache, the copy-on-append 2× transient, the pinned-reader and
parked-binding retention, the 30–60 GiB plausible peak, and what machine class
that demands) is written at `50-storage.md` § memory discipline; the
bursty-rare retraction and its dependents at `00-product.md` § write design
point, `40-execution.md` D1 (including the fired-trigger reversal record), and
`50-storage.md` § eviction.

## Follow-ups (recorded, not started)

- **F1 (L, owner-gated machine time):** the large-data measurement campaign —
  ephemeral-open cost at big maps; image build wall + peak RSS at 8/16/30 GiB;
  commit latency + freelist length under delete-churn at 10⁸+ facts (LMDB's
  known large-DB pathology: `mdb_page_alloc` scans the freelist linearly, so
  churn at tens of GiB inflates page allocation — the big map enables the
  regime, churn causes it); the `MDB_MAP_FULL` wall exercised functionally on
  a small-map validation build. Prerequisite to ANY perf claim about
  32 GiB-scale operation — until it runs, every such claim is
  pending-measurement, never asserted.
- **F2 (L + owner ruling):** the image byte-budget/eviction doctrine if the
  working set is ever meant to approach the ceiling (nothing exists today: no
  ceiling in `image/build.rs` below `usize`, no cache budget, no
  reap-on-pressure); chunked columns with structural sharing of full chunks as
  the copy-on-append-peak killer (kills the prefix copy; the
  `TransientImage::append` doubling-headroom precedent).
- **F3 (M): Design B — the persisted per-store map size.** A `_meta` size key
  written at creation, read by the probe-first open (the probe pattern already
  exists: the ephemeral constructor probes durable-flagged before the flagged
  reopen), consumed by `map_size()` and the preallocate request. A new meta
  key consulted at open is an encoding change → FORMAT_VERSION bump per the
  version-bump law (`env.rs`). The public surface can stay knob-free (a
  doc(hidden)/feature-gated creation surface serves validation lanes); whether
  a public parameter ever appears is a separate owner ruling. Reopens 32 GiB
  ephemeral staging if ever needed.
- **F4 (S, needs a Linux box):** verify durable-store non-materialization on
  ext4/overlayfs and the ephemeral preallocate's `posix_fallocate` behavior
  (glibc's write-a-byte fallback on non-fallocate filesystems; the ramdisk
  Linux arm stays honestly labeled "written carefully but untested") — closes
  the provenance gap of the retracted overlayfs claim.

## Passing criteria

- `MAP_SIZE_DURABLE = 32 << 30`, `MAP_SIZE_EPHEMERAL = 4 << 30`; zero
  remaining sentences asserting a 4 GiB DURABLE ceiling anywhere in code,
  docs, scripts, or README (grep-proven); every surviving "4 GiB" names the
  ephemeral map or the u32 byte-heap false friend explicitly.
- The retraction written at the spec sites (50-storage, env.rs doc comment,
  README note, 00-product scale axiom) — the old number and the old
  materialization claim named as retracted, the ruling cited.
- Every named engineering problem solved by the per-kind decision or recorded
  above as an explicit accepted cost / follow-up — no silent inheritance; the
  owner sign-off flag on the per-kind split stated loudly.
- The full gate battery green on the committed tree, including the env tests'
  no-fixture probes (their bounds unweakened) and the ephemeral crashpoint/kill
  lanes (unchanged in size by design).

## Size

Landed at M: the constant split was XS; the honest sweep (docs, tests, scripts,
README, packet record) was the body.
