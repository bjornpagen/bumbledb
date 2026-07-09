# 02 — Engine vs naive-model direction divergence on no-op re-insert + target delete

**Kind:** latent verify-red — the two oracles disagree on a verdict *label* for an
op pattern the generator can legally emit. Not a soundness bug (both sides abort);
a red verify on a label is still a red verify, and `bench` refuses to time without
the stamp. **Decided: the naive model is normative** — "source side" means *facts
this transaction actually added*; the model is implementation-blind by design and
stays that way.

## Current behavior

The pattern: containment `A(x) <= B(y)` holds; fact `a ∈ A` and its target `b ∈ B`
are both committed. One transaction does `insert(a)` (a **no-op** — `a` is already
present) and `delete(b)`.

- **Engine:** the apply phase short-circuits the no-op insert against storage
  (`storage/commit/applier.rs:85-87`) but the delta entry keeps
  `Disposition::Insert`; the source-side judgment iterates **all** insert-disposition
  entries (`storage/commit/judgment.rs:168-169`), runs before the target side
  (`storage/commit/write.rs:79-80`), and reports
  `ContainmentViolation { direction: SourceUnsatisfied }`.
- **Naive model:** computes genuine inserts against the pre-state and skips
  already-present facts, so `a` is never judged source-side; the delete of `b` is
  judged target-side: `direction: TargetRequired`.

Same statement, same abort, different `Direction`.

## The work

**Mechanism (decided): the apply phase produces the genuinely-applied-inserts list
as its output, and the judgment phase consumes that** — not raw delta dispositions,
and not a mutated per-entry flag. Judgment reads what apply *did*, not what the
host *asked*; the phases stay one-directional. The applier already knows (it
short-circuited); today it drops the knowledge.

Consequences to carry through:

- Source-side judgment iterates applied inserts only. This also deletes the wasted
  work of re-judging every redundant insert (visible in `JUDGMENT_SOURCE` probe
  counts).
- The reported violation for the pattern becomes target-side, naming the *source
  fact stranded by the delete* through the R-edge path — check `error.rs` payload
  expectations and any tests pinning the previous message.
- The `==`/totality corner is already correct under this rule (verified in
  review): a no-op parent re-insert plus child delete is caught target-side
  through the parent's standing R edge.

## Acceptance

- A differential regression op-stream: pre-seed `{a, b}`, transaction
  `[insert(a), delete(b)]` — engine and naive verdicts identical including
  `Direction`.
- The op-stream generator gains the pattern class (redundant insert alongside a
  delete of its containment target) so it stays covered under randomization.
- `JUDGMENT_SOURCE` probe counts drop for deltas containing redundant inserts
  (observable via the trace names in `obs.rs`).

## Doc amendments (rule 5)

`30-dependencies.md` enforcement summary: "source side: per **genuinely** inserted
A-fact…" — make the word normative. `60-validation.md` gains the pattern in the
covered-cases list.
