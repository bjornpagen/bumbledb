# engine-011: Program/strata/IDB/predicate vocabulary still structures live data and one false invariant

- **Severity:** high
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F11
- **Depends on:** engine-001, engine-012 (structural halves); the rename halves are parallel-safe

## The bug

Program IR is deleted; the words remain, and in three places they still steer behavior:

1. **A false invariant, stated twice.** `ir/normalize/normalize.rs:16-18`: "The query path: no `Interior` occurrence exists in a sealed `ValidatedQuery` (the query boundary has no predicate address space)". `plan/fj/validate.rs:199,217`: "a sealed `ValidatedQuery` carries no `Interior` occurrence." FALSE — interiors/rec/main all carry Interior occurrences; production routes around it via `normalize_rules`/`validate_with_signatures` while the dead `normalize()` (`#[allow(dead_code)]`, `normalize.rs:25-28`) encodes the deleted claim (engine-030 owns the deletion).
2. **A nonexistent counted surface.** `introspect.rs:39-40` and `exec/introspection.rs:90`: "the counted surface is `stats.strata`" — `ExecutionStats` has no `strata` field (it has `reach: Option<ReachStats>`); `display.rs:23-25` sends readers to "the strata section".
3. **Names on live parameters and output:** `run_join.rs:31-32` `idb_images`/`idb_retired`; `build.rs:107` `ground_program` (engine-034); `display.rs:153` prints `predicate p{}` for Interior sources and `display.rs:87` `interior p{}` (engine-033); `prepared.rs:219-224` "Inert when `rec` is `None`" on a type with no `rec` field; `bumbledb-bench/src/closure.rs:502`-area "the profile path is query-shaped; rec queries skip it"; `tests/reach_finalize_hunt.rs:203` "the two-strata program became"; comments passim "the whole program", "fixpoint program", "query-shaped programs".

## Why it's wrong

Names are the representation readers execute (Insight 1): a comment citing `stats.strata` sends a maintainer hunting for a deleted field; a false "no Interior occurrences on the witness" invariant, stated in two modules, is one refactor away from being *believed* and acted on. The denotation did not keep Program, strata, or IDB — the names did (Insight 2).

## The fix

Present-tense vocabulary sweep, coordinated with the structural issues that own some sites:

- Delete both "no Interior occurrence" claims WITH the dead `normalize()` they excuse (engine-030); `fj/validate.rs` doc rewritten to say what's true (test entry passes an empty surface for EDB-only fixtures, as data).
- `stats.strata` / "strata section" / "per-stratum" prose → `stats.reach.rounds` / "the reach rounds section" (rides engine-007/engine-012).
- `idb_*` → `derived_*` (rides engine-010). `ground_program` → `ground_main` (engine-034). Display strings (engine-033).
- Comment fixes owned HERE (nobody else's structure): `prepared.rs:219-224` budget comment speaks the pipeline arm; `reach_finalize_hunt.rs:203`, wordmap/clear.rs and closure.rs prose (the closure.rs profile-skip CODE change is engine-008; the sentence goes with it); "whole program"/"fixpoint program"/"query-shaped programs" → "whole query"/"reach query"/"cq queries" across `api/prepared/`, `exec/introspection*`.

## Acceptance criteria

- [ ] Gone: `rg -in 'stats\.strata|per-stratum|strata section' crates/bumbledb/src` → no matches; `rg -n 'carries no .?Interior.? occurrence|no Interior occurrence exists' crates/bumbledb/src` → no matches; `rg -inw 'idb' crates/bumbledb/src` → no matches; `rg -in 'whole program|fixpoint program|query-shaped program' crates/bumbledb/src` → no matches.
- [ ] Unchanged tests: `cargo test -p bumbledb` green; rendered introspection snapshots unchanged UNLESS the string change is owned by engine-033 (then `INTROSPECTION_VERSION` bumps there, once).
- [ ] Green: `./scripts/check.sh`; `./scripts/lean.sh` (docs' and Bridge's `crates/…` citations move with renames).

## Constraints

- Prose/rename-only where structure is owned elsewhere — do not fork the structural edits; land after (or with) engine-001/007/010/012/030/033/034 per INDEX.
- No Program vocabulary may survive on live data, parameters, or labels.
