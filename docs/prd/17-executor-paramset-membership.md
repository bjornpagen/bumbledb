# PRD 17 — Executor: param sets and membership

**Depends on:** 15; 14 (filter evaluator scalar path).
**Modules:** `crates/bumbledb/src/exec/colt/` (selection levels), `crates/bumbledb/src/exec/kernel/`, `crates/bumbledb/src/image/view/`, `crates/bumbledb/src/api/prepared/` (bind path).
**Authority:** `docs/architecture/40-execution.md` (§ perf-suite mechanisms — selection levels bullet; § access paths — interval lowering), `20-query-ir.md` (§ param sets).

## Goal

Set-bound selections probe k times and union survivors; membership and interval
predicates evaluate as two-word filters with NEON parity to the existing predicate
scans.

## Technical direction

1. **Bind path:** execution bind accepts per-param scalars or slices (typed check
   per the anchored type; PRD 20 owns the public signature — here, the internal
   resolved-params representation gains a set variant holding a **sorted,
   deduplicated** word list in pooled storage; string/bytes elements resolve to
   intern ids per element with per-element miss sentinels, `20-query-ir.md`).
   Empty set ⇒ the `Eq`-miss short-circuit path (query result empty where sound —
   the same machinery as the never-interned literal).
2. **Selection levels** (`Colt::select`): a set-bound selection probes the
   prepended trie level once per element and unions the survivor position lists
   (concatenate + the level's own dedup guarantees disjointness — positions under
   distinct keys are disjoint by construction; assert, don't dedup). The union
   feeds the node exactly as a single-word selection's survivors do. Never
   re-execute the query per element.
3. **Membership/interval filters** (from PRD 13: `PointIn`, `AnyPointIn`, the
   three Overlaps/Contains shapes): implement the scalar evaluator in the view
   filter machinery (if PRD 14 left it incomplete) and add NEON variants under
   the sanctioned predicate-scan shape: each is 1–2 fixed-width two-column
   compare-and-mask passes (`start ≤ p` mask AND `p < end` mask), which is the
   existing predicate-scan kernel applied twice with an AND — compose existing
   kernels; do not add a new kernel *shape* (the architecture says none exists).
   `AnyPointIn` = OR over the set's per-point masks, k small; scalar fallback
   mandatory, bit-identity property tests per the unsafe policy.
4. **Residual interval comparisons** (cross-atom, over two-slot vars): the batch
   residual evaluator gains the three fixed word-comparison compositions reading
   slot pairs; branchless compaction as for existing residuals.
5. **Anti-probes with sets** (negated atoms carrying ParamSet bindings): the
   anti-probe's filter list handles set membership per element — a binding is
   rejected if the negated occurrence matches under **any** element (the
   existential reading; one comment citing `20-query-ir.md`'s "any element"
   sentence).

## Out of scope

Planner constants (15, done); sink changes (18).

## Passing criteria

- `[shape]` No new NEON kernel shape: interval filters are compositions of the
  existing predicate-scan primitives (reviewable by kernel module diff).
- `[shape]` Set probes are k level-probes + survivor union; `rg` finds no
  per-element query re-execution.
- `[test]` IN family correctness: set of {0, 1, 2, 200} elements over a selective
  column — results equal the union of per-element scalar-param executions
  (assert against that construction); duplicates in the bound slice collapse;
  out-of-vocabulary string elements contribute nothing.
- `[test]` Membership: point-var join (`Payroll(emp, during ∋ t), Event(emp, at=t)`
  shaped fixture) returns exactly the rows whose event time falls in the payroll
  interval, boundaries asserted both ends.
- `[test]` Overlaps residual: two relations' intervals, all 13 Allen configurations
  constructed pairwise, `Overlaps` keeps exactly the 9 intersecting ones;
  `Contains` keeps exactly the containment configurations.
- `[test]` NEON/scalar bit-identity property tests for each new filter
  composition across boundary shapes (empty, single, odd lengths, lane ±1).
