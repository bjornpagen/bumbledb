# PRD 10 — `verify_store`: global judgments + CLI

**Depends on:** 09.
**Modules:** `crates/bumbledb/src/verify_store/` (extends 06),
`crates/bumbledb-bench/src/cli/` + `driver/` (the subcommand wrapper),
`crates/bumbledb/src/storage/commit/applier.rs` (one comment).
**Authority:** `30-dependencies.md` (the two judgments — the semantics to
re-verify literally), `60-validation.md` (the validation story it joins).

## Context

PRD 09 verifies the namespaces agree with each other; this PRD verifies the
*judgments* hold globally — both forms, over the full committed state, not
delta-restricted. This catches the class no incremental check can see: "the
incremental form was wrong once, long ago, and every commit since preserved the
corruption." It is the naive model's semantics run against the real store.

## Technical direction

1. **Global functionality:** per key statement, walk the `U` namespace in order
   (it is grouped by guard bytes by construction): duplicate scalar guards are
   impossible by LMDB key uniqueness — so the *real* check is the F-side one PRD
   06 already does (every fact's guard present) plus, for pointwise keys, the
   per-group disjointness walk (already PRD 09 §3). This PRD adds only the
   **cross-check comment** and the report wiring: functionality findings are
   namespace findings; say so in the module doc rather than duplicating sweeps.
2. **Global containment:** per containment statement, for every source fact
   satisfying φ (one F scan per source relation, shared across that relation's
   statements — do not scan per statement): scalar form — probe the target key
   guard and check ψ on the resolved fact; interval form — run the coverage
   walk against the target's guard group (reuse the commit path's walk if its
   signature permits a plain read txn; if it is write-txn-coupled, lift the
   walk into a shared function first — that lift is in scope here and the
   commit path must consume the shared version, not a copy).
   Finding variant: `JudgmentViolation { statement, direction, fact }` — same
   payload style as the commit-time error but as a report finding.
3. **CLI wrapper:** `bumbledb-bench verify-store --dir PATH [--scale/--seed…]`
   (follow the existing subcommand registration in `cli/`/`driver/` exactly):
   opens the store read-only, runs `Db::verify_store`, prints the report
   (findings rendered through the statement renderer where a statement id is
   present), exits nonzero iff findings are non-empty. Update the help text.
4. **The comment that started this:** `applier.rs:64-68`'s "offline sweeper"
   deferral comment now names `Db::verify_store`.

## Passing criteria

- `[shape]` The coverage walk has exactly one implementation consumed by both
  the commit path and the sweeper (grep: one definition).
- `[test]` A store hand-corrupted into a judgment violation the namespaces
  cannot see — delete a target fact's `F`/`M`/`U`/`R` rows *consistently* (so
  every namespace sweep passes) while a source fact still requires it — yields
  `JudgmentViolation` with the right statement and direction. Scalar and
  coverage (interval) cases both.
- `[test]` A clean store yields an empty report through the full pipeline
  including the CLI exit code (unit-level: call the driver fn, not a spawned
  process).
- `[shape]` `applier.rs`'s deferral comment names the tool.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`50-storage.md`: the R-delete asymmetry paragraph names `Db::verify_store` as
the compensating control — making the existing citation true. `60-validation.md`:
the sweeper joins the validation story as the third leg (oracles judge
semantics; the sweeper judges the store), with the CLI name.
