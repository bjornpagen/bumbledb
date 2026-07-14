# PRD 21 — Cookbook epistemics: every recipe wears its label

**Depends on:** 11 (tiling corrected), 12 (`==` theorem stated), 20
(maintenance recipe exists) — this PRD labels the FINAL corpus.
**Modules:** `docs/cookbook.md` (every recipe), `crates/bumbledb-query/
tests/cookbook.rs` (negative witnesses where missing), `docs/formal/`
cross-references.
**Authority:** brief B8, approved: the cookbook is the persuasion
surface — its claims must be exactly as strong as their proofs. The
audit's label discipline (Lean theorem / theorem + validator premise /
host discipline / intentionally refused / documentation error) becomes
a visible, verifiable property of every recipe.
**Representation move:** none. Claims get provenance; over-claims get
corrected; under-tested claims get their negative witness.

## Context (decided shape)

1. **The label line.** Every recipe gains one line under its heading:
   `Guarantee: <label> — <one clause naming the source>`. Examples:
   - "Lean theorem + validator premise — key-backed correspondence
     (`KeyBackedEquality.unique_target`); both projections must be
     declared keys" (discriminated-union recipes);
   - "validator premise — per-group disjointness (pointwise key)";
   - "host discipline — freshness under a generation witness; the
     dependency proves soundness only" (derived-facts recipes);
   - "theorem of the primitives — mutual coverage; see § exact
     partition" (recipe from PRD 11).
2. **The classification pass.** Every recipe (roster ~27 by now) is
   audited against its label: claims stronger than the label supports
   are REWRITTEN (each rewrite listed in this file's Results with
   before/after); claims the label supports but no test witnesses get
   their witness.
3. **Negative witnesses.** Each recipe whose guarantee has a failure
   mode gains one negative case in the compiled copy's test where
   missing: the union recipe's double-arm rejection, the optional-
   child's second-child rejection, the closure idiom's staleness note
   (host-discipline recipes get their failure documented, not tested —
   the label says why), the coverage recipes' gap rejection. Audit
   what exists first — many negatives already live in the schema
   reject suites; a pointer in the recipe satisfies the criterion
   (don't duplicate tests).
4. **The sync law extends:** the token-identity test now also asserts
   every recipe HAS a label line (mechanical check on the doc block).

## Technical direction

One pass over the corpus with the theorem↔evidence table (PRD 01) open
— labels cite table rows where one exists. The Results section carries
the full recipe × label × witness matrix. No recipe is deleted; no
claim is strengthened; the direction of every edit is downward-or-
equal in strength, cited.

## Passing criteria

- `[shape]` Every recipe carries the label line (the sync test
  enforces — it fails on a labelless recipe).
- `[shape]` The recipe × label × witness matrix complete in Results;
  every over-claim rewrite listed before/after.
- `[test]` Cookbook suite green; every new negative witness green;
  zero token-sync drift.
- `[gate]` Docs + tests only; full suite green; fingerprint pin
  untouched.

## Doc amendments (rule 6)

This PRD is its amendments.

## Results (2026-07-13)

Every final-corpus recipe has an immediate `Guarantee:` line. The sync test
walks numbered headings and fails unless the first nonblank line is that label,
in the same 1–28 order as the schema roster.

| recipe | epistemic label | negative/runtime witness |
|---:|---|---|
| 1 | formal predicate + validator/runtime premise | pointwise functionality matrices; checked `Interval`; `r01_duration_sum_round_trips` |
| 2 | Lean theorem + validator/runtime premises | `KeyBackedEquality.unique_target/unique_source`; equality reverse-key reject locks; DU macro runtime locks |
| 3 | definition + validator/runtime premises | `r03_a_second_optional_child_is_rejected`; absence query round-trip |
| 4 | host discipline + validator premises | schema validation; bounded-sum runtime locks; scale/currency remain explicitly host-owned |
| 5 | validator/runtime premises + host discipline | fixed-bytes round trips and determinant locks; external hashing/durability is not claimed |
| 6 | validator/runtime premise | closed-extension validation/member-set tests; `r06_vocabulary_handle_round_trips` |
| 7 | validator/runtime premise | ψ member-set tests; `r07_classification_round_trips` |
| 8 | validator/runtime premise | `r08_sub_vocabulary_violating_insert_aborts` |
| 9 | validator/runtime premise + host discipline | scalar functionality conflict locks; result ordering explicitly host-side |
| 10 | Lean theorem + validator/runtime premises for arms; host discipline for acyclicity | equality macro locks; edge containment validation; acyclicity failure documented, not engine-tested |
| 11 | validator/runtime premises | containment and composite-key judgment matrices; no transitive claim |
| 12 | definition + validator/runtime premises | functionality/containment matrices and recipe 3's second-child countercase |
| 13 | Lean theorem + validator/runtime premises for the shipped arm; host discipline for transitions | equality direction locks; transition-path failure is documented host policy |
| 14 | Lean theorem + validator/runtime premises | calendar schema/family tests, DU locks, pointwise and coverage judgment matrices, `r14_booking_probe_round_trips` |
| 15 | Lean theorem/countermodel + validator/runtime premise | `interior_gap_aborts`; recipe-26 matrix pins one-way overhang acceptance |
| 16 | Lean theorem/countermodel + validator/runtime premise | `r26_exact_partition_commit_matrix` pins one-way overhang acceptance; `interior_gap_aborts` pins gap failure |
| 17 | validator/runtime premises + host discipline | pointwise-key and coverage matrices; the recipe states full bracket coverage/proration as host duties |
| 18 | runtime query semantics | `r18_pack_round_trips` plus Pack overlap/adjacency/ray suites; no stored invariant claimed |
| 19 | Lean theorem + runtime invariant for sums; host discipline for double entry | `r19_balances_round_trips`, checked-sum overflow locks; double entry remains a host assertion |
| 20 | generation-witness/runtime premise + host retry discipline | update-where, insert-select, snapshot-RMW movement locks; final-state point-read integration suite |
| 21 | Lean theorem + validator/runtime premises for soundness; host discipline for completeness | `stale_derived_fact_is_rejected_after_source_movement`; omissions explicitly remain representable |
| 22 | Lean theorem + represented planner/runtime premise | `r22_union_read_round_trips`; `r22_a_double_arm_payment_is_rejected`; disjoint-union introspection locks |
| 23 | intentionally refused | each gravestone points to its compiled replacement; no unsupported failure is presented as an engine guarantee |
| 24 | host discipline | `r24_closure_idiom_reaches_the_exact_set`; cross-snapshot staleness is documented rather than misclassified as an engine failure |
| 25 | host discipline + runtime aggregate semantics | `r25_subtree_rollup_matches_the_hand_computed_sum`; recursion remains outside the engine |
| 26 | Lean theorem + validator/runtime premises | `r26_exact_partition_commit_matrix`: exact/adjacent acceptance, forward gap, reverse overhang, one-way contrast, composite prefix |
| 27 | host discipline + validator/runtime premises | `r27_maintenance_rederives_after_generation_movement`; stale-derived soundness rejection lock |
| 28 | validator/runtime premises + host discipline | `r28_migration_is_etl`: fingerprint refusal, snapshot-consistent export, identity-preserving load, fresh-id catch-up, and judgment under the target theory |

Over-claim rewrites (before → after):

- Recipe 20: “the three witness idioms” → “three write idioms”; only the
  snapshot-derived update-where/insert-select paths are generation-witnessed,
  while `WriteTx` point reads use the final-state class.
- Recipe 21: “staleness the schema can name is uncommittable” → “unsoundness
  the schema can name is uncommittable; incompleteness remains representable.”
- Recipe 25: recursion “solved on the current engine” → “handled by explicit
  host composition over the current query engine.”

No other claim exceeded its new label. Host-discipline failures are named as
documentation limits rather than fabricated engine tests; existing general
validator/runtime matrices are cited instead of duplicated.
