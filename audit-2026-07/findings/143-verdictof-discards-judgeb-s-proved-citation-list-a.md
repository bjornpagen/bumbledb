## verdictOf discards judgeB's proved citation payload and re-derives it with duplicated filter predicates

category: unification | severity: low | verdict: CONFIRMED | finder: lean:txn-oracle

### Summary

The judgment conformance driver's `verdictOf` (lean/Main.lean) calls the proved executable judge `Txn.judgeB` only for its accept/reject bit, throws the returned violation list away (`| some _ =>`), and rebuilds the citation indices by re-running textually duplicated copies of `judgeB`'s own phase predicates over `indexed 0 T.statements`. The citation list the conformance lane actually compares against the engine corpus therefore comes from unproved IO-shell glue, not from the artifact `judgeB_agrees` covers. Consistency between the two copies is maintained today only by textual identity, and the duplication also makes every statement's `checkB` — O(rows²) pairwise for the key forms — run up to four times on a statement-phase rejection.

### Evidence

- lean/Main.lean:290-300 — the payload discard and the two inline predicate copies:
  ```lean
  def verdictOf (T : Theory) (W : RowInstance) : RVerdict :=
    match Txn.judgeB T W with
    | none => .accept
    | some _ =>
      let keyIdx := (indexed 0 T.statements).filterMap fun (i, st) =>
        if st.isKey && !st.checkB T W then some i else none
      if keyIdx.isEmpty then
        .reject false ((indexed 0 T.statements).filterMap fun (i, st) =>
          if !st.isKey && !st.checkB T W then some i else none)
      else
        .reject true keyIdx
  ```
- lean/Bumbledb/Decide.lean:1017-1023 — the originals, same predicates a second time:
  ```lean
  def keyViolationsB (T : Theory) (W : RowInstance) : List Statement :=
    T.statements.filter fun st => st.isKey && !st.checkB T W
  def statementViolationsB (T : Theory) (W : RowInstance) : List Statement :=
    T.statements.filter fun st => !st.isKey && !st.checkB T W
  ```
- lean/Bumbledb/Decide.lean:1092-1100 — `judgeB_agrees` states agreement in terms of `keyViolationsB`/`statementViolationsB` as Statement lists and set membership (`mem_keyViolationsB`, 1040; `mem_statementViolationsB`, 1060). No theorem anywhere speaks about statement POSITIONS, which is what `verdictOf` emits and the lane compares (verified: no position/index lemma exists in Decide.lean or Main.lean).
- lean/Main.lean:29-31 and 282-289 — the docs admit the design: "`verdictOf` spends `judgeB` for the verdict and re-derives the citation indices with `judgeB`'s own filter predicates". Documented, but not mechanically tied.
- Redundant work (Lean is strict): on a statement-phase rejection, `judgeB` (Decide.lean:1029-1035) evaluates `checkB` on every statement twice (`keyViolationsB`, then `statementViolationsB`), and `verdictOf` evaluates it twice more (`keyIdx`, then the statement `filterMap`) — 4 full passes. `funcB` (Decide.lean:366-369) and `pointwiseKeyB` (Decide.lean:614-618) are pairwise `L.all fun a => L.all fun b => ...`, genuinely O(rows²) per evaluation.
- Corpus contract confirmed: crates/bumbledb-bench/src/conformance/judgment.rs:8-10 — "the MATERIALIZED statement list — indices are the engine's statement ids". Positions ARE the ids, which is exactly why the executable violation lists could be position-tagged from birth.

This matches the audit's doctrine (docs/design/representation-first.md lineage): the phase predicate exists in two representations — a proved filter over statements and an unproved filter over indexed statements — where one representation (position-tagged from birth) would erase the duplicate.

### Failure scenario

Not a live bug — the predicate copies are identical today, and `verdictOf`'s output currently equals the index projection of `judgeB`'s payload. The failure mode is maintenance drift: an edit to `keyViolationsB`'s predicate (e.g. a new statement form changing the `isKey` reading) that misses Main.lean's copy leaves `judgeB_agrees` elaborating green while the lane compares citation lists no theorem speaks for. Mitigation exists but is weak: if a corpus case exercises the divergence, the lane fails with a MISMATCH — but per the driver's own triage doctrine (Main.lean:10-12) that mismatch would be investigated as an engine/naive-model/spec bug, when the actual defect is lane glue. If no case exercises the divergent form, the drift passes silently. Secondary cost: the 4x re-evaluation of O(rows²) checkers is conformance-lane-only (IO shell, milliseconds), not an engine hot path — cosmetic, not a bench item.

### Suggested fix

Make positions part of the representation instead of re-deriving them: define `keyViolationsB`/`statementViolationsB` over `indexed 0 T.statements` returning `List (Nat × Statement)` (positions are the engine's statement ids by the corpus contract), have `judgeB` return the indexed payload, restate `judgeB_agrees`/`mem_*ViolationsB` membership over the `.2` projection (the proofs are filter-membership arguments and port directly), and collapse `verdictOf` to a single pattern match on the proved output. One filter per phase, one `checkB` evaluation per statement per phase, zero duplicated predicates, and the compared citation list becomes the proved artifact itself.
