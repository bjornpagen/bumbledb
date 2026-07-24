## Stale Lean theorem citation `Txn.judgeB_agrees_of_declared` in the conformance README — outside every census battery

category: incoherence | severity: low | verdict: CONFIRMED | finder: lean:txn-oracle

### Summary

The judgment-case section of the conformance corpus README cites the write-side agreement theorem by a name that no longer exists in the tree. `lean/conformance/README.md:162` says the executable judge agrees with the model "(`Txn.judgeB_agrees_of_declared`)", but the declaration is `theorem judgeB_agrees` (lean/Bumbledb/Decide.lean:1092), whose doc comment states "No premise beyond the merge" — the `_of_declared` suffix names an instance-side premise that was purged. Commit `9851c664` ("feat!: the order purge") performed the rename and updated Decide.lean's own prose citations, but missed this one README. Every other citer in the tree uses the current name. The drift is invisible to CI because both citation-integrity batteries of the spec census scope past this file: this is exactly the drift class the census exists to kill, occurring in a file the census does not read.

### Evidence

All verified directly against the working tree and history:

- `lean/conformance/README.md:162` — `(`Txn.judgeB_agrees_of_declared`). The Rust serializer` — the only hit for that string in the entire repo (`grep -rn judgeB_agrees_of_declared`).
- `lean/Bumbledb/Decide.lean:1092` — `theorem judgeB_agrees {T : Theory} {W : RowInstance}`; its doc comment (lines ~1085-1090) reads "**The two-phase agreement**… No premise beyond the merge."
- Rename provenance: `git show 9851c664 -- lean/Bumbledb/Decide.lean` shows `-…(`Txn.judgeB_agrees_of_declared`; the key phase` / `+NO instance-side premise beyond the merge (`Txn.judgeB_agrees`,` — the commit that killed the premise form updated Decide.lean but not lean/conformance/README.md (its only -S hit there is the introducing commit `5b45b87b`).
- Current-name citers, all consistent: `lean/Main.lean:23`, `lean/Bumbledb/Decide.lean:20` and `:79`, `lean/Bumbledb/Bridge.lean:591` (the ledger row `.row @Txn.judgeB_agrees`), `crates/bumbledb-bench/src/conformance/judgment.rs:7`, `docs/architecture/60-validation.md:68`.
- The census scope hole: `scripts/spec-census.sh:93` — `docs=(docs/architecture/*.md docs/cookbook.md)` fences battery (c), the only battery that resolves `lean/….lean: name` declaration citations (lines 105-118). Battery (d) (lines 128-150) greps lean/ (including `--include='*.md'`) but only for backticked `path.rs::symbol` Rust citations. A backticked Lean declaration name inside lean/**/*.md matches neither pattern; the Lean build only checks the term-level `@theoremName` references in Bridge.lean, not markdown prose.

Cross-checked against the validation contract: docs/architecture/60-validation.md:68 is the doc-side spec citation for this theorem and carries the correct name (`lean/Bumbledb/Decide.lean: Txn.judgeB_agrees`) — battery (c) holds that one honest, underscoring that the README escaped only by living outside the fence.

### Failure scenario

A reader triaging a judgment-case verdict mismatch follows the README — the corpus's own operating manual — to `Txn.judgeB_agrees_of_declared`, finds no such declaration anywhere, and must reconstruct from git history whether the proved artifact was deleted or renamed. More broadly, the census's covenant ("every ledger row's grep-checked half resolves") is silently weaker than advertised: any Lean declaration cited in lean/**/*.md or in crates/ doc comments can drift forever unchecked, and this instance proves the class is live, not hypothetical.

### Suggested fix

1. `lean/conformance/README.md:162`: `Txn.judgeB_agrees_of_declared` → `Txn.judgeB_agrees`.
2. Close the scope hole in `scripts/spec-census.sh`: extend battery (c)'s file set (line 93) to include `lean/**/*.md` (and optionally crates/**/*.rs doc comments) for `lean/….lean: name` citations, or add a small battery (e) that greps backticked `Txn.…`/`Bumbledb.…` declaration names in lean-side markdown and resolves each final dot-segment word-bounded in the Lean sources — the same resolution rule battery (c) already uses (spec-census.sh:105-118), applied where the citations actually live.
