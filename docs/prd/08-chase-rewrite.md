# PRD 08 — The chase: analysis and rewrite

**Depends on:** nothing in this set (reads the post-rebuild planner as-is).
**The only PRD pair with real regression risk — after 09 lands, the full
two-oracle verify runs green before anything stacks on top.**
**Modules:** new `crates/bumbledb/src/plan/chase.rs`, wiring in
`crates/bumbledb/src/plan/planner.rs` (pre-DP), reading
`crates/bumbledb/src/ir/normalize/` (occurrence table, filters, residuals,
anti-probes) and `crates/bumbledb/src/schema.rs` (statements, `Resolved`).
**Authority:** `30-dependencies.md` (statements, the acceptance gate — key-ness
of Y is what makes this sound), `40-execution.md` (planner placement),
`20-query-ir.md` (set/aggregation semantics the proof leans on).

## Context

Containment-implied inner-join elimination — the rewrite Postgres rejected for
decades because deferred constraints make FKs untrustworthy at plan time. This
engine deleted the objection: no deferral modes exist and every readable
snapshot satisfies every statement, so the rewrite is *always* sound here when
its conditions hold. Classical name: the chase. This PRD builds the analysis and
the rewrite; PRD 09 builds the surfaces (EXPLAIN, generator coverage,
differential switch).

## The rewrite (conditions are normative — implement exactly)

A **positive** atom occurrence `B` is removable when:

1. An accepted containment `A(X | φ) <= B(Y | ψ)` exists (either direction of an
   `==` pair is its own statement already), and the query joins `A` to `B`
   exactly on X→Y: every variable shared between the two occurrences pairs a
   source-projection position of `A` with its `key_permutation`-corresponding
   target position of `B`, and every X→Y position pair is join-covered
   (partial-key joins do not qualify — uniqueness needs the whole key).
2. `B` is otherwise unused: no `B` field outside Y is projected, filtered,
   compared in residuals, **or referenced by any other occurrence — positive or
   negated, including anti-probe bindings and membership points**; `B` carries
   no selections beyond ψ (literal subset of ψ is fine — the containment's
   match satisfies all of ψ); and the `A` occurrence's own filter list contains
   φ — **literal-subset checked as (field, encoded literal) set containment,
   never inferred**. If φ is not literally present on `A`, no elimination.
3. Every variable of `B` is either a join variable (unified with `A`'s) or dead
   in the sense of condition 2.
4. **Interval refusal (v0):** no paired position is interval-typed — pointwise
   coverage is not 1:1 fact-to-fact. Refuse and move on; record the OPEN
   sub-question in the doc amendment ("trigger: a census query that would
   benefit").

**Why it is sound (carry this into the module doc):** existence — the
containment guarantees each surviving `A` binding a ψ-satisfying `B` match;
uniqueness — the acceptance gate requires Y to be a key of `B`, so the match is
exactly one; aggregate safety — key-ness makes every non-Y field of the match
functionally determined, so a variable bound only on `B`'s non-key fields takes
exactly one value per binding and cannot multiply the fold domain. Removal is
therefore bit-identical under both sinks.

## Technical direction

1. **Placement:** after normalization (occurrences/filters/residuals explicit),
   before statistics and the DP — a `chase(&mut normalized, &schema)` pass in
   `plan/chase.rs` called from the planner entry. It must run **as a fixpoint**:
   removing one occurrence can make another removable (chains `A<=B<=C`); loop
   until no removal. ≤20 occurrences makes the loop trivially cheap.
2. **Removal mechanics:** dropping occurrence `B` re-indexes the occurrence
   table and every structure that references occurrences by index (filters,
   residual sides, anti-probe specs, find-variable sources). Prefer a single
   `remove_occurrence(idx)` function owning ALL the re-indexing, exhaustively —
   this is where a subtle bug would live; make it one auditable place with a
   test that round-trips a synthetic normalized query through removal and
   checks every index.
3. **Record what was done:** the normalized output carries
   `eliminated: Vec<(OccurrenceId, StatementId)>` — dead data for execution,
   consumed by PRD 09's EXPLAIN surface and by tests.
4. **The proof-shaped precedent:** model the condition-checking code on
   `plan/provably_distinct.rs` (same move: plan-time proof from schema
   statements). Read it first; match its style.
5. **Test-only off switch:** a `#[cfg(test)]`-visible constructor knob or a
   crate-internal `fn chase_disabled_for_tests()` path so differential tests
   (PRD 09) can run the same query with and without the rewrite. **No runtime
   mode ships** — no public API, no env var, no feature flag.

## Passing criteria

- `[shape]` Conditions 1–4 implemented as separate, named predicate functions
  (auditable one-to-one against this PRD); `remove_occurrence` is the single
  re-indexing site; no public off switch exists.
- `[test]` Positive cases: the existence-walk shape (join parent only to prove
  the reference) eliminates; the DU one-sided `==` query eliminates the header;
  a chain `A<=B<=C` eliminates both in fixpoint. Assert `eliminated` contents
  and that the DP saw the reduced occurrence count.
- `[test]` Negative cases, one per condition: partial-key join; a projected
  non-Y field; a negated atom referencing `B`; a membership point sourced from
  `B`; missing φ on the `A` side; extra selection on `B` beyond ψ; an
  interval-typed pair. Each refuses (assert not eliminated).
- `[test]` The removal re-indexing round-trip test.
- `[test]` Result equality on a fixture: eliminated vs chase-disabled execution
  produce identical result sets, projection and aggregate sinks both (this is
  the unit-level version of PRD 09's randomized differential).
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

Deferred to PRD 09 (one amendment covering pass + surfaces together).
