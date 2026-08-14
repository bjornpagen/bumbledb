# docs-023: cookbook recipe 30's example relation is named `Program`

- **Severity:** low
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F23
- **Depends on:** none (prose+example; same file as docs-022)

## The bug

`docs/cookbook.md:1483,1498-1499` — recipe 30's worked schema:

> The schema says `Program(grp) -> Program` — one program per group
> `Program(grp) <= Grp(id);`
> `Program(grp) -> Program;    // one program per group — the callable law`

## Why it's wrong

After the IR type `Program` is deleted, a worked example named `Program` collides with the deleted coordinate (Insight 1): a reader hunting the deleted type greps the docs and lands in a training-course schema, unsure which Program the codebase means.

## The fix

Per `audit/CONTRACT.md §C7` (the cookbook rename ruling): rename the example relation (`Course`, `Track`, or `Offering` — pick one, apply consistently through recipe 30's schema, rules, and prose). The law reads "one row per group" (or "one course per group"), never "one program".

## Acceptance criteria

- [ ] Gone: `rg -n 'Program' docs/cookbook.md` → no matches.
- [ ] Recipe 30's structure (functional-dependency law, rules, expected results) unchanged apart from the name.
- [ ] The recipe's CODE twins rename in the same change: `crates/bumbledb-query/tests/cookbook.rs:707-714, 2183-2230` (the `relation Program { … }` fixture, `ProgramId`, `ProgramByGrp`, and recipe-30 test body) AND `cpp/tests/cookbook/r30_keyed_read.cc` — `rg -nw 'Program|ProgramId|ProgramByGrp' crates/bumbledb-query/tests/cookbook.rs cpp/tests/cookbook` → no matches after; `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-query --test cookbook` green; `cd cpp && cmake --build --preset dev && ctest --preset dev` green.
- [ ] The other prose twins rename too: `docs/cookbook.md:1512,1517` (`ProgramId`, `scan_facts::<Program>()`) and `docs/architecture/70-api.md:1206` ("program-by-grp") — `rg -n 'program-by-grp' docs` → no matches.

## Constraints

- Rename only; the recipe teaches the same law.
