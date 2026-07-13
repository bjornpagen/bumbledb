# PRD 09 — The census floor: re-audit at the new fixpoint

**Depends on:** 04–08 all landed (this PRD measures their combined
effect; it is Phase C's terminal reconciliation and must not start
early).
**Modules:** read-mostly across `crates/`; write access to
`docs/architecture/*` amendment blocks, `docs/reference/
apple-silicon-performance.md` (the staleness ruling), and this campaign's
README ledger.
**Authority:** the audit discipline from the witness campaign (batteries
of greps with expected-zero results, run at the END, so regressions
introduced mid-campaign cannot hide), and policy 5 (docs found wrong get
a conflict block, not a silent fix).
**Representation move:** none — this PRD converts "we believe the
campaign removed the defect classes" into counted, dated evidence, and
re-establishes the unreachable!/assert floor that future audits diff
against.

## Context (decided shape)

The batteries, each with its expected result recorded IN THIS FILE when
run:

1. **Vocabulary battery** (PRD 04/05 closure): the PRD 05 greps re-run
   at campaign end — `PredicateTree`, `.predicates`, `head_types`,
   `result_types`, `column_types`, `resolve_predicates` all zero across
   `crates/` AND `docs/`.
2. **Defensive-check census**: count `unreachable!`, `assert!`,
   `debug_assert!`, `.expect(` across `crates/bumbledb/src`, per file.
   The witness campaign's floor was 147 `unreachable!`; record the new
   floor and attribute the delta to PRDs by mechanism (02's slice APIs,
   08's classified comparisons). Any file whose count ROSE gets an
   explanation or a fix.
3. **Underscore battery**: zero `_`-prefixed parameters/functions
   outside trait-impl-required positions (the standing refactoring-debt
   rule).
4. **Lint posture**: zero `#[allow(` outside the recorded policy sites
   (the one `unsafe_code` policy site is on the record; `expect` is the
   house form). List every survivor with its justification line.
5. **Doc-reference battery**: zero references to deleted chapters or
   pre-reset numbering in `docs/architecture/` and `docs/cookbook.md`.
   Known offenders going in: six "30-execution" references confined to
   `docs/reference/` and `brainlift-sources/` — see the ruling below.
6. **Nightly-dividend battery** (PRD 02 closure): the "fixed-width
   slice" expect count at its recorded target; zero `(|| {` error
   closures.

**The staleness ruling** (decided here, executed here):
`docs/reference/apple-silicon-performance.md` cites pre-reset chapter
numbers and pre-reset measurements. Reference documents are records of
what was true when written — they are not silently renumbered. The
ruling: add a dated header block stating the document predates the
2026-07 reset, that its chapter citations refer to the pre-reset tree at
`1b65ae8^`, and that current measurement doctrine lives in the
architecture chapters; fix NOTHING else in it. `brainlift-sources/` gets
the same one-line treatment in its README if it has one, else a
`PROVENANCE.md` line. Post-reset docs citing dead chapters (if battery 5
finds any outside these two roots) are plain bugs — fix them.

## Technical direction

Run every battery, paste the actual command + count into this file's
"Results" section (append it; the PRD file is the ledger). Where a
battery fails: fix if mechanical, conflict-block per policy 5 if the fix
requires a decision, and never relax a battery to pass. Close by writing
the campaign amendment blocks the earlier PRDs promised, verifying each
landed doc amendment actually exists (grep, don't trust).

## Passing criteria

- `[shape]` All six batteries green with results recorded in this file
  (command, count, date).
- `[shape]` The staleness header block present in
  `apple-silicon-performance.md`; zero other edits to that file.
- `[shape]` Every doc amendment promised by PRDs 01–08 verified present
  (a checklist in the results section, each item with its grep).
- `[gate]` The full workspace gate suite green — this PRD is where
  "green at campaign close" from Phases A–C is actually cashed.

## Doc amendments (rule 5)

The verification checklist above IS this PRD's amendment duty; no new
prose beyond the staleness headers.
