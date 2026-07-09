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
the rewrite; PRD 09 builds the surfaces.

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
   no selections beyond ψ (literal subset of ψ is fine); and the `A`
   occurrence's own filter list contains φ — **literal-subset checked as
   (field, encoded literal) set containment, never inferred**.
3. Every variable of `B` is either a join variable (unified with `A`'s) or dead
   in the sense of condition 2.
4. **Interval refusal (v0):** no paired position is interval-typed — pointwise
   coverage is not 1:1 fact-to-fact. Refuse; the OPEN sub-question rides the
   doc amendment ("trigger: a census query that would benefit").

**Why it is sound (carry this into the module doc):** existence — the
containment guarantees each surviving `A` binding a ψ-satisfying `B` match;
uniqueness — the acceptance gate requires Y to be a key of `B`, so the match is
exactly one; aggregate safety — key-ness makes every non-Y field of the match
functionally determined, so a variable bound only on `B`'s non-key fields takes
exactly one value per binding and cannot multiply the fold domain. Removal is
therefore bit-identical under both sinks.

## Technical direction

1. **Placement:** after normalization, before statistics and the DP — a
   `chase(&mut normalized, &schema)` pass in `plan/chase.rs` called from the
   planner entry, run **as a fixpoint** (removing one occurrence can make
   another removable; chains `A<=B<=C` are real; ≤20 occurrences makes the loop
   trivially cheap).
2. **Elimination is a mark, not a removal.** Index-shifting removal
   (`remove_occurrence` with centralized re-indexing) was considered and
   **rejected**: dense indices that move on removal are the off-by-one
   coordinate system — the re-indexing bug class exists only because of that
   representation. Instead the occurrence entry gains
   `eliminated: Option<StatementId>` (`None` = live). Occurrence ids never
   move; every structure that references occurrences by index stays valid by
   construction. The DP already excludes negated occurrences via its
   occurrence-filtering boundary — eliminated occurrences ride the **same
   accessor** (one "participates in planning" predicate: positive ∧ not
   eliminated), so downstream sees a smaller problem through the mechanism it
   already trusts. Audit the witness-construction and stats paths for any
   iteration that does NOT go through that predicate and route it through
   (this audit replaces the rejected design's re-indexing round-trip test).
3. **`eliminated` doubles as the record:** PRD 09's EXPLAIN surface and the
   tests read the marks directly; no separate eliminated-list is kept.
4. **The proof-shaped precedent:** model the condition-checking code on
   `plan/provably_distinct.rs` (same move: plan-time proof from schema
   statements). Read it first; match its style. Conditions 1–4 as separate,
   named predicate functions, auditable one-to-one against this PRD.
5. **Test-only off switch:** a crate-internal, `#[cfg(test)]`-reachable bypass
   so differential tests (PRD 09) run the same query with and without the
   rewrite. **No runtime mode ships** — no public API, no env var, no feature
   flag.

## Passing criteria

- `[shape]` Conditions 1–4 as named predicates; `eliminated` is an
  `Option<StatementId>` on the occurrence entry; exactly one
  participates-in-planning predicate exists and planner/stats/witness paths
  consume it (grep the iteration sites); no index re-mapping code exists; no
  public off switch.
- `[test]` Positive cases: the existence-walk shape eliminates; the DU
  one-sided `==` query eliminates the header; a chain `A<=B<=C` eliminates
  both in fixpoint. Assert the marks and that the DP saw the reduced count.
- `[test]` Negative cases, one per condition: partial-key join; a projected
  non-Y field; a negated atom referencing `B`; a membership point sourced from
  `B`; missing φ on the `A` side; extra selection on `B` beyond ψ; an
  interval-typed pair. Each refuses.
- `[test]` Result equality on a fixture: eliminated vs chase-disabled execution
  produce identical result sets, projection and aggregate sinks both.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

Deferred to PRD 09 (one amendment covering pass + surfaces together).
