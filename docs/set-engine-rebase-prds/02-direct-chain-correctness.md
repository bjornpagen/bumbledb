# PRD 02: Direct Chain Correctness

## 01. Status

Not started.

## 02. Severity

Critical correctness.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must write regression tests before changing execution logic.

The implementer must not broaden direct-chain eligibility while fixing this bug.

The implementer must not weaken LFTJ fallback behavior.

The implementer must preserve exact result-set equality with the generic query path.

## 04. Dependency Order

PRD 01 should be complete first.

This PRD can be developed in parallel only if PRD 01 cache tests are unaffected.

PRD 09 depends on direct-chain correctness because projection short-circuiting must not inherit broken direct paths.

PRD 11 depends on this PRD because direct kernels may later become Free Join node implementations.

PRD 15 depends on this PRD because direct kernels must not enter the optimizer as unsafe candidates.

## 05. Problem Statement

The direct chain executor can evaluate existence checks before required variables are bound.

The planner decides that an atom is fully checkable after certain prior steps.

The executor currently evaluates all existence checks before any step recursion.

This can request a bound variable that does not exist yet.

This can produce an internal error.

This can also cause an incorrect empty result if an unsafe fallback path treats the missing value as failed existence.

This is a correctness bug.

Direct chain is a specialized path and must be stricter than the generic path, not looser.

## 06. Code Map

Primary file: `crates/bumbledb-lmdb/src/query.rs`.

Planner region: `try_direct_chain_kernel`.

Executor region: `DirectChainExecutor`.

Prefix construction region: `direct_prefix`.

Storage fallback region: `storage_facts_for_terms`.

Image fallback region: `image_facts_for_terms`.

Relevant current regions:

- `query.rs:4511-4529` creates existence checks.
- `query.rs:4552-4561` creates chain steps and advances planned bound vars.
- `query.rs:4711-4782` runs every existence check before recursion.
- `query.rs:4945-4953` binds variables inside step recursion.
- `query.rs:5202-5222` errors when a direct prefix references an unbound variable.

## 07. Existing Behavior

The direct-chain planner walks relation atoms in order.

It tracks a planned set of bound variables.

If an atom has no unbound variables, it becomes an existence check.

If an atom has exactly one unbound variable, it becomes a chain step.

If an atom has more than one unbound variable, the direct path is rejected.

After a chain step is created, the planner marks the new variable as bound.

This planned bound set is correct as a planning abstraction.

The executor does not honor the planned timing.

The executor runs all existence checks before step zero.

Therefore checks that require step-bound variables run too early.

## 08. Concrete Failure Case

Create relations `A(a)`, `B(a, b)`, and `C(b)`.

Query finds `a` where `A(a)`, `B(a, b)`, and `C(b)`.

The first atom starts with `a`.

The second atom binds `b` from `a`.

The third atom is an existence check over `b`.

The planner can classify `C(b)` as checkable after `B`.

The executor currently runs `C(b)` before `B` binds `b`.

The prefix for `C` cannot be constructed.

The correct execution should bind `b`, then check `C(b)`.

If `C(b)` is absent, the partial binding should be discarded.

If `C(b)` is present, the result should be emitted.

## 09. Desired Invariants

Every direct-chain prefix term must be available before it is read.

Inputs are available at depth zero.

Literals are available at depth zero.

Variables from the initial binding source are available at depth zero only if the executor has actually bound them.

Variables introduced by step `n` are available only after step `n` succeeds.

An existence check may run at the earliest depth where all its terms are available.

An existence check must never run earlier than that depth.

A direct-chain plan with impossible timing must not be selected.

Direct-chain output must equal LFTJ output for the same query.

## 10. Research Context

Free Join decomposes execution into iteration and lookup operations.

Every lookup key must be composed from values already bound by the current node or prior nodes.

The direct chain path is a restricted iteration and lookup plan.

It must obey the same availability discipline as Free Join.

In the Free Join paper, invalid plans are rejected when a lookup needs unavailable variables.

Bumbledb must do the same for direct-chain plans.

This PRD is a stepping stone toward making direct kernels real Free Join nodes.

## 11. Data Model Definitions

Depth zero is the state before executing the first chain step.

Step depth `n` is the state after executing `n` successful steps.

A depth-0 check can use only literals, inputs, and variables bound before step recursion begins.

A post-step check can use variables bound by earlier successful steps.

The assigned check depth is the maximum availability depth of all terms in the check.

An unsafe check is any check whose required term depth is unknown.

Unsafe checks must reject direct-chain planning.

## 12. Implementation Plan

Add `available_depth` to `DirectExistenceCheck`.

Add a planner-side map from variable ID to availability depth.

Initialize the map with variables truly available before recursion.

When creating a chain step, set the new variable's availability depth to `steps.len() + 1`.

When creating an existence check, compute required depth from its terms.

Inputs contribute depth zero.

Literals contribute depth zero.

Variables contribute their recorded availability depth.

Unknown variables reject direct-chain planning.

Wildcards must not become prefix terms.

The check's depth is the maximum of term depths.

Store checks grouped by depth.

Run depth-zero checks in `execute` before `execute_step(0)`.

Run checks for depth `n + 1` immediately after a step at depth `n` binds its variable.

Only recurse when those checks pass.

Unbind variables correctly after check failure.

## 13. Minimal Safe Alternative

If depth-aware checks are too invasive, reject any direct-chain plan that would require a non-zero-depth existence check.

This means direct-chain planner returns `None` for those shapes.

The generic LFTJ path then handles the query.

This is acceptable only as a temporary correctness-preserving outcome for this PRD.

If this alternative is chosen, document it in comments and tests.

The later PRD 11 or PRD 15 must remove the limitation.

## 14. Required Planner Validation

Add a validation helper for `DirectChainProbePlan`.

The helper must verify every check depth is less than or equal to the number of steps.

The helper must verify every variable term in every check has availability depth less than or equal to the check depth.

The helper must verify every step prefix term is available at the step depth.

The helper must verify the bind variable of a step is not already bound by the step prefix.

The helper must verify no wildcard is used as a direct prefix term.

Use this helper in tests.

Use this helper before returning a direct chain plan in debug builds or always if cheap.

## 15. Required Executor Changes

Do not loop over all existence checks in `execute`.

Replace that loop with `run_checks_at_depth(0)`.

Add `run_checks_at_depth(depth)`.

Call `run_checks_at_depth(depth + 1)` after a step binds its variable and before recursion.

Ensure storage-backed checks use current binding state.

Ensure image-backed checks use current binding state.

Ensure hash-trie checks use current binding state.

Ensure failed checks return to the caller without leaking bound variables.

Ensure counters still increment exactly once per attempted check.

## 16. Required Tests

Add a test for a post-step existence check that succeeds.

Add a test for a post-step existence check that fails.

Add a test for a depth-zero existence check using only inputs or literals.

Add a test for a multi-step chain where the final check depends on the second step.

Add a test that direct-chain results match LFTJ/reference for the same fixture.

Add a test that an unsafe check shape falls back if the minimal safe alternative is chosen.

Add a test for storage-backed direct check if an image access path is unavailable.

Add a test for image-backed direct check when an image access path is available.

Add a test that no direct-chain execution returns an internal missing-bound-variable error.

## 17. Test Data Requirements

Use small deterministic relation facts.

Include at least one branch that should be filtered by the existence check.

Include at least one branch that should survive the existence check.

Use facts that make the incorrect early check observable.

Use projection output so final result equality is easy to assert.

Use a reference evaluator or LFTJ comparison for confidence.

Do not rely on optimizer choosing a direct path unless the test can assert the runtime kind.

If needed, add a narrow test-only forcing hook for direct-chain planning.

## 18. Diagnostics Requirements

Preserve `direct_kernel_probes`.

Preserve `direct_kernel_facts`.

Preserve `direct_kernel_predicates`.

Preserve `direct_bind_attempts`.

Preserve `direct_bind_successes`.

Preserve `direct_chain_steps`.

Add check-depth diagnostics only if useful and deterministic.

If added, expose depth-zero checks and post-step checks separately.

Do not add per-fact logs.

## 19. Passing Criteria

No direct-chain path can request an unbound variable.

The post-step success test passes.

The post-step failure test passes.

Depth-zero checks still execute early.

Unsafe shapes either execute correctly or are not selected.

Direct-chain output equals LFTJ/reference output on regression fixtures.

The global validation gate passes.

The query-focused validation gate passes.

## 20. Failure Modes

Running every check after every step is a failure if it duplicates side effects or counters.

Running all checks only at the end is a performance regression and not the intended fix.

Forgetting to unbind after a failed post-step check is a correctness bug.

Treating a wildcard as an available term is a bug.

Treating a planned variable as runtime-bound before the step executes is the original bug.

Changing query result ordering as a side effect is unacceptable.

Deleting direct-chain tests instead of fixing the path is unacceptable.

Adding a public option to disable direct chains is unacceptable.

## 21. Non-Goals

Do not improve direct-chain cost estimates.

Do not add new direct-chain shapes.

Do not rewrite direct kernels into Free Join nodes.

Do not optimize memory allocations.

Do not implement vectorized direct execution.

Do not change aggregate execution.

Do not change storage layout.

## 22. Completion Notes

Document whether the full depth-aware implementation or minimal safe alternative was chosen.

If the minimal safe alternative was chosen, add an explicit follow-up note in PRD 11 or PRD 15.

Keep regression tests permanent.

Direct-chain correctness is required before direct kernels can become optimizer candidates.
