# PRD 22 — The verifier matrix: one corruption fixture per rebuilt claim

**Depends on:** 08 (verify_store files carry final names).
**Modules:** `crates/bumbledb/src/verify_store/` (facts, determinants,
containment/reverse-edge, coverage, closed-image, fingerprint passes —
enumerate what exists), its test suite, possibly small additions to
the fixture harness (raw-byte write access for tests, following the
crash/HANGHUNT-era precedent of back-door store construction).
**Authority:** brief B6, approved: the Lean model assumes a valid
database state; `verify_store` is the arbiter that stored bytes
realize one. The verifier already rebuilds the semantic indexes (and
the crash fuzz target leans on it continuously) — but "every index has
a corruption fixture that proves the verifier would catch its
corruption" is currently unverified. A verifier pass without a fixture
is a smoke detector never held to a flame.
**Representation move:** none. The verifier's coverage claim becomes a
matrix with a fixture per row.

## Context (decided shape)

1. **The matrix.** Enumerate every claim `verify_store` makes (read
   the module: fact decode validity, interval validity, scalar key
   image parity, pointwise disjointness, reverse containment edges,
   scalar containment satisfaction, coverage satisfaction, closed
   image parity, fingerprint match — the actual list comes from the
   code, not this sketch). One row each: claim × the pass that checks
   it × the corruption fixture that violates it × the expected finding
   (relation, statement, key context per the brief).
2. **The fixtures.** For each row WITHOUT an existing test fixture: a
   test that opens a healthy store, corrupts exactly one artifact
   through raw LMDB access (flip a key-image byte, delete one reverse
   edge, break one interval's halves, remove one closed-image row,
   perturb the stored fingerprint), runs `verify_store`, and asserts
   the finding identifies the corrupted artifact with its context —
   and that a healthy sibling store stays green (no false positives
   from the harness).
3. **Findings quality:** where a pass detects but reports without
   context (no relation/statement identification), upgrading the
   finding payload is IN scope (it is the diagnostics discipline of
   PRD 14 applied to the auditor).
4. **The delete-asymmetry row:** the old audit noted reverse-edge
   delete verification leans on the offline pass — that row's fixture
   is mandatory and its doc sentence in 50-storage states the division
   (online maintains, offline proves).

## Technical direction

Read-first: build the matrix from the code, mark existing fixtures
(several corruption tests exist — census them), write only the missing
ones. Raw-byte corruption helpers live in the test tree, never in the
engine. Every fixture is deterministic (no random corruption — the
byte and location are chosen and commented).

## Passing criteria

- `[shape]` The matrix complete in this file's Results: every
  verifier claim has a fixture row marked pre-existing or added.
- `[test]` Every fixture green (detects its corruption with context;
  healthy control stays clean); full verify_store suite green.
- `[shape]` Finding-payload upgrades (if any) enumerated with
  before/after.
- `[gate]` Full suite green; fingerprint pin untouched; clippy; fmt.

## Doc amendments (rule 6)

`50-storage.md` § the offline verifier: the matrix referenced as the
coverage claim's evidence; the online/offline division sentence.
