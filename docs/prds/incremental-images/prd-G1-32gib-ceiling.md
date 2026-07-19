# PRD-G1 — The 32 GiB ceiling

Repo: bumbledb (`crates/`, `docs/`, `scripts/`, README, CI) · depends on: the
32G scout's report (this PRD's work list is a STUB until it lands — see below)
· gates: `scripts/check.sh` + `scripts/lean.sh`.

## The ruling (owner, verbatim in substance)

> 4 GiB is too low as a hard limit; 32 GiB is the new hard limit, with the
> engineering problems that follow.

This is a doctrine FLIP: the constant is a decision, not a knob
(`storage/env.rs:150-151` — "Not configurable — path-only public surface"), and
the decision changes. A flip is a documented retraction with the new truth
written, never a silent edit: every sentence that states or leans on 4 GiB is
rewritten to state 32 GiB and WHY the old number fell, and the engineering
problems the ruling names are solved or explicitly recorded as accepted costs —
never silently inherited.

## What is known now (the anchors)

- **The constant:** `crates/bumbledb/src/storage/env.rs:168` —
  `const MAP_SIZE: usize = 4 << 30;` → `32 << 30`, with its doc comment
  (`env.rs:150-167`) rewritten: the hard-ceiling paragraph (MDB_MAP_FULL, the
  resize gravestone, "a new store, never a knob" — all unchanged in kind) and
  the container-filesystem paragraph (the numbers change by 8×; see problems).
- **The spec of record:** `docs/architecture/50-storage.md:13-14` ("map_size is
  fixed at 4 GB, comfortably above the 1 GB scale axiom") and `:442` (the
  WRITEMAP consequence: the data file holds the full map).
- **The contributor note:** `README.md:430` — "every store opens as a fixed
  4 GiB memory map" (disk requirements for tests).
- **The scripts:** `scripts/ramdisk.sh:35` (the comment citing the full-map
  ftruncate under `MDB_WRITEMAP`) and `:62` (`SIZE_GIB=5` — sized to hold one
  4 GiB store plus slack; 32 GiB stores do not fit a 5 GiB ramdisk).
- **Test prose:** `storage/env/tests.rs:348,402,409,427-428` — the
  no-4-GiB-fixture probes' comments and the "must fail loudly, not by
  allocating 4 GiB into the byte compare" discipline.
- **The relation to the scale axiom** (`docs/architecture/00-product.md:83`:
  ≤10⁷ facts, ≤1 GB LMDB file, ≤2 GB peak process): the ceiling rises; whether
  the axiom's numbers move is an OPEN QUESTION the scout's report answers —
  the ruling raises the wall, it does not by itself re-true the axiom.

## The engineering problems that follow (named, unsolved here)

1. **Container filesystems materialize the map.** Open ftruncates `data.mdb` to
   the full map; overlayfs materializes it (`env.rs:161-167`). At 32 GiB per
   store, test suites (many stores, many temp dirs) go from "can exhaust a
   container's disk" to "will, almost immediately." CI sizing, the
   real-filesystem contributor note, and possibly the test-store strategy all
   need re-truing.
2. **`preallocate_blocks`** (`storage/env/open_env.rs:94`) runs at the full map
   size — its cost, its failure mode, and whether it stays unconditional at 8×
   the size need measurement/ruling.
3. **The ramdisk strategy** (`scripts/ramdisk.sh`): a 32 GiB-plus ramdisk is
   not a casual ask of a dev machine; the script's sizing, or the
   bench-on-ramdisk doctrine itself for full-size stores, needs a decision.
4. **Sparse-file behavior across hosts** (APFS/ext4 keep it sparse; the
   ENOSPC-on-overlayfs death) — the loud-failure tests in `env/tests.rs` and
   their "never allocate the fixture" discipline must survive at 32 GiB.
5. Anything else the scout finds — address-space, mmap limits, lock-file /
   reader-table interactions, fuzz/crash-sweep store counts, CI runner disk.

## Work list — STUB

**Deliberately a stub.** The 32G scout (a parallel agent of this wave) is
auditing every consequence of the flip; its report REFINES this section into
the real work list — sizes, per-site edits, the CI plan, and the accepted-cost
record. Do not start G1's edits from this stub alone. What is certain
regardless of the report:

1. Flip the constant (`env.rs:168`) and rewrite its doc comment.
2. Sweep every 4 GiB sentence (the anchors above + a fresh
   `grep -rn '4 GiB\|4 GB\|4 << 30'` at execution time) — each becomes the
   32 GiB truth with the retraction stated, or is deleted with its reason.
3. Re-true scripts/tests/CI sizing per the scout's findings; every loud-failure
   probe stays loud (never weaken a test to pass).
4. `scripts/spec-census.sh` clean; `scripts/check.sh` + `scripts/lean.sh`
   exit 0 (no model surface — the map size is below the model).

## Passing criteria (to be sharpened by the scout's report)

- `MAP_SIZE = 32 << 30`; zero remaining assertions that the ceiling is 4 GiB
  anywhere in code, docs, scripts, or README (grep-proven).
- The retraction written at the spec sites (50-storage, env.rs doc comment,
  README note) — the old number named as retracted, the ruling cited.
- Every named engineering problem either solved in this PRD or recorded as an
  explicit accepted cost with the owner's sign-off noted — no silent
  inheritance.
- The full gate battery green on the committed tree, including the env tests'
  no-fixture probes at the new size.

## Size

**Unknown until the scout reports** — the constant flip is XS; the honest sweep
plus CI/scripts re-truing is the real body, plausibly M.
