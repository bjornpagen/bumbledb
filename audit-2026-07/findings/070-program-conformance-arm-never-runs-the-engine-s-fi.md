## Program conformance arm never runs the engine's fixpoint driver, despite four docs claiming it does

category: missing-free-feature | severity: medium | verdict: CONFIRMED | finder: lean:txn-oracle

### Summary

The conformance lane names itself three-way, but it is three-way per arm only for queries and judgments. Query cases assert engine-vs-naive on every build and replay (`execute_case`), judgment cases assert `engine_write`-vs-naive, but program cases (`program-*.json`, 27 checked in under `lean/conformance/cases/`) are attested by the naive stratified fixpoint plus SQLite only. `one_program_case` — the single function serving both the corpus builder and the replay path — never touches `world.db`. Consequently the per-push `scripts/lean.sh` gate (battery 5, `three_way_conformance_over_the_checked_in_corpus`) never executes the engine's landed fixpoint driver on the recursive corpus, while the module doc, the Lean driver doc, the parent comparator doc, and the lean.sh comment all say it does. Both ingredients for the engine leg already exist in the crate, making this a free feature left on the table plus a cluster of over-claiming docs.

### Evidence (all verified in the working tree)

- `crates/bumbledb-bench/src/conformance/program.rs:289-292` — `one_program_case` computes answers via `world.naive.program(program, &[])`; the only other oracle is the SQLite twin (`sqlite_program_expressible` / `sqlite_answers`, lines 302-318). No use of `world.db` anywhere in the file; `grep engine program.rs` hits only the doc comment on line 7.
- `crates/bumbledb-bench/src/conformance/program.rs:535-570` — `replay_program_case` re-derives the program from provenance and calls the same `one_program_case`, so the replay/comparator path inherits the missing engine leg.
- `crates/bumbledb-bench/src/conformance.rs:1864-1888` — `replay_checked_in_corpus` dispatches `program-*` files to `replay_program_case`; `conformance.rs:2138` — `three_way_conformance_over_the_checked_in_corpus` is `replay_checked_in_corpus` + `lake exe conformance`. `scripts/lean.sh` battery 5 runs exactly this test per push.
- Contrast, the query arm: `crates/bumbledb-bench/src/conformance.rs:1210-1215` — `let engine = differential::engine_query(&world.db, query, params); assert_eq!(engine, model, "TROPHY (engine vs naive) ...")`. The judgment arm: `crates/bumbledb-bench/src/conformance/judgment.rs:1160-1169` — `differential::engine_write(&db, &delta)` + `assert_eq!(engine, model, "TROPHY ...")`.
- The engine leg is nearly free: `crates/bumbledb-bench/src/differential.rs:292-313` — `engine_program` (currently `#[cfg(test)] pub(crate)`) prepares and executes a program under the fixpoint driver and returns the same `BTreeSet<Tuple>` shape `one_program_case` compares; `crates/bumbledb-bench/src/conformance.rs:187-190` — `World` already carries `pub db: Db<target::Target>` loaded with the same corpus rows.
- The driver is real and landed: `crates/bumbledb/src/api/prepared/fixpoint.rs:264` (`run_fixpoint`); `lean/Bumbledb/Bridge.lean:576` ledger row.
- Over-claiming docs, all checked verbatim:
  - `crates/bumbledb-bench/src/conformance/program.rs:6-8`: the third oracle "now holds the landed fixpoint driver to the same cases" — nothing holds the driver to these cases.
  - `lean/Main.lean:8-9`: "compare against the recorded engine verdicts" — the recorded program answers came from the naive fixpoint (SQLite-corroborated where expressible), never the engine.
  - `crates/bumbledb-bench/src/conformance.rs:22-25`: the comparator replays "per checked-in case ... the engine fresh, the naive model fresh" — false for the program arm.
  - `scripts/lean.sh` battery-5 comment: "replays the corpus through the real engine" — true for two of three arms.
  - `crates/bumbledb-bench/src/conformance.rs:30-34` is stale in the opposite direction ("the third oracle wired for recursion before the engine can run one program") — the driver has since landed.
- Spec check (docs/architecture/60-validation.md § "All three oracles run recursion"): lines 102-112 accurately record the current split — corpus program cases "written only after the naive fixpoint — and SQLite, where expressible — agreed", with the ENGINE held three-way only on the hand closure goldens; lines 812-824 record the engine leg on the querygen recursive-shape arm. So the code follows the spec doc; the four docs above are the drift. The finding's substance survives: the gate that names itself three-way is two-oracle-plus-Lean on its recursive arm.
- Coverage gap is real, not hypothetical redundancy: the querygen engine differential (`crates/bumbledb-bench/src/querygen/tests.rs:564-585`) runs 240 programs from `SEED = 11` (one continuous rng), while corpus seeded cases replay from per-case seeds `PROGRAM_CASE_SEED_BASE = 0x0014_0000 + attempt` (program.rs:58, 484) — disjoint program streams. The recursive goldens (`differential/tests/recursive.rs:204-240`) run the engine on fixed graphs only.

### Failure scenario

A regression in the engine's per-stratum fixpoint driver (`run_fixpoint` — e.g. a frontier/watermark bug in the delta variant) that manifests on a corpus-shaped org-tree program passes `three_way_conformance_over_the_checked_in_corpus` and every `scripts/lean.sh` run untouched, because the corpus answers were recorded from the naive fixpoint and the replay re-runs only naive + SQLite + Lean. Detection then depends on the same shape happening to occur in the querygen stream's 240 seeds — a different rng stream with no guarantee of overlap.

### Suggested fix

Add the engine leg to `one_program_case`, mirroring the other two arms:

```rust
let engine = differential::engine_program(&world.db, program, &[]);
assert_eq!(
    engine, answers,
    "TROPHY (engine vs naive) on program case {name}: triage per the fuzzing charter\n{program:#?}"
);
```

`engine_program` must lose its `#[cfg(test)]` (or gain a non-test twin), since `one_program_case` is reachable from non-test lib code (`replay_checked_in_corpus` is `pub`). Then true the docs to the one reality: program.rs:6-8 and conformance.rs:22-25/30-34 (and, if the engine leg lands, Main.lean:8-9 and the lean.sh battery-5 comment become accurate as written). This makes the recursive arm engine+naive+SQLite+Lean like its siblings, and closes the loop 60-validation.md:87 headlines ("All three oracles run recursion — and so does the engine") over the corpus itself rather than only the goldens and the querygen stream.
