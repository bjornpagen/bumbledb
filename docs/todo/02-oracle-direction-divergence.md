# 02 — Engine vs naive-model direction divergence on no-op re-insert + target delete

**Kind:** latent verify-red — the two oracles disagree on a verdict *label* for an
op pattern the generator can legally emit. Not a soundness bug: both sides agree
the transaction must abort; they disagree on `Direction`.

## Context

The differential runner compares whole `Verdict`s, including the violated statement
id **and** `Direction` (`crates/bumbledb-bench/src/naive/differential.rs:86`). The
engine and the naive model classify one case differently, so the first generated op
stream hitting the pattern turns `verify` red on a label mismatch. The two-oracle
construction is normative (`60-validation.md`) and `bench` refuses to time without
the stamp — this blocks the ratchet the day it fires.

## Current behavior

The pattern: containment `A(x) <= B(y)` holds; fact `a ∈ A` and its target `b ∈ B`
are both committed. One transaction does `insert(a)` (a **no-op** — `a` is already
present) and `delete(b)`.

- **Engine:** the apply phase short-circuits the no-op insert against storage
  (`storage/commit/applier.rs:85-87`) but the delta entry keeps
  `Disposition::Insert`. The source-side judgment iterates **all** insert-disposition
  entries (`storage/commit/judgment.rs:168-169`), so `a` is judged source-side;
  source runs before target (`storage/commit/write.rs:79-80`) and reports
  `ContainmentViolation { direction: SourceUnsatisfied }`
  (`judgment.rs:198-211`).
- **Naive model:** computes genuine inserts against the pre-state
  (`crates/bumbledb-bench/src/naive.rs:110-116`) and skips already-present facts in
  pass two (`naive.rs:150-166,173`), so `a` is not judged source-side; the delete of
  `b` is judged target-side and reports `direction: TargetRequired`
  (`naive.rs:168-185`).

Same statement, same abort, different `Direction`.

## The work

Decide which semantics is normative, then align the other side and pin it:

- **Option A (recommended): the naive model is right.** "Source side" means *facts
  this transaction actually added*; a no-op re-insert added nothing, and the
  violation is genuinely caused by the target's disappearance. Fix: the engine's
  source-side iteration skips delta entries whose apply was a storage no-op. The
  applier already knows (it short-circuited); it needs to record that fact — either
  flip the entry's disposition to a `NoopInsert` state or carry a per-entry
  applied flag into the judgment phase. This also removes over-broad source-side
  work: today every redundant insert is re-judged (harmless but wasted probes,
  counted in `JUDGMENT_SOURCE`).
- **Option B: the engine is right** (any insert-disposition entry is source-judged).
  Then the naive model must judge re-inserts source-side too. This is worse: it
  bakes an implementation artifact (delta disposition) into the semantic model, and
  the model exists precisely to be implementation-blind.

Note the error-payload consequence of A: the violation fact bytes reported switch
from `a` to `b`'s key tuple — check `error.rs` payload expectations and any tests
pinning the message.

## Acceptance

- A differential regression op-stream: pre-seed `{a, b}`, transaction
  `[insert(a), delete(b)]` — engine and naive verdicts identical including
  `Direction`.
- The op-stream generator gains the pattern class (redundant insert alongside a
  delete of its containment target) so it stays covered under randomization.
- `JUDGMENT_SOURCE` probe counts drop for deltas containing redundant inserts
  (observable via the trace names in `obs.rs`) — evidence the over-broad iteration
  is gone, if Option A.

## Doc amendments (rule 5)

`30-dependencies.md` enforcement summary: "source side: per **genuinely** inserted
A-fact…" — make the word normative. `60-validation.md` gains the pattern in the
covered-cases list.
