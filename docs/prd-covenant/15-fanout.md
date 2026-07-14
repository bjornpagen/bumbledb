# PRD 15 — The fanout: the spec judges the implementation, subsection by subsection

**Depends on:** 14 (the tree is complete, zero-sorry, census-closed).
The campaign's true terminal — 14 closes the SPEC; this PRD spends it.
**Modules:** read-everything; write access to `docs/reports/
spec-fidelity/` (new) and, for any fix that falls out, the normal
commit discipline (engine fixes are standalone commits, trophy rules
apply).
**Authority:** the owner's directive verbatim: "verify the lean
version, have fable compare it to the rust version in a full fanout at
the very end to spot potential bugs. every single one of those fanouts
should write a report on their subsection grading the rust
implementation."
**Representation move:** none. The campaign's payoff: with the spec
formally checked, every divergence between it and the Rust is by
definition a finding — a bug, a naive-model drift, or a spec error —
and the fanout hunts them exhaustively.

## Context (decided shape)

One review agent per spec↔implementation pairing, run in parallel,
each blind to the others. The pairings (one report each):

| # | Lean subsection | Rust surface |
|---|---|---|
| 1 | `Values.lean` | `value.rs`, `interval.rs`, `encoding/{encode,decode}.rs`, the exhaustive order suites |
| 2 | `Dependencies.lean` (acceptance) | `schema/validate.rs` (exact-set rule, pointwise gate, selections, closed refusals) |
| 3 | `Dependencies.lean` (judgment) | `storage/commit/judgment.rs` (keys, containment, coverage dispatch, violations) |
| 4 | `Query/Syntax+Denotation.lean` | `ir.rs`, `ir/validate/*` (matching, safety, typing), `ir/normalize/*` (DNF, conditions) |
| 5 | `Query/Aggregates.lean` | `exec/sink/aggregate/*` (folds, groups, checked sums, Arg, empty-global), `interval/sweep.rs` (Pack), `allen.rs` |
| 6 | `Exec/Sweep.lean` | `interval/sweep.rs` + `judgment.rs::check_coverage` (the premise plumbing) |
| 7 | `Exec/Dedup.lean` | `exec/sink.rs` seen-sets + union regime, `plan/fj/provably_{distinct,disjoint}.rs`, the witness spends |
| 8 | `Exec/Rewrites.lean` | `plan/ground/*`, the KeyProbe lowering, statically-empty folds, the latch |
| 9 | `Txn.lean` | `api/db/{write,read,snapshot}.rs`, `storage/commit/*` (final-state view, generation compare, bulk_load/scan) |
| 10 | `Bridge.lean` + the naive model | `crates/bumbledb-bench/src/naive/*` — the SECOND implementation audited against the spec (the shared-misreading hunt is two-sided) |

Each agent's mandate:
1. Read the Lean subsection FIRST (the normative side), then the Rust.
2. For every theorem/definition: locate the implementing code; judge
   fidelity — does the code compute exactly the modeled function,
   under exactly the modeled premises? Every premise: who discharges
   it, and is the discharge site the one Bridge claims?
3. Hunt the three divergence classes: (a) Rust behavior the spec
   forbids (BUG — the jackpot), (b) Rust behavior the spec doesn't
   determine (UNDERSPECIFICATION — a spec gap to record or close),
   (c) spec claims no code implements (SPEC ERROR or dead theorem).
4. Write `docs/reports/spec-fidelity/NN-<subsection>.md`: a per-
   theorem fidelity table, every divergence with file:line on both
   sides, and a GRADE (A–F) of the Rust subsection against the spec,
   with the grade's one-paragraph justification. Honest grades — an
   A must mean "no divergence found under adversarial reading," not
   "looks fine."
5. NO fixes inside the review (blind agents don't mutate the shared
   tree); findings return to the orchestrator.

The orchestrator (me) then: consolidates into
`docs/reports/spec-fidelity/00-summary.md` (grades table, findings
ranked, reconciliation verdicts); every class-(a) finding is triaged
under trophy discipline (minimal repro, standalone fix commit or
recorded deferral with reason); class-(b) gaps become recorded spec
obligations (Countermodels/Bridge additions in a follow-up commit);
class-(c) errors fix the spec (with the gate law observed — same
commit updates any doc citation).

## Technical direction

Launch all ten in parallel (they are read-only). Each prompt carries:
the subsection file list, the mandate above, the grade rubric, and
the house citation discipline. Reports are evidence documents — every
claim carries file:line on both sides. The consolidation pass
re-verifies every class-(a) finding before it becomes a fix (the
adversarial-verify discipline: a plausible-but-wrong finding must not
survive to a commit).

## Passing criteria

- `[shape]` Ten reports + the summary exist; every report carries the
  per-theorem table, divergence lists with dual citations, and a
  justified grade.
- `[shape]` Every class-(a) finding: re-verified, then fixed in a
  standalone commit OR recorded as a deferred trophy with reason;
  zero silently dropped.
- `[shape]` Every class-(b)/(c) item: a Countermodels/Bridge/spec
  commit or a recorded refusal.
- `[gate]` After reconciliation: full gates green (workspace + lean +
  spec-census + conformance corpus) — the two spec halves agree at
  campaign end, with every known divergence either fixed or on the
  record.

## Doc amendments

The summary report IS the record; `60-validation.md`'s oracle roster
gains one line: the fidelity fanout as a repeatable audit instrument.
