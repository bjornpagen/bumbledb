# PRD 09 — The chase: surfaces and coverage

**Depends on:** 08. **After this PRD lands, the orchestrator runs the full
two-oracle verify and it must be green before any later PRD starts.**
**Modules:** `crates/bumbledb/src/exec/explain/` + the stats surface
(`api/stats.rs`), `crates/bumbledb-bench/src/querygen/`,
`crates/bumbledb-bench/src/verify/` (differential wiring), docs.
**Authority:** `40-execution.md` (EXPLAIN: mechanism-names-its-reader),
`60-validation.md` (generator coverage contract).

## Technical direction

1. **EXPLAIN/stats:** eliminated occurrences surface in the EXPLAIN report and
   the structured stats, read directly from PRD 08's `eliminated` marks (no
   separate list exists) — occurrence, relation name, and the licensing
   statement id rendered through `schema/render.rs` (e.g.
   `eliminated: Grading via Grading(id | kind == Deterministic) == DeterministicGrading(grading)`).
   The reader is EXPLAIN plus the DP (which sees a smaller problem) — say so in
   the module doc.
2. **Generator coverage:** querygen emits eliminable shapes deliberately — the
   existence-walk (join the containment target on the full key, use no other
   target field) and the DU one-sided shape (both `==` directions) — plus
   near-miss variants that must NOT eliminate (one extra projected target
   field; missing φ), so the differential exercises both the rewrite and its
   refusals. Coverage-contract test extends with the new shapes' bands and a
   structural assertion that both an eliminated and a refused shape appear per
   run.
3. **Differential wiring:** the verify randomized lane needs no structural
   change — the naive model computes the unrewritten query, which *is* the
   differential test. Add the explicit dual-run check where it is cheap: in the
   bench crate's differential unit tests (not the harness), run each eliminable
   fixture through the engine twice via PRD 08's test-only switch (chase on /
   off) and three-way compare with the model.
4. **The decision block** (rule 5, into `40-execution.md`): chase-based
   occurrence elimination under accepted statements — placement, the four
   conditions, the interval refusal with its OPEN trigger. **Alternative:**
   leave it to D2 skip-suffix dynamics. **Why it loses:** skip-suffix still
   pays per-binding probes and a larger DP, and is illegal under aggregate
   sinks, while elimination is sink-independent. **Reverses if:** measured
   plan-time cost of the fixpoint exceeds its execution savings on the ledger
   suite (implausible at ≤20 occurrences). `30-dependencies.md` gains one
   sentence: statements license planner rewrites (pointer to 40).

## Passing criteria

- `[test]` EXPLAIN golden on the DU fixture: the eliminated line with the
  rendered statement.
- `[test]` Coverage-contract assertions for the new shapes (eliminable +
  near-miss) pass at n=1000.
- `[test]` The dual-run differential unit tests (chase on/off/model) agree on
  every eliminable fixture, projection and aggregate sinks.
- `[shape]` Doc amendments landed (decision block + the one-sentence pointer).
- `[gate]` Workspace gates green. **Orchestrator gate (recorded here, executed
  outside PRD content): full `bumbledb-bench verify` green post-landing.**
