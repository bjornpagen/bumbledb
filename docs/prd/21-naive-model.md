# PRD 21 — The naive model

**Depends on:** 11, 12 (IR), 02, 03 (statements). Buildable in parallel with phases B/C once those land.
**Modules:** new `crates/bumbledb-bench/src/naive.rs` (+ submodule directory), wiring in `crates/bumbledb-bench/src/verify/`.
**Authority:** `docs/architecture/60-validation.md` (§ the two oracles — the naive model's contract), `30-dependencies.md` + `20-query-ir.md` (the semantics it implements literally).

## Goal

The required second oracle: an obviously-correct in-memory implementation of the
data model, both judgments, and the full query semantics — nested loops and
BTreeSets, zero cleverness, zero shared code with the engine's execution or commit
paths (sharing IR/schema *types* is required; sharing *algorithms* is forbidden —
a shared bug is an invisible bug).

## Technical direction

1. **State:** `NaiveDb { relations: Vec<BTreeSet<Vec<Value>>> }` — facts as decoded
   value vectors (`Value` from the IR — the one blessed shared type). Apply a
   write delta as: remove deletes, insert inserts, then **judge every statement
   over the full final state**; on any violation return the statement id (and
   direction) *without applying* — the caller compares verdict and violator
   against the engine's commit result.
2. **Judgments, brute force:**
   - Functionality, scalar: group by projection values; any group with ≥2 facts ⇒
     violation. Pointwise: for every pair sharing the scalar prefix, overlap test
     `a.start < b.end && b.start < a.end` ⇒ violation. O(n²) is the point.
   - Containment: for every source fact satisfying σ (plain value equality), scan
     the target relation for a fact satisfying ψ whose projected tuple matches —
     scalar: equality; interval position: collect ALL matching target segments,
     sort by start, merge, and test the source interval's containment in the
     merged union. No reliance on target disjointness (the model must not assume
     what the engine enforces — it re-derives truth from scratch).
3. **Queries:** evaluate a *validated* `Query` by enumerating the cross product of
   the positive atoms' relations (nested loops), building bindings (membership
   rule: element-typed term binds any `t`? No — evaluate membership as a
   *constraint*: a binding's point value must lie in the fact's interval; point
   variables take values from their scalar anchors, which validation guarantees
   exist), applying predicates (including Overlaps/Contains via the endpoint
   formulas), rejecting bindings any negated atom matches, deduplicating full
   bindings into a BTreeSet, then projecting / folding aggregates per the
   `20-query-ir.md` semantics (Sum via i128; CountDistinct via BTreeSet;
   Arg-restriction as literal restrict-then-project; empty-input global
   aggregates ⇒ empty set). Param sets: substitute before evaluation.
4. **Comparison plumbing** (`verify/`): a differential runner API taking (engine
   db, naive db, operation stream) and asserting, per write: same verdict, same
   violating statement; per query: set-equal results. The *harness executions and
   corpus wiring are human work* — this PRD delivers the model and the comparison
   functions plus their unit tests, not verify-suite integration runs.
5. Size discipline: this is the "smaller than its triggers would be" bet — target
   the whole model under ~800 lines. If it grows past that, it is being clever;
   stop and simplify.

## Out of scope

Randomized generator changes (23), SQLite lane (22), running the suite (human).

## Passing criteria

- `[shape]` No imports from `bumbledb::exec`, `bumbledb::plan`,
  `bumbledb::storage::commit` — only schema/IR/value types.
- `[test]` Judgment goldens against hand-computed verdicts: every PRD 07/08/09
  test fixture re-expressed against the model yields the same verdict and
  violator (write these as table-driven cases in the model's own tests — they
  double as the engine-agreement seed corpus).
- `[test]` Query goldens: the `20-query-ir.md` semantics landmarks — duplicate
  witnesses collapse, the aggregation footgun triples the sum, empty-input
  global aggregate is empty, Arg tie yields both, membership boundaries, negation
  with multiplicities.
- `[test]` One end-of-PRD differential unit test: a fixed 200-op random stream
  (seeded) over a two-relation schema with one `==` pair and one pointwise key —
  engine and model agree on every verdict and every one of 20 fixed queries.
  (This is a unit test co-located with the model, not a harness run.)
