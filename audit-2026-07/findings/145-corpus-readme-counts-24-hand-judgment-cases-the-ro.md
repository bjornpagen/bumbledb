## Corpus README pins 24 hand judgment cases; the roster and checked-in corpus carry 26

category: incoherence | severity: low | verdict: CONFIRMED | finder: lean:txn-oracle
outcome: fixed 9bcbe8fd

### Summary

The scope-fences section of the conformance README records the corpus composition as "219/325 expressible (200 seeded + 19 hand cases), plus the 24 hand judgment cases outside the report" (lean/conformance/README.md:122). The actual judgment estate is 26: 26 fixtures in the Rust roster and 26 `judgment-*.json` files checked in. The number was correct when written; it drifted when a later commit added a fixture pair without truing it.

### Evidence

- lean/conformance/README.md:122 — `+ 19 hand cases), plus the 24 hand judgment cases outside the report` (verified by direct read).
- crates/bumbledb-bench/src/conformance/judgment.rs — `fn fixtures()` at line 531; `grep -c 'name: "judgment-'` = **26**.
- `ls lean/conformance/cases | grep -c '^judgment-'` = **26**.
- Git archaeology (blame + per-commit counts):
  - The count line was previously trued in lockstep: 19 (5b45b87b) → 20 (f337e113) → 21 (95c60f90 branch) / 23 (5f7c5e6d branch) → **24 at the merge 76e13629**, and 24 was correct at the merge (95c60f90 added the two window-exclusion cases and deleted `judgment-window-vacuity`, net +1 over the shared base).
  - Commit **da761a37** (2026-07-19, "ψ's closed ref earns its fixture pair") added `judgment-closed-ref-psi-valid.json` and `judgment-closed-ref-psi-invalid.json`, bringing the corpus to 26, and did not update line 122 — `git show da761a37:lean/conformance/cases` already shows 26 judgment files against the README's 24.
- So the two unaccounted cases are the **ψ closed-ref pair**, not the multi-citation cases (`judgment-statement-mixed-citations`, `judgment-containment-both-directions`, `judgment-multi-key-collisions`) the original finding named — those three are inside the 24, described at README:239–243. The README paragraph at lines 200–225 (ψ-narrowed closed containment) does describe the ψ material, so as with the multi-citation cases, the prose was updated while the count was not.
- The module docs in judgment.rs (line ~24, "Every fixture is hand-authored") carry no literal count and remain correct; only the README number drifted.
- The suggested fix's mechanism is real: `the_corpus_replays_byte_identical_from_its_provenance` (crates/bumbledb-bench/src/conformance.rs:2121) replays the checked-in corpus from the roster byte-identically, so a roster-derived reference cannot drift.

### Failure scenario

An auditor reconciling recorded coverage against the corpus (exactly this audit's task) reads the two-case shortfall as either an unrecorded deletion or two silently added, unattested cases, and must re-derive the truth from git history — the scope-fences section's explicit promise is "counted, never silent" (README:117), which this literal now breaks.

### Suggested fix

True line 122 to 26 — or, better, drop the literal and point at the roster ("the hand judgment roster, `judgment.rs::fixtures`, replayed byte-identical by `the_corpus_replays_byte_identical_from_its_provenance`"), which removes the only judgment-count literal that has ever drifted while every neighboring count stayed machine-checked.
