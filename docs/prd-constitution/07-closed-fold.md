# PRD 07 — closed_fold: the pass stops impersonating the chase

**Depends on:** 06 (serialize the big ir/plan sweeps). Runs SOLO — the
widest rename in the set (~56 files).
**Modules:** `crates/bumbledb/src/plan/chase{.rs,/evaluate.rs,/tests…}`,
the `chase-off` feature (`crates/bumbledb/Cargo.toml:15`,
`crates/bumbledb-bench/Cargo.toml:25`, `fuzz/Cargo.toml:23`, ~45 cfg
sites), `scripts/check.sh` (the feature-matrix line), `scripts/fuzz.sh`,
`fuzz/src/rewrites.rs` (the runtime switch calls), `api/prepared/*`,
`ir/normalize*`, `exec/explain/*`, `api/stats.rs`, bench
`querygen/shapes_chase.rs` + `differential/tests/chase.rs`, and eleven
docs mentions in `40-execution.md` plus scattered others.
**Authority:** audit deep issue #7, verified: to a database theorist
"the chase" means the dependency-theory fixpoint that repairs a
database with fresh values. This pass is query planning: it folds
closed-relation ground axioms into the plan, eliminates atoms whose
join is proven redundant, and resolves filters against sealed
extensions. No fresh values, no repair, no fixpoint. Actively
misleading in a codebase whose pitch is theoretical honesty.
**Representation move:** none — a grep-zero rename. The chosen name is
`closed_fold`: the pass FOLDS CLOSED extensions into the plan (the
repo's own docs describe it this way), and it composes honestly with
the existing `fold` vocabulary — `ir/normalize/fold` folds conditions,
`plan/closed_fold` folds closed atoms; both are folds; the qualifier
disambiguates. (`join_elimination` — the spec's alternative — is
REFUSED: it names one effect of the pass, not the mechanism.)

## Context (decided shape) — the rename ledger

- Module: `plan/chase` → `plan/closed_fold` (git mv; `plan.rs` mod
  decl; the re-exports in `lib.rs`).
- Feature: `chase-off` → `closed-fold-off` in all three Cargo.tomls +
  every `#[cfg(feature = "chase-off")]` gate + the thread-local switch
  names (`with_chase_disabled` → `with_closed_fold_disabled`).
- `scripts/check.sh` matrix line and `scripts/fuzz.sh` follow in the
  SAME commit (policy 7 — the fuzz rewrites target calls the switch).
- Identifiers: every `chase`-stem identifier in the ledger files
  (`shapes_chase.rs` → `shapes_closed_fold.rs`, `ChaseResolvable`-style
  names if present, obs/stats field names, EXPLAIN render strings).
- EXPLAIN output strings that say "chase" change with this PRD — the
  affected goldens update here (values otherwise identical); this is a
  recorded golden churn, not drift.
- Docs: the eleven `40-execution.md` mentions, `30-dependencies.md`
  (2), `60-validation.md` (2), `10-data-model.md` (1), `70-api.md` (1),
  architecture README (2), plus `docs/reference/recursion-design.md`
  seam-ledger rows that name the module (update the paths, not the
  design). One new sentence in `40-execution.md` where the pass is
  introduced: "not the dependency-theory chase — no fresh values, no
  repair; the name records what it does: fold sealed closed extensions
  into the plan."
- NOT renamed: `docs/prd-crucible/` and `docs/prd-constitution/`
  packet files (campaign ledgers record history verbatim — exempt, as
  in the crucible battery exemptions).

## Technical direction

git mv first, then compiler-driven, then feature strings (Cargo.tomls,
cfg gates, scripts, fuzz), then display/docs strings. The feature
rename must be atomic across workspace + fuzz + scripts in one commit
or nothing builds — this PRD is why the campaign rule "tree need not
typecheck between PRDs" exists, but within the PRD the final state must
be whole. Verify `cargo fuzz check` and both `--features
closed-fold-off` test matrices (engine on/off) before commit.

## Passing criteria

- `[shape]` `grep -rni "chase" crates fuzz scripts docs/architecture docs/reference docs/cookbook.md README.md` → zero.
- `[test]` Engine suite green with `closed-fold-off` ON and OFF;
  `cargo test` in fuzz/ green; bounded rewrites-target smoke green
  (10k runs — the dual-pipeline switch is this PRD's blast radius).
- `[shape]` check.sh matrix + fuzz.sh updated in the same commit
  (grep the scripts for the new feature name).
- `[gate]` Fingerprint pin untouched; clippy workspace `-D warnings`
  with and without the feature; fmt.

## Doc amendments (rule 6)

As in the ledger; `40-execution.md` carries the disambiguation
sentence.
