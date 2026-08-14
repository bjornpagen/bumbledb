# engine-034: `ground_program` and "the whole program" in prepare

- **Severity:** low
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F34
- **Depends on:** none (rename; parallel-safe — textual overlap with engine-001 in build.rs)

## The bug

Main-rule grounding is named for the deleted IR: `build.rs:107` `let (survivors, subsumed) = ground_program(normalized, witness, schema);` (definition ~581-597; interiors/rec ground via `ground_rules`). Doc at `build.rs:23-26`: "Validation and normalization see the whole program". Tests: `plan/ground/tests.rs:688-691` `grounded_program`; comments at `plan/ground.rs:402` and `build.rs:1217-1221` same vocabulary.

## Why it's wrong

The asymmetric pair (`ground_program` for main vs `ground_rules` for derived) implies main IS the program — exactly the pre-cut model (Insight 1). Names are the first documentation; these document the deleted system.

## The fix

Per `audit/CONTRACT.md §C3`: rename `ground_program` → `ground_main`; test helpers `grounded_program` → `grounded_main` (or `grounded_query` where it grounds a whole query); comments "the whole program" → "the whole query", "program order" → "rule order" where they appear in prepare/ground files. `ground_rules` keeps its name (already honest).

## Acceptance criteria

- [ ] Gone: `rg -nw 'ground_program|grounded_program' crates/bumbledb/src` → no matches; `rg -in 'whole program' crates/bumbledb/src/api crates/bumbledb/src/plan` → no matches.
- [ ] Unchanged tests: pure rename — all green, zero assertion edits.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`; `./scripts/lean.sh` (if any Bridge/docs token cites `ground_program`, it moves in the same change — check `rg -rn 'ground_program' lean docs`).

## Constraints

- Rename-only. Coordinate textually with engine-001 (same file).
