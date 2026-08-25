# 10 — Endgame: the finite list between the tree and the canon

Everything still open, with acceptance criteria. When every item here is
closed, [00-canon.md](00-canon.md) describes the tree with no remainder
and this folder retires the way its predecessors did.

## E1 — One encoding, one coordinate ([20-one-encoding.md](20-one-encoding.md))

The last representational change. JSON documents die; every protocol
object speaks the batch codec's binary grammar; `Vector` becomes a
first-class type owning sum/domination/order/floor. Acceptance: the
deletions in 20's table are absent from the tree (grep), the document
goldens are binary, and both drivers walk the re-rendered inventory.

## E1b — No raw regex

Every raw regex literal and `new RegExp` in the TypeScript surfaces
(`ts-log/src`, `ts/src`, `examples/lambda/src`) is replaced with the
`arkregex` package's typed patterns — after E1's deletions, not before
(the `CKPT-SCRATCH` text capture and any hex-shape patterns die with the
JSON format; do not port a regex E1 deletes). The lease/temp name
grammars, the key prefix trim, and the `.id` newtype suffix are the
survivors to port. Acceptance: zero raw regex literals in those trees
(grep), refusal identities unchanged.

## E2 — Reconciliation: green once

All test suites migrated to the landed shapes — migration asserts the
new named outcomes (strengthening; loosening any assertion remains a
hard-bound violation) — then the full battery loops until green:

1. `cargo fmt --all --check` · `cargo clippy --workspace --all-targets
   -- -D warnings` · `cargo test --workspace` (repo root)
2. From inside `crates/bumbledb-log/` (its own workspace — root
   commands do NOT reach it): `cargo fmt --check` · `cargo clippy
   --all-targets -- -D warnings` · `cargo test`
3. `scripts/check.sh` · `scripts/lean.sh` (0 disagreements) ·
   `scripts/spec-census.sh`
4. `ts/`: tests + `tsc --noEmit` + biome · `ts-log/`: tests +
   `tsc --noEmit` + biome (via each package's own scripts, `pnpm`)

Acceptance: one transcript, all lanes green, attached to the receipt.

## E3 — Proof

1. **Grep-for-absence transcript**, both drivers
   (`crates/bumbledb-log/src`, `ts-log/src`, `examples/lambda/src`):
   zero hits for `kill(0)` / `kill -0`, `pid_alive`, `pidAlive`,
   `applied_pending`, `upsert`, base64 pending, JSON-`number` u64,
   `refresh_braid`, downward-break sweep, `Ok(status.success())`
   liveness, `batch.header.timestamp` aging, "gc fodder", the TS
   manifest-birth arm — **plus the E1 absences**: `serde_json`,
   `document.ts`, hex-rendered digests in document paths, quoted-decimal
   u64 rendering, `.json` in `StoreKey` spellings, BOM/whitespace/
   leading-zero/duplicate-key arms.
2. **The 141-row adversarial audit** of
   [90-traceability.md](90-traceability.md): every row, no sampling;
   verifiers briefed to *refute* closure; a row passes only when the
   refuter must cite the type or invariant that stopped them, file:line;
   two independent verifiers per critical (rows 0–9), one per
   major/minor; any refutation reopens the owning work.
3. **Lockstep version bump**: every manifest the lockstep gate compares
   (root workspace crates, the napi crate, `bumbledb-c`, `ts/` main +
   both platform packages, `ts-log/` with its peer range) moves to ONE
   new version number in one commit; lockfiles re-derived. No publish,
   no tag — the owner's ceremony (`ts/PUBLISHING.md`).

## E4 — The receipt

One receipt commit reporting: per-stage commit hashes, the battery
transcript, the grep transcript, the 141 verdicts, the rulings log
([RULINGS.md](RULINGS.md), updated with any E1 rulings), the deletion
tally, and the one line the owner runs: `git push origin HEAD`. Then
this folder is eligible for retirement by the owner, the same way
`grail/`, the numbered PRD set, and `representation-first-cutover/`
retired before it.

## Cancelled by rotation

- **Tier A (parent-PRD alignment) is dead.** The parent PRD set is
  deleted; [00-canon.md](00-canon.md) already states the law against the
  landed representations. Do not recreate or edit `proposals/00-*.md`
  through `proposals/90-*.md`; any in-flight alignment work to those
  files is moot.
- The old dispatch (`representation-first-cutover/DISPATCH.md`) is
  superseded by [DISPATCH.md](DISPATCH.md) in this folder.

## Owner-only cleanup (not agent work)

The stale Aug-19 residue: 12 `../bumbledb-audit-lane-*` worktrees and
the `audit-lane-*` branches (one holds 2 unmerged engine commits —
re-derive if ever wanted); ~11 GB of untracked `bench-out/` +
`bench-data/` (regenerable; now candidates for `.gitignore`).
