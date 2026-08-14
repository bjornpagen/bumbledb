# docs-030: architecture still teaches `DeltaVariant` and `PreparedBody::Empty` as current

- **Severity:** medium
- **Tree:** docs
- **Status:** FIXED(a52be97a)
- **Source:** adversarial pass (final validation; not in audit/docs.md)
- **Depends on:** engine-007 (`DeltaVariant` dies), engine-023 (`Empty` is not a variant) — this doc describes the post-fix prepared object. Do **not** re-file those engine issues.

## The bug

Present-tense architecture names deleted prepared types as if they were the coordinate:

- `docs/architecture/20-query-ir.md:144-146` — "Interiors-only never enters the reach driver (`PreparedBody::Rules` or `Empty`; …). A rec query runs … one `DeltaVariant` per rec arm"
- `docs/architecture/40-execution.md:81` — "every main rule dies prepares to `PreparedBody::Empty`"
- `docs/architecture/40-execution.md:450`, `:466` — "each rec arm's **one** `DeltaVariant`"; "(`DeltaVariant`, `api/prepared.rs`)"

Shipping code still has those types (`crates/bumbledb/src/api/prepared.rs:314-357`). C3 deletes them: statically-dead main is `rules: []` (Empty is not a variant); a rec arm is `RecArm { delta, rule: FreeJoin }` inhabitable only in `ReachDriver.rec`; `RecursiveRule`/`DeltaVariant` die. docs-011 owns "program" in `40-execution.md` and must not rewrite these names into a third query type.

## Why it's wrong

Insight 1: the execution/IR chapters are the prepared-object teaching surface. Naming `DeltaVariant` (k-variant machinery) and `PreparedBody::Empty` (a third pipeline arm) keeps the pre-sum coordinate alive next to the Query cutover the rest of C7 is deleting. After engine-007/023 land, these sentences are false.

## The fix

Per `audit/CONTRACT.md §C3` + §C7, after those engine issues:

- Interiors-only is the `Cq` / `PreparedPipeline::Cq` arm (or `Reach` without running the driver — interiors then main). Statically-dead main is `rules: []`; the empty fast path is the zero-iteration loop. Do not name an `Empty` variant.
- Rec arms: one plan per rec arm; the unique positive self-atom is the delta occurrence. No "variant" noun. Cite `ReachDriver` / `RecArm` as they land.
- `evalQuery_plain` in `40-execution.md:462` is docs-011's sweep, not this file.

## Acceptance criteria

- [x] Gone: `rg -n 'DeltaVariant|PreparedBody::Empty' docs/architecture/20-query-ir.md docs/architecture/40-execution.md` → no matches.
- [x] The reach-driver facts (one delta occurrence per rec arm, no k-variant mint, interiors-only skips the driver) survive under C3 names.
- [x] No code changes in this issue.

## Constraints

- Prose only. Blocked by engine-007 / engine-023 (otherwise the doc describes code that does not exist yet). Do not invent a third query type. Program vocabulary in the same files is docs-011.
