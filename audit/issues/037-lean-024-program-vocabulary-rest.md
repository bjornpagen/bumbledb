# lean-024: rest-of-tree Lean comments still say "program"

- **Severity:** low
- **Tree:** lean
- **Status:** FIXED(428005dd)
- **Source:** audit/lean-rest.md L1
- **Depends on:** after lean-001/005 (same-file comment churn on Dedup/Rewrites/Denotation)
- **Conflicts with:** none. Not DUPLICATE(lean-020): that issue is SCC/Tarjan/strata in Syntax/Reach/Main. Not DUPLICATE(lean-019): that issue is the `translate/program.rs` census *path* token. Not DUPLICATE(lean-011): that issue is `idb` on `recDom`.

## The bug

The spec-of-record's comments in modules wave 1 skipped still teach the deleted Program coordinate:

- `lean/Bumbledb/Exec/Dedup.lean:45,94,104,161,513,606,674,711,715,1247,1545,1581` — "every rule of a program", "2+-rule program", "program-wide", "program-level", "HAND-WRITTEN multi-rule programs", "multi-rule program's derivations"
- `lean/Bumbledb/Exec/Rewrites.lean:58,107,1601,2299,2351-2355,2425,2454` — "prepared program", "rewrite step on a program", "program-level face", "lifted to the program"
- `lean/Bumbledb/Query/Denotation.lean:933-935` — "set semantics at the program level" / "every rule of a program"
- `lean/Bumbledb/Query/Aggregates.lean:1542` — "single-rule programs key the whole slot array"
- `lean/Bumbledb/Bridge.lean:287,460` — obligation *prose* ("program level", "program union"). The mechanism *path* `"translate_query (…/translate/program.rs)"` is lean-019. Tokens that name live engine symbols (`ground_program`, `the_empty_program_builds_no_image_and_binds_no_view`) are C8 — they move with engine-034, they are not this finding.

## Why it's wrong

Insight 1: the Lean tree is what humans read to learn the model, and it names the query after the Program artifact the cut deleted — the same defect lean-020 fixes for SCC/Tarjan/strata, in the rest of the tree. `audit/CONTRACT.md §C7`: no `program` in present-tense vocabulary.

## The fix

Per `audit/CONTRACT.md §C7`: present-tense "query" / "rule list" / "prepared pipeline" at every listed comment site. Bridge obligation prose (`:287`, `:460`) rewrites; Bridge *path* tokens that still contain `ground_program` / `the_empty_program_*` wait for engine-034 (C8) and are out of this issue's gone-regex.

## Acceptance criteria

- [x] Gone: `rg -inw 'program' lean/Bumbledb/Exec/Dedup.lean lean/Bumbledb/Exec/Rewrites.lean lean/Bumbledb/Query/Denotation.lean lean/Bumbledb/Query/Aggregates.lean` → no matches; `rg -n 'program level|program union' lean/Bumbledb/Bridge.lean` → no matches.
- [x] Comment-only on the listed files (plus the two Bridge prose strings). Zero theorem/def/name changes in this issue; `lake build` output identical modulo comment hash if any; `./scripts/lean.sh` fully green.
- [x] Out of scope here (do not fail this issue on them): `translate/program.rs` (lean-019); `ground_program` / `the_empty_program_*` path tokens (engine-034 + C8); `Exec/Reach.lean` "old program domain" (lean-011); Syntax/Main SCC comments (lean-020).

## Constraints

- Prose only. No identifier changes (identifier-level Program wrap is lean-005's `RewriteStep : List Rule → List Rule`). Land after lean-001/005 touch Dedup/Rewrites/Denotation.
- No C5 split.
