# PRD 12 — `verify`: the oracle command and the stamp

Authority: `00-product.md` success criterion 1 (exactness before any timing claim),
`50-validation.md` (arbitration), README rule 8.

## Purpose

The command that earns the right to time anything: every family query and N
randomized queries produce value-identical result multisets on bumbledb and SQLite,
or the run fails loudly with an arbitration bundle.

## Technical direction

- `verify::VerifyConfig { gen: GenConfig, random_cases: u32 /* default 500 */,
  out_dir: PathBuf }`.
- `verify::run(cfg) -> Result<VerifyReport, VerifyFailure>`:
  1. Generate + load both stores (PRD 08) into `out_dir/db` and
     `out_dir/oracle.sqlite` (fresh directories; delete-and-recreate is the tool's
     own scratch, not user data).
  2. For every read family (PRD 14 registry) × its param sets, and for
     `random_cases` PRD 11 queries × their param sets: prepare + execute on
     bumbledb; translate + prepared-execute on SQLite; `compare::multisets`.
  3. First mismatch: write the **arbitration bundle** to
     `out_dir/mismatch-{n}/`: `query.txt` (IR Debug), `query.sql`, `params.txt`,
     `mismatch.txt` (the Display), plus the golden SQL if the query is a family —
     then continue collecting up to 8 mismatches and return `VerifyFailure`.
  4. Success: write `out_dir/verify.stamp` = hex of blake3 over: crate version,
     the corpus digest (PRD 07), the family-list digest (PRD 14's
     `families::digest()`), `random_cases`, and the seed. `verify::stamp_matches
     (cfg, path) -> bool` is the gate PRD 13's harness and PRD 19's CLI consume.
- Arbitration procedure documented in the module docs: engine-vs-SQLite mismatch
  on a family ⇒ compare translator output against the hand-written golden (PRD
  09); golden ≠ translator ⇒ translator bug; golden == translator ⇒ a human reads
  the semantics docs and rules which engine is wrong. Randomized mismatches:
  minimize by re-running the case's shape at smaller scales (manual; the bundle
  carries everything needed).
- Progress lines to stderr every 100 cases (long L-scale runs must visibly move).

## Non-goals

Timing (nothing here is measured). Automatic case minimization.

## Passing criteria

- Unit tests: stamp determinism (same cfg ⇒ same stamp; any ingredient change ⇒
  different); `stamp_matches` accepts/rejects correctly; the mismatch path — a
  test hook `verify::run_with_sql_override(cfg, |family| Option<String>)` injects
  a deliberately wrong SQL for one family and the run returns `VerifyFailure`
  whose bundle directory contains the four artifact files with non-empty content;
  a full `verify::run` at `Scale::S` with `random_cases = 50` succeeds in a
  `#[test]` (S is sized to keep this a unit-scale assertion).
- `scripts/check.sh` green.
