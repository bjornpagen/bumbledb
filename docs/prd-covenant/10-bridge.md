# PRD 10 — The bridge: the obligation ledger becomes machine-listable

**Depends on:** 02–09 (it indexes every theorem).
**Modules:** `lean/Bumbledb/Bridge.lean`, `scripts/lean.sh` (grows the
census hooks), `scripts/spec-census.sh` (new),
`docs/architecture/30-dependencies.md` (its prose table RETIRES here —
the first deletion of the campaign, ahead of PRD 11, because the
bridge replaces it one-for-one).
**Authority:** the constitution's theorem↔evidence table (11 prose
rows) — correct but unenforceable; the covenant's zero-duplication
law: a table maintained by hand in markdown IS duplication once the
theorems exist.
**Representation move:** the Lean↔Rust boundary becomes a Lean data
structure the toolchain checks half of and a script greps the other
half of.

## Context (decided shape)

1. **`Bridge.lean`** — a literal Lean value:

   ```lean
   structure Obligation where
     theoremName : Name          -- checked: resolves via `by exact?`-free
                                 -- reference (see direction) or `#check`
     premise     : String        -- one sentence: what Lean assumes
     mechanism   : String        -- the Rust discharge site, exact:
                                 -- "crate::Interval::new (interval.rs)"
     instrument  : String        -- what empirically watches the seam:
                                 -- a test name, a fuzz oracle, a battery

   def ledger : List Obligation := [ … ]
   ```

   Required rows (minimum — every Level-0/1/2 premise that Rust
   discharges): interval nonemptiness → `Interval::new`; encoding
   order-embeddings → the exhaustive order suites; exact-field-set
   acceptance → `resolve_target_key` + its locks; pointwise
   disjointness premise → `DisjointDeterminantProof` + the verifier's
   overlap fixture; distinct-bindings licence → `DistinctWitness` +
   the two-regime test; disjoint-arms licence → `DisjointWitness`;
   grounding preservation → the `ground-off` dual-pipeline fuzz
   target; key-probe shape → the KeyProbe suite; safety/range
   restriction → `NegatedVariableUnbound` + the hostile locks;
   checked sums → the overflow locks; Pack canonicality → the sweep
   suites; final-state judgment → `FinalStateView`; complete
   violations → the citation trophy; ETL identity → recipe 28's test;
   answer identity → the seen-set suites. (The executor enumerates
   exhaustively from 02–09's Bridge notes; the count lands in this
   PRD's Results.)
2. **The Lean half is CHECKED**: each `theoremName` is referenced so
   that a renamed/deleted theorem breaks the build — realize by making
   the field not a string but a cheap reference (e.g. a `#check`
   block per row, or an `abbrev _row_i := @theoremName` line — the
   executor picks the lightest mechanism that makes `lake build` fail
   on a dangling name, records it).
3. **`scripts/spec-census.sh`** — the Rust/docs half: (a) every
   `mechanism` token greps to an existing path/symbol in `crates/`;
   (b) every `instrument` token greps to an existing test fn or fuzz
   target; (c) the docs-side citation integrity check for PRDs 11/12:
   every `lean/…` citation in `docs/architecture/` resolves to a real
   theorem name (grep the tree). Exit nonzero on any dangler.
4. **The prose table retires**: `30-dependencies.md`'s
   theorem↔evidence section is REPLACED by three lines — what the
   bridge is, where it lives, how the census checks it. (PRD 11 does
   the rest of that chapter; this PRD does only the table, because
   leaving two ledgers alive even one PRD longer violates the law
   this campaign exists for.)

## Technical direction

Write the ledger FROM the module docs' Bridge notes (02–09 each named
their consumers — collate, verify each against the tree, then delete
those inline notes in favor of the central ledger IF duplication
results; a one-line pointer per module is the allowed residue).
`spec-census.sh` follows check.sh's conventions; wire it into
`scripts/lean.sh` (one entry point) and the CI lean job.

## Passing criteria

- `[shape]` `ledger` complete (count recorded; every 02–09 premise
  present); dangling-theorem breakage demonstrated once (rename a
  theorem locally, show the build fails, revert — note it in the
  Results).
- `[shape]` `spec-census.sh` exit 0; deliberately break one mechanism
  token, show nonzero, revert (Results note).
- `[shape]` The prose table gone from `30-dependencies.md`; the
  three-line pointer present; `grep -c "Lean theorem" docs/architecture/30-dependencies.md`
  reflects the removal.
- `[gate]` `scripts/lean.sh` (build + placeholders + census) exit 0;
  CI green.

## Doc amendments

The table replacement above; nothing else (11/12 own the rest).
