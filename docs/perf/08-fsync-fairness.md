# PRD 08 — Fsync fairness: both engines pay for durability

Authority: `00-product.md` (`synchronous=FULL` pinned; "the same fsync bill"),
`50-validation.md` (the oracle protocol), docs/benchmarks/16 (the fairness
contract as code). Independent of PRDs 00–07.

## Purpose

commit_single measured 5,086 µs (ours) vs 124 µs (SQLite) — a 41× gap that is
almost certainly not engine work but a **durability asymmetry**: LMDB on macOS
issues `F_FULLFSYNC` (a true flush to media, 3–8 ms on Apple SSDs — the 5 ms
matches), while SQLite's default `fullfsync=OFF` issues a plain `fsync(2)`,
which macOS does not propagate through the drive cache. Both sessions claim
`synchronous=FULL`; only ours currently buys it. Make the comparison honest —
in SQLite's direction: it pays full flush too.

## Technical direction

- **Verify the asymmetry first, in-tree.** Read the vendored LMDB source under
  the heed dependency (`lmdb-master-sys`' `mdb.c`) and confirm the Darwin sync
  path (`F_FULLFSYNC` / `MDB_FDATASYNC` handling), and SQLite's `fullfsync`
  default (OFF) in the bundled amalgamation. Record both findings — file and
  line — in the commit message and in the doc amendment below. If the reading
  contradicts the hypothesis (e.g., heed's LMDB does *not* use `F_FULLFSYNC`),
  **stop and re-diagnose before changing anything**: the fix must follow the
  evidence, and the PRD's remaining direction assumes the hypothesis holds.
- **The change**, assuming confirmation: `corpus::configure_sqlite` — the one
  session-setup function shared by the loader, `sqlite_run::open_for_bench`,
  and the write mirrors — gains:

  ```text
  PRAGMA fullfsync = ON;             -- flush to media, like our commits
  PRAGMA checkpoint_fullfsync = ON;  -- and checkpoints too
  ```

  Unconditionally (both pragmas are no-ops off macOS). Each line carries its
  fairness rationale comment, in the same voice as the existing pragmas.
- **The contract asserts it:** `sqlite_run::FairnessCheck::run` gains
  `PRAGMA fullfsync` == 1 (and `checkpoint_fullfsync` == 1) with a named
  failure, so a misconfigured oracle can never flatter us again.
- **Doc amendments, same change:** `50-validation.md`'s oracle protocol
  section and `docs/benchmarks/README.md`'s fairness notes state the rule —
  "under `synchronous=FULL`, both engines flush to media: LMDB via
  `F_FULLFSYNC` (evidence: …), SQLite via `fullfsync=ON` (default OFF lies on
  macOS)" — with the file/line evidence. `00-product.md` criterion 2's "same
  fsync bill" now literally true; no wording change needed unless the evidence
  says otherwise.
- Expected consequence, stated for the human who re-runs: SQLite's
  commit_single p50 will rise from ~124 µs into the same millisecond class we
  pay; write families remain `Kind::Report`. If ours is *still* multiples
  slower after parity, that residue is real engine work — the report will
  finally show it undistorted.

## Non-goals

Weakening our own durability (no `MDB_NOSYNC`/`NOMETASYNC` — the product doc
pins full durability). Group commit, WAL-for-LMDB, or any write-path redesign
(future work, only worth designing against honest numbers). Touching
`synchronous` (already FULL).

## Passing criteria

- `FairnessCheck` unit test: passes on a `configure_sqlite`d connection;
  clearing `fullfsync` (`PRAGMA fullfsync = OFF`) makes it fail with a message
  naming `fullfsync`.
- The write-mirror tests (`sqlite_run`) still pass at S — they assert
  direction and work counts, never microseconds, so the slower commits change
  nothing structurally. If any hidden time-sensitivity surfaces, fix the test,
  not the pragma.
- Both doc amendments landed with the file/line evidence; grep finds
  `fullfsync` in `50-validation.md`.
- Full `verify` S test green (load protocol shares the session config; results
  are unaffected, only load wall-time). `scripts/check.sh` green.
