# PRD 07 — grounding: the pass stops impersonating the chase

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
`ground`: GROUNDING is the Datalog/ASP term for eliminating atoms by
evaluating them over fixed finite extensions — exactly what this pass
does to sealed closed relations. (`join_elimination` — the spec's
alternative — REFUSED: names one effect, not the mechanism;
`closed_fold` — considered — REFUSED by the language law: FP-flavored
where the literature has a word.)

## Context (decided shape) — the rename ledger

- Module: `plan/chase` → `plan/ground` (git mv; `plan.rs` mod
  decl; the re-exports in `lib.rs`).
- Feature: `chase-off` → `ground-off` in all three Cargo.tomls +
  every `#[cfg(feature = "chase-off")]` gate + the thread-local switch
  names (`with_chase_disabled` → `with_grounding_disabled`).
- `scripts/check.sh` matrix line and `scripts/fuzz.sh` follow in the
  SAME commit (policy 7 — the fuzz rewrites target calls the switch).
- Identifiers: every `chase`-stem identifier in the ledger files
  (`shapes_chase.rs` → `shapes_ground.rs`, `ChaseResolvable`-style
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
  repair; the pass GROUNDS the sealed atoms: it evaluates them over
  their fixed finite extensions at plan time, the Datalog term."
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
ground-off` test matrices (engine on/off) before commit.

## Passing criteria

- `[shape]` `grep -rni "chase" crates fuzz scripts docs/architecture docs/reference docs/cookbook.md README.md` → zero.
- `[test]` Engine suite green with `ground-off` ON and OFF;
  `cargo test` in fuzz/ green; bounded rewrites-target smoke green
  (10k runs — the dual-pipeline switch is this PRD's blast radius).
- `[shape]` check.sh matrix + fuzz.sh updated in the same commit
  (grep the scripts for the new feature name).
- `[gate]` Fingerprint pin untouched; clippy workspace `-D warnings`
  with and without the feature; fmt.

## Doc amendments (rule 6)

As in the ledger; `40-execution.md` carries the disambiguation
sentence.
