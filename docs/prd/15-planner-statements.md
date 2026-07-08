# PRD 15 — Planner over statements

**Depends on:** 03, 13, 14.
**Modules:** `crates/bumbledb/src/plan/{planner.rs,fj.rs,selectivity.rs}` and the plan witness.
**Authority:** `docs/architecture/40-execution.md` (§ planner, § inputs from normalization), `20-query-ir.md` (§ prepared queries — param-set assumption).

## Goal

The planner consumes schema *statements* instead of constraints, plans around
negated occurrences and param sets, and the plan witness carries anti-probe
attachment and the two-slot interval layout.

## Technical direction

1. **Statement-driven knowledge:** everywhere the planner asked "do these bound
   fields cover a unique constraint" it now asks "cover a `Functionality`
   statement's projection" (the per-relation `keys` index, PRD 02). The join
   cardinality estimator's two key rules and the elision proof both switch to this
   source — mechanical rename plus the index change; semantics identical for
   scalar keys. A **pointwise key does not certify fanout 1** for a join binding
   only its scalar prefix (two facts may share the prefix with disjoint
   intervals) — key coverage requires the full projection including the interval
   field bound by value; add this guard and a comment.
2. **Negated occurrences:** excluded from the DP state entirely (positive
   occurrences only). After node construction, attach each anti-probe descriptor
   (PRD 13) to the earliest node at which its variable set is bound — the same
   attachment computation residual comparisons use; extend that function to
   handle both and store per-node `anti_probes` lists in the witness. Negated
   occurrences still get trie schemas (their probe key order = the order their
   variables appear in the probing node's binding — derive per §3.3 exactly as
   positive occurrences do) and count against the validation-boundary occurrence
   cap (already enforced, PRD 12).
3. **Param sets:** a set-bound selection plans on the documented small-set
   assumption — add one documented constant `PARAM_SET_PLANNING_CARDINALITY`
   (16; a floor-style constant like the selectivity ladder's, documented at its
   definition citing `20-query-ir.md`) and use it as the assumed distinct-match
   count in the selectivity ladder where a measured survivor count is impossible
   (params are unmeasurable at prepare — the existing carve-out extends).
4. **Interval variables in plans:** a variable of interval type binding two atoms
   (value-equality join) is plannable as an ordinary join variable whose COLT
   level key is the **two-word pair** — trie schemas treat it as one variable
   with a two-word key (the wordmap already keys tuples; extend key width
   bookkeeping). Membership-bound fields are never join keys (validation
   guarantees the point var has a scalar anchor; the interval field itself
   contributes only filters).
5. **Witness additions:** per-node `anti_probes`; `ColumnSpan`-based field maps
   (PRD 14); the binding-slot layout with two-slot interval vars (PRD 13);
   negated-occurrence trie schemas. Validated once at construction, per the
   existing witness discipline.

## Out of scope

Executor changes (16–18).

## Passing criteria

- `[shape]` `rg -i 'unique|constraint' crates/bumbledb/src/plan/` returns no
  identifier hits.
- `[shape]` `PARAM_SET_PLANNING_CARDINALITY` exists once, documented, cited.
- `[test]` Estimator: a join binding a pointwise key's scalar prefix only does
  NOT get fanout-1 (assert the estimate is the general case); binding the full
  projection including the interval by value does.
- `[test]` Attachment: a negated atom over variables bound at node 2 of a
  three-node plan lands in node 2's `anti_probes`; one over root-only variables
  lands at the root.
- `[test]` Plan construction for the `20-query-ir.md` outer-join-idiom pair and
  an `Overlaps` residual query both validate into witnesses (golden node/residual
  shapes).
