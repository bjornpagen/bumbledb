# 17 — Stale vocabulary and missing tables in docs and Lean prose

- **Status:** OPEN (DOC-01–04 fixed in tree; these rows verified open in
  the second pass and unchanged; the tree is hot).
- **Severity:** should-fix (documentation only; no code).
- **Supersedes:** DOC-05, DOC-06, DOC-07, LEAN-01, REP-15, SPINE-09's naming
  half.

## Principle

The census discipline the agents built (`scripts/spec-census.sh`) is the
right idea: docs are a projection of the code, and drift is a defect. These
are the rows the census does not yet catch because they are prose, not
tokens.

## Evidence and fixes

1. **DOC-05** — `docs/architecture/60-validation.md` lacks the
   incremental-vs-complete-admission fence table (closed-source containments
   lift under L5). The table exists in `lean/conformance/README.md` and the
   bench lane doc; step 15 of the proposal names it for 60-validation.
   *Fix:* one table, three citations.
2. **DOC-06 / REP-15** — colloquial "snapshot" survives in
   `docs/cookbook.md` (recipes 20, 28), `00-product.md`, `50-storage.md`,
   `10-data-model.md`, and public rustdoc on `ReadInstance` / `Db::prepare`
   / point reads. The API words are `ReadInstance` and `Witness`; README's
   "MVCC snapshots" (LMDB semantics) is fine and stays.
   *Fix:* vocabulary sweep; add "snapshot" (API sense) to the census's
   deleted-token list so it cannot return.
3. **DOC-07** — `lean/conformance/README.md` says "pin is 268 answers";
   actual is 277 including 9 `complete-*`. *Fix:* the number, plus a note
   that the count is pinned by the lane so the prose can cite the pin
   instead of restating it.
4. **LEAN-01** — `lean/Bumbledb/Txn.lean` module-doc/bridge prose cites
   `Snapshot.read` and `ForeignSnapshot`; Rust is `ReadInstance`,
   `ForeignWitness`, `Witness<S>`. The mathematical `structure Snapshot`
   stays (it is the consistent-state premise, not the deleted API — recorded
   in kept.md). *Fix:* prose only.
5. **SPINE-09 (naming half)** — `ParkedReader` comments still say
   "snapshot"; the parked read lease wording lands with the same sweep.

## Acceptance

- `spec-census.sh` extended with the API-sense "snapshot" token and green.
- 60-validation carries the fence table; conformance README count matches
  `ls lean/conformance/cases | wc -l`.
