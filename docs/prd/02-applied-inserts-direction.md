# PRD 02 — Applied inserts and judgment direction

**Depends on:** nothing (PRD 11 depends on this).
**Modules:** `crates/bumbledb/src/storage/commit/{applier.rs,judgment.rs,write.rs}`,
`crates/bumbledb-bench/src/naive/differential.rs` (regression stream),
`crates/bumbledb-bench/src/verify/` (op-stream generator).
**Authority:** `docs/architecture/30-dependencies.md` (enforcement summary),
`60-validation.md` (the two-oracle contract — verdicts compared whole, including
`Direction`).

## Context (decided)

The two oracles disagree on a verdict *label* for a legal op pattern: containment
`A(x) <= B(y)` holds, `a ∈ A` and its target `b ∈ B` committed; one transaction
does `insert(a)` (a storage no-op) and `delete(b)`. The engine short-circuits the
no-op insert at apply (`applier.rs:85-87`) but judgment iterates **all**
insert-disposition delta entries (`judgment.rs:168-169`); source runs first
(`write.rs:79-80`) → `SourceUnsatisfied`. The naive model computes genuine
inserts against the pre-state → judges the delete target-side →
`TargetRequired`. Same statement, same abort, different `Direction` — a latent
verify-red. **Decided: the naive model is normative.** "Source side" means facts
this transaction *actually added*; the model is implementation-blind by design
and stays byte-for-byte untouched by this PRD.

## Technical direction

1. **The apply phase produces the genuinely-applied-inserts list as its
   output**, and the judgment phase consumes that — not raw delta dispositions,
   and not a mutated per-entry flag. Concretely: the applier already computes
   applied-vs-no-op per insert entry; collect the applied inserts (the natural
   shape is whatever the judgment loop wants to iterate — fact bytes +
   relation, or indices into the delta's insert set; pick the one that keeps
   judgment's loop signature simplest) and pass it from the apply phase to
   `check_source` through `write.rs`'s existing phase plumbing. Judgment reads
   what apply *did*, not what the host *asked* — one-directional phases.
2. **Source-side judgment iterates applied inserts only.** The wasted
   re-judging of redundant inserts disappears with the bug (observable:
   `JUDGMENT_SOURCE` probe counts drop for deltas containing redundant
   inserts — the obs names registry has the counter).
3. **Consequence check:** for the pattern above the violation now surfaces
   target-side through the standing R edge, naming the *source* fact. Audit any
   test pinning the previous `SourceUnsatisfied` classification for this shape
   and update its expectation with a comment citing the normative rule. The
   `==`/totality corner needs no code: a no-op parent re-insert + child delete
   is caught target-side via the parent's standing R edge (verified in review —
   re-verify with a test, below).
4. **Generator coverage:** the op-stream generator gains the pattern class —
   redundant insert alongside a delete of its containment target — so
   randomization keeps covering it.

## Passing criteria

- `[shape]` `check_source`'s input is the applied-inserts output of the apply
  phase; no judgment code reads raw insert dispositions; no `applied` flag was
  added to delta entries.
- `[test]` Differential regression: pre-seed `{a, b}`, transaction
  `[insert(a), delete(b)]` — engine and naive verdicts identical **including
  `Direction`** (this is the exact case that was red).
- `[test]` The `==`/totality corner: no-op parent re-insert + child delete in
  one delta — both oracles abort target-side on the totality statement.
- `[test]` A delta with a redundant insert and no violation commits, and the
  engine's source-side probe count for it is zero (assert via the stats/trace
  surface, trace feature build).
- `[test]` Op-stream generator emits the pattern class (coverage assertion in
  the generator's contract test).
- `[gate]` Workspace gates green; the naive model's files show zero diff.

## Doc amendments (rule 5)

`30-dependencies.md` enforcement summary: "source side: per **genuinely**
inserted A-fact…" — make the word normative. `60-validation.md`: the pattern
joins the covered-cases list.
