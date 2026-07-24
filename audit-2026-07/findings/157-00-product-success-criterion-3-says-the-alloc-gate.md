## 00-product success criterion 3 still says the alloc gate becomes a CI gate "when CI exists" — CI exists and runs it

category: incoherence | severity: low | verdict: CONFIRMED | finder: r2:docs-vs-code-drift

### Summary

The product doc's normative success criterion 3 (the zero-allocation contract) closes with a pre-CI conditional: "Enforcement today is `scripts/check.sh` (the checked-in gate suite, run before every commit); it becomes a CI gate verbatim when CI exists." CI exists and does exactly that. Three other sources — the workflow file, the gate script itself, and two docs — describe the CI check lane as a live present-tense fact. This is the only surviving "when CI exists" in the repo (verified by grep), and it sits in the document that defines the success criteria, violating that doc set's own rule 5 ("Docs describe the system in the present tense.", docs/architecture/README.md:29). Note: the finder attributed this to rule 6; rule 6 is the no-history rule — the applicable law is rule 5.

### Evidence

- docs/architecture/00-product.md:404-406 — "Enforcement today is `scripts/check.sh` (the checked-in gate suite, run before every commit); it becomes a CI gate verbatim when CI exists."
- .github/workflows/ci.yml:58-59 — "Lane 1 — the workspace gate: exactly scripts/check.sh (fmt, clippy -D warnings, workspace tests + doctests, the release alloc gate, …)"; :86 — `- run: scripts/check.sh`, executed on a `[macos-latest, ubuntu-latest]` matrix.
- scripts/check.sh:2-5 — "CI's check lane (.github/workflows/ci.yml) runs exactly this — on macos-arm64 AND on x86_64-linux"; :32 — the alloc gate itself: `cargo test --features alloc-counter --test alloc_gate --release -- --test-threads=1`.
- docs/architecture/60-validation.md:930-932 — "CI after the deletion (`.github/workflows/ci.yml`): the check lane (`scripts/check.sh`, a macOS + ubuntu matrix …)".
- README.md:657-659 — "CI's check lane runs this whole script natively on an x86_64-linux runner".
- docs/architecture/README.md:28-29 (rule 5) — "When implementation contradicts a doc, the doc is amended in the same change … Docs describe the system in the present tense."

### Failure scenario

A reader consulting the normative success criteria — the document product decisions are checked against — concludes the allocation contract is enforced only by local pre-commit discipline and that CI enforcement is a future milestone. That contradicts the workflow file and two other documents, and misrepresents the strength of the guarantee (the gate actually runs on every push, on two OS targets, including the x86-64 scalar-fallback path).

### Suggested fix

Replace the conditional clause at docs/architecture/00-product.md:404-406 with the present-tense fact, e.g.: "Enforcement is `scripts/check.sh` (the checked-in gate suite, run before every commit), executed verbatim by CI's check lane (`.github/workflows/ci.yml`) on macos-arm64 and x86_64-linux."
