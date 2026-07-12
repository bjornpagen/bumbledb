# PRD 07 — The elision ratchet: revert or re-earn

**Depends on:** 06 and its locked isolated number. The owner's complete
unattended-campaign instruction delegates the branch mechanically: a loss
selects R and a win selects E. This PRD has two written branches until the
measurement picks; no further owner interaction is required.
**Modules:** `crates/bumbledb/src/api/prepared/either_sink.rs`,
`api/prepared/build.rs` (`union_elided` composition),
`api/prepared/introspect.rs` + `api/stats.rs` (the elision observable),
`crates/bumbledb/src/plan/fj/provably_disjoint.rs` (stays — see below),
`crates/bumbledb-bench/src/calendar/families.rs` (`rsvp_union_off`),
`docs/architecture/40-execution.md` § set semantics,
`docs/architecture/30-dependencies.md` (the theorem's consumer list).
**Authority:** the refutation policy, verbatim: "a mechanism that
measures as a loss is reverted, and the record keeps the numbers and the
failure mechanism — deletion is gated exactly like addition."
**Representation move:** the exclusivity THEOREM is untouched either way
— it is a consequence of the declaration, enforced by the checker and
spent by the chase. What stands or falls is the executor's *second
spend*: the empirical claim that cross-rule dedup was pure cost.

## Context (decided shape)

**Branch R (revert — executes iff the isolated number still shows the
elision as a loss):**
- The seen-set always spans multi-rule programs. `union_elided` dies as a
  concept: the sink is built with the spanning seen-set whenever
  `rules.len() > 1`, unconditionally. Single-rule distinct-bindings
  elision (a different, per-rule mechanism with its own license) is NOT
  touched.
- `DisjointWitness` computation (`provably_disjoint.rs`) STAYS: the
  checker's exclusivity enforcement derives from the schema (untouched),
  the chase spends the theorem (untouched), and EXPLAIN keeps reporting
  `disjoint_rules: proven (R.f)` as a diagnostic — a proof is knowledge
  even when unspent. Only its executor consumer dies.
- `force_disjoint_off` and the `rsvp_union_off` family die with the
  mechanism (there is nothing left to force off). The record: a
  **Refutation** block in `40-execution.md` § set semantics carrying the
  numbers (pre-isolation −32% p50 across three runs; the isolated
  number), the failure mechanism as diagnosed (expected: the seen-set's
  D2 cancellation was load-bearing pruning — state whichever mechanism
  the isolated run demonstrated), and the reversal trigger ("a workload
  where the spanning seen-set's cost measurably dominates and the D2
  skip provably never fires").
- `30-dependencies.md`'s "the theorem's third consumer" sentence is
  trimmed to two consumers with a pointer at the refutation block.
- The `union_elided` observable in stats/introspect dies; tests pinning
  it are updated; the alloc-gate's union scenarios keep exercising the
  never-elided regime (they already do — verify the union-rules gate
  scenario still measures the spanning seen-set at zero warm
  allocation).

**Branch E (re-earn — executes iff the isolated number shows a win,
i.e. hypothesis 2 held and the loss was the un-isolated harness):**
- The mechanism stays; the doc's elision paragraph gains the isolated
  number as its citation; the README's honest-losses paragraph drops the
  elision line; `rsvp_union_off` stays as the standing sub-measurement.
- The pre-isolation numbers and the harness defect are recorded in
  `60-validation.md` as a measurement-discipline case study (one
  paragraph: what the un-isolated toggle actually compared).

Exactly one branch lands. The other branch's text is deleted from this
PRD file at execution time and the choice recorded in the commit body
with the adjudicating number.

## Technical direction (branch R, the policy-favored default)

1. Delete the elision composition in `build.rs` (`union_elided = …` and
   the `make_sink` argument it feeds); the sink constructor's
   multi-rule arm takes the spanning seen-set path unconditionally.
2. Delete `EitherSink`'s force/elide plumbing and the
   `force_disjoint_off` entry; delete the introspection field; update
   the four explain/introspect tests that pin the stats shape.
3. Bench: delete the `rsvp_union_off` family registration, its QUERIES.md
   render, and the delta line in the report writer; the report goldens
   update. Tripwires pinning elision structure die with it.
4. Docs per the decided shape above. The EXPLAIN `disjoint_rules` line
   stays (its meaning narrows to "proof available"; reword its
   40-execution sentence accordingly).
5. Verify by grep that `provably_disjoint` retains its two live
   consumers (chase, EXPLAIN) — if the chase turns out not to consume it
   (audit left this unpinned), record THAT in the commit body and keep
   the witness for EXPLAIN alone; do not delete the proof.

## Passing criteria

- `[shape]` (R) `grep -rn "union_elided\|force_disjoint_off\|rsvp_union_off"
  crates docs/architecture README.md` → zero hits; the multi-rule sink construction has no
  elision branch. (E) the elision paragraph cites the isolated number;
  README honest-losses drops the line.
- `[test]` (R) the multi-rule prepared-query tests (union rules, union
  aggregate, disjoint fixtures) pass with the seen-set always spanning —
  the disjoint fixtures now assert `absorbed == 0` at runtime INSTEAD of
  asserting elision (the theorem's truth is still observable: nothing is
  ever absorbed across provably disjoint rules).
- `[shape]` Either branch: this PRD file's other branch is deleted; the
  commit body carries the adjudicating number and names the branch.
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

As specified per branch above — the refutation/case-study block IS the
deliverable's record half.
