# PRD 12: Free Join Factoring

## 01. Status

Not started.

## 02. Severity

High performance and Free Join completeness.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must complete PRD 11 first.

The implementer must produce structural plan tests before executing factored plans.

The implementer must not make heuristic reorderings that change semantics.

The implementer must preserve deterministic plans.

## 04. Dependency Order

PRD 11 is mandatory.

PRD 09 and PRD 10 should be complete first if payload demand impacts factoring decisions.

PRD 13 depends on factored plan shapes for lazy trie build benefits.

PRD 14 depends on factored node batches for vectorized probes.

PRD 15 depends on factoring metrics for cost modeling.

## 05. Problem Statement

The current engine does not implement Free Join factoring.

Factoring is the pass that moves already-keyed probes earlier in a Free Join plan.

Without factoring, an acyclic query can expand a large intermediate binding set before a later selective probe removes it.

This repeats one of the classic performance failures of naive binary-style plans.

The Free Join paper's clover example demonstrates why factoring matters.

Bumbledb currently lacks the plan representation and pass to do this systematically.

PRD 11 supplies the representation.

This PRD supplies the factoring transformation.

## 06. Code Map

Primary files after PRD 11:

- `crates/bumbledb-lmdb/src/free_join.rs`.
- `crates/bumbledb-lmdb/src/query.rs`.
- `crates/bumbledb-lmdb/src/planner_stats.rs` if estimates are added.
- `crates/bumbledb-lmdb/src/query_tests.rs` for focused plan tests.

Expected new or changed areas:

- Free Join plan construction.
- Free Join plan validation.
- Optimizer trace construction.
- Plan counters and estimates.
- Explain output.

## 07. Research Context

Free Join starts from a plan that may resemble a binary plan.

It then factors lookups into earlier nodes when their required variables are already available.

This avoids expanding bindings that a later lookup would reject.

Factoring moves the plan toward a WCOJ-style intersection only where useful.

It does not require abandoning binary optimizer intuition.

It is one of the core paper contributions.

Bumbledb needs factoring to avoid being pure LFTJ on one side and separate direct kernels on the other.

## 08. Example Shape

Consider relations `R(x, a)`, `S(x, b)`, and `T(x, c)`.

Suppose a starting plan iterates `R`, probes `S`, expands `b`, then probes `T` by `x`.

The `T(x)` probe only needs `x`.

`x` is already available in the first node.

Factoring moves `T(x)` into the first node.

The engine can reject `x` values absent from `T` before expanding all `S` values.

This preserves output semantics.

This can reduce candidate bindings dramatically.

## 09. Desired Invariants

Factoring must preserve query semantics.

Factoring must preserve set result semantics.

Factoring must keep plans valid under PRD 11 validation.

Factoring must never make a probe require unavailable variables.

Factoring must never duplicate a subatom consumption.

Factoring must never place incompatible pieces of the same atom in the same invalid node.

Factoring must be deterministic.

Factoring must have visible diagnostics.

## 10. Starting Plan Requirement

Implement at least one source of factorable Free Join plans.

Preferred source: binary-style linear plan conversion.

Acceptable first source: existing variable-order plan converted into richer Free Join nodes where factoring can be demonstrated.

The source plan must be valid before factoring.

The factored plan must be valid after factoring.

The test suite must compare outputs before and after factoring.

## 11. Factoring Algorithm Requirements

Traverse plan nodes from later to earlier.

For each probe subatom in a later node, determine required variables.

Find the earliest prior node where all required variables are available.

Move the probe to that node if validation remains true.

Do not reorder probes in a way that violates explicit optimizer order unless a later cost PRD allows it.

Do not move a cover if doing so removes the only source of a node's new variables.

Do not move a subatom across a node that binds variables it requires unless those variables are already available earlier.

Repeat until fixed point or perform one deterministic reverse pass.

Document chosen strategy.

## 12. Conservative Rule Set

Only move probe-only subatoms in the first implementation.

Do not move selected covers in the first implementation.

Do not split a subatom further in the first implementation.

Do not combine two subatoms from the same atom in one node unless PRD 11 validation explicitly permits it.

Do not change output payload assignment in the first implementation.

Do not change variable order in the first implementation.

This conservative version is enough to capture the major early-probe win.

## 13. Plan Validation Requirements

Run validation before factoring.

Run validation after every move in debug mode or tests.

Run validation after final factoring in production code.

Reject or skip moves that make validation fail.

Expose skipped move reasons in debug trace if practical.

Do not produce invalid plans and rely on executor fallback.

## 14. Cost And Heuristic Requirements

This PRD does not require full cost-based factoring.

The initial rule may be deterministic and conservative.

However, do not move probes later.

Do not increase required materialization.

Do not duplicate probe work across sibling branches.

Record simple estimates for candidate reduction if available.

Full cover-cost optimization belongs to PRD 15.

## 15. Counters And Trace Requirements

Add `free_join_factoring_attempts` if counters are available.

Add `free_join_factoring_moves`.

Add `free_join_factoring_skipped_unavailable`.

Add `free_join_factoring_skipped_invalid`.

Add before/after node counts in optimizer trace.

Add before/after probe placement in explain output.

Keep trace deterministic.

## 16. Required Structural Tests

Clover-like plan moves a later `x` probe into the first node.

Chain plan with no movable probes remains unchanged.

Probe requiring a later variable is not moved.

Move that would create invalid same-atom placement is skipped.

Factoring fixed point is deterministic.

Before and after plans both validate.

Trace records the move count.

## 17. Required Execution Tests

Factored and unfactored plans produce identical result sets.

Focused skew fixture has fewer completed bindings after factoring.

Projection query remains correct.

Aggregate query remains correct if aggregates are enabled for factored plans.

Prepared query remains correct.

Cyclic query remains correct.

Acyclic query remains correct.

## 18. Required Benchmark Fixture

Add a small clover-like focused benchmark or test fixture.

The fixture must have skew that causes unfactored expansion.

The factored plan must reduce candidate bindings or completed bindings.

Exact result facts must be validated.

The benchmark should report factoring moves and binding reductions.

## 19. Interaction With Projection

Factoring should help projection semijoin checks.

Do not break early projection from PRD 09.

If early projection makes a factoring move unnecessary, trace should still be valid.

Projection output must remain identical.

Projected payload availability must not move later.

## 20. Interaction With Aggregates

Factoring should help aggregate suffix existence checks.

Do not break early aggregate events from PRD 10.

Aggregate domain events must remain identical.

Factoring must not change which domain keys exist.

Aggregate output must remain identical.

## 21. Passing Criteria

At least one tested plan is structurally changed by factoring.

Every factored plan validates.

Factored and unfactored execution produce identical result sets.

Focused skew fixture shows reduced witness work.

Optimizer trace reports factoring.

No invalid move is accepted.

The global validation gate passes.

The query-focused validation gate passes.

## 22. Failure Modes

Changing result sets is a failure.

Moving probes that need unavailable variables is a failure.

Changing variable order without explicit design is a failure.

Duplicating subatom consumption is a failure.

Factoring only in explain output but not execution is a failure.

Adding nondeterministic plan transforms is a failure.

## 23. Non-Goals

Do not implement COLT.

Do not implement vectorized execution.

Do not implement full cost-based cover selection.

Do not add external optimizer dependencies.

Do not change storage layout.

Do not add approximate heuristics that can change results.

## 24. Completion Notes

Document the exact conservative factoring rule.

Keep clover-like regression permanent.

Record any skipped factoring opportunities for PRD 15.

This PRD should make the first visible Free Join paper optimization real.
