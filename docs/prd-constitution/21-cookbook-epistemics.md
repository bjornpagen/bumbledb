# PRD 21 — Cookbook epistemics: every recipe wears its label

**Depends on:** 11 (tiling corrected), 12 (`==` theorem stated), 20
(maintenance recipe exists) — this PRD labels the FINAL corpus.
**Modules:** `docs/cookbook.md` (every recipe), `crates/bumbledb-query/
tests/cookbook.rs` (negative witnesses where missing), `docs/formal/`
cross-references.
**Authority:** brief B8, approved: the cookbook is the persuasion
surface — its claims must be exactly as strong as their proofs. The
audit's label discipline (Lean theorem / theorem + validator premise /
host discipline / intentionally refused / documentation error) becomes
a visible, verifiable property of every recipe.
**Representation move:** none. Claims get provenance; over-claims get
corrected; under-tested claims get their negative witness.

## Context (decided shape)

1. **The label line.** Every recipe gains one line under its heading:
   `Guarantee: <label> — <one clause naming the source>`. Examples:
   - "Lean theorem + validator premise — key-backed correspondence
     (`KeyBackedEquality.unique_target`); both projections must be
     declared keys" (discriminated-union recipes);
   - "validator premise — per-group disjointness (pointwise key)";
   - "host discipline — freshness under a generation witness; the
     dependency proves soundness only" (derived-facts recipes);
   - "theorem of the primitives — mutual coverage; see § exact
     partition" (recipe from PRD 11).
2. **The classification pass.** Every recipe (roster ~27 by now) is
   audited against its label: claims stronger than the label supports
   are REWRITTEN (each rewrite listed in this file's Results with
   before/after); claims the label supports but no test witnesses get
   their witness.
3. **Negative witnesses.** Each recipe whose guarantee has a failure
   mode gains one negative case in the compiled copy's test where
   missing: the union recipe's double-arm rejection, the optional-
   child's second-child rejection, the closure idiom's staleness note
   (host-discipline recipes get their failure documented, not tested —
   the label says why), the coverage recipes' gap rejection. Audit
   what exists first — many negatives already live in the schema
   reject suites; a pointer in the recipe satisfies the criterion
   (don't duplicate tests).
4. **The sync law extends:** the token-identity test now also asserts
   every recipe HAS a label line (mechanical check on the doc block).

## Technical direction

One pass over the corpus with the theorem↔evidence table (PRD 01) open
— labels cite table rows where one exists. The Results section carries
the full recipe × label × witness matrix. No recipe is deleted; no
claim is strengthened; the direction of every edit is downward-or-
equal in strength, cited.

## Passing criteria

- `[shape]` Every recipe carries the label line (the sync test
  enforces — it fails on a labelless recipe).
- `[shape]` The recipe × label × witness matrix complete in Results;
  every over-claim rewrite listed before/after.
- `[test]` Cookbook suite green; every new negative witness green;
  zero token-sync drift.
- `[gate]` Docs + tests only; full suite green; fingerprint pin
  untouched.

## Doc amendments (rule 6)

This PRD is its amendments.
