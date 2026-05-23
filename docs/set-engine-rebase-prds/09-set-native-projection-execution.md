# PRD 09: Set-Native Projection Execution

## 01. Status

Not started.

## 02. Severity

High performance and semantic architecture.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must complete correctness PRDs first.

The implementer must add counters before claiming reduced work.

The implementer must not rely on final result deduplication as the primary execution model.

The implementer must preserve exact result-set semantics.

## 04. Dependency Order

PRDs 01 through 04 are mandatory.

PRD 08 should be complete first for best field-scope behavior.

PRD 10 depends on the same early-event concepts but is separate.

PRD 11 depends on lessons from this PRD for payload demand.

PRD 16 depends on the counters introduced here.

## 05. Problem Statement

Projection execution still behaves like a full-binding pipeline.

The LFTJ executor binds every query variable.

Only at full depth does it emit to the projection sink.

The projection sink deduplicates encoded result facts.

The final `QueryResultSet` sorts and deduplicates again.

This produces correct results in many cases.

It is not a set-native execution model.

If projected variables are already bound and remaining variables are existential, the engine should not enumerate all existential bindings.

It should prove existence and emit the projected result fact once.

## 06. Code Map

Primary files:

- `crates/bumbledb-lmdb/src/query.rs`.
- `crates/bumbledb-lmdb/src/free_join.rs`.
- `crates/bumbledb-lmdb/src/query_image.rs` if relation access helpers are needed.
- Free Join access abstractions if semijoin probe behavior is extended.

Relevant current regions:

- `query.rs:5595-5622` for LFTJ full-depth emission.
- `query.rs:8217-8252` for projection sink dedup.
- `query.rs:231-236` for final result set sort/dedup.
- `free_join.rs:88-99` for payload demand fields.
- `query.rs:7488-7518` for payload demand construction.

## 07. Existing Behavior

Planner chooses variable order.

LFTJ recursively binds one variable per depth.

Predicates are checked when ready.

At full variable depth, the binding is complete.

The complete binding is passed to the sink.

Projection sink builds an encoded result fact from projected variables.

Projection sink inserts that fact into a set.

Duplicate projected facts are suppressed by the sink.

Final result set sorts and deduplicates again.

Counters record seen and inserted projected facts.

Duplicate counter exists but is not incremented.

## 08. Concrete Waste Case

Relations `Account(account)` and `Posting(posting, account, tag)`.

Query projects `account` where account has any posting with a tag.

An account can have thousands of postings.

The result set contains the account once.

Current execution can enumerate every matching posting binding.

Projection sink deduplicates thousands of identical account result facts.

Set-native execution should emit the account once after proving at least one posting exists.

The remaining posting variable is existential and should not multiply work.

## 09. Desired Semantics

Projection output is a set of result facts.

A projected result fact can be emitted when every projected variable is bound and every remaining clause can be proven existentially satisfied.

Remaining existential clauses must be checked as semijoins or existence proofs.

If existence proof fails, the projected result fact is not emitted.

If existence proof succeeds once, no further witnesses are needed for that projected fact.

Repeated projected facts must still be deduplicated across different paths.

All public result facts remain canonical and duplicate-free.

## 10. Research Context

Free Join generalizes iteration and probing.

Set projection should exploit that generality.

Once payload demand is satisfied, remaining relations often serve only as filters.

Traditional full conjunctive-query evaluation produces every full binding.

Bumbledb usually needs projected result sets, not every binding.

This PRD begins moving from witness completion to result-set construction.

It is a necessary precursor to factorized and vectorized execution.

## 11. Definitions

Projected variables are variables included in `ProjectPlan.vars`.

Payload-satisfied depth is the earliest depth where all projected variables are bound.

Existential suffix is the remaining query work after payload-satisfied depth.

Semijoin proof is a check that at least one extension exists for the remaining constraints.

Early projection emission is emitting a result fact before all query variables are bound.

Duplicate projected result is an encoded result fact already emitted for the query.

## 12. Invariants

Early emission must never emit a result fact that lacks a valid complete extension.

Early emission must never omit a result fact that has at least one valid complete extension.

Early emission must preserve exact projection set semantics.

Projected variables must be bound before emission.

Predicates involving unbound existential variables must be included in the semijoin proof.

Repeated-variable constraints must be included in the semijoin proof.

Input and literal constraints must be included in the semijoin proof.

Duplicate suppression remains required across different projected paths.

## 13. Implementation Plan

Add projection-aware execution metadata.

Compute the set of projected variable IDs.

Compute earliest projected depth from variable order.

At or after that depth, determine whether remaining work can be treated as existential.

Add a semijoin proof helper for remaining atoms and predicates.

If proof succeeds, emit projected fact and stop exploring deeper bindings for that projected fact.

If proof fails, continue leapfrog seeking as needed.

Keep full-depth execution as fallback when proof cannot be built.

Add counters for early projection attempts, successes, failures, and fallback completions.

## 14. Semijoin Proof Requirements

The proof must know current bound variables.

The proof must inspect remaining atoms.

The proof must respect already-bound variables.

The proof must respect literals and inputs.

The proof must respect predicates whose operands can be evaluated.

The proof must recursively prove existence when one remaining atom alone is insufficient.

Initial implementation may support a conservative subset.

If proof cannot prove existence cheaply, it must fall back to normal execution.

It must never guess success.

It must never guess failure.

## 15. Projection Sink Changes

Add a method that emits projected encoded facts from partial bindings.

Do not require non-projected variables to be bound for projection emission.

Increment `encoded_project_facts_seen` for every projected fact considered.

Increment `encoded_project_facts_inserted` only for new projected facts.

Add a real duplicate-projection counter only if it is incremented by the set insertion path.

Do not reintroduce a dead duplicate counter that always reports zero.

Avoid final dedup hiding sink errors.

Final `QueryResultSet::new` may retain sort for canonical ordering.

Final `QueryResultSet::new` should not be the primary duplicate-defense for internal sinks.

## 16. Planner Changes

Add explicit projected-variable demand metadata for projection queries.

Mark relations that are existence-only after projected vars are bound.

Expose projected depth in plan diagnostics.

Expose whether early projection is enabled.

Do not force early projection for all queries immediately.

Start with safe shapes.

Expand shapes only with tests.

## 17. Required Tests

Add duplicate-witness projection fixture.

Assert output result set is correct.

Assert completed bindings decrease compared with forced full-depth execution if a test hook exists.

Assert early projection success counter increments.

Assert duplicate projected fact accounting works if a real duplicate counter is added.

Add a case where existential proof fails and no result is emitted.

Add a case where proof cannot be built and executor falls back safely.

Add a case with predicate on existential variable.

Add a case with input-backed existential filter.

Add a case with repeated variable constraint.

## 18. Required Golden Tests

Ledger projection with duplicate existential witnesses remains correct.

Sailors projection with duplicate witness paths remains correct.

Joinstress projection remains correct.

LDBC two-hop projection remains correct.

Any golden example with projection duplicates must assert counters if deterministic.

## 19. Required Diagnostics

Add `projection_early_attempts`.

Add `projection_early_successes`.

Add `projection_early_failures`.

Add `projection_early_fallbacks`.

Add `projection_semijoin_probes`.

Keep `bindings_completed` for comparison.

Keep `encoded_project_facts_seen`.

Keep `encoded_project_facts_inserted`.

Do not add duplicate counters unless execution increments them.

## 20. Benchmark Requirements

Add a focused projection benchmark with high duplicate existential witness count.

Benchmark must validate exact result facts first.

Benchmark must report completed bindings.

Benchmark must report projected facts inserted.

Benchmark must report early projection successes.

The benchmark should fail a focused gate if witness work regresses above a documented threshold after this PRD.

## 21. Passing Criteria

Projection can emit before full binding for at least one tested safe shape.

Projection output remains exact and duplicate-free.

Existential suffix filters are respected.

Counters prove reduced completed bindings on a duplicate-witness fixture.

Duplicate projection counter is correct or removed.

Final result dedup is not hiding internal duplicate production in normal paths.

The global validation gate passes.

The query-focused validation gate passes.

## 22. Failure Modes

Emitting projected facts without proving existential suffix existence is a failure.

Completing all bindings for the focused duplicate-witness fixture is a failure unless fallback is explicitly expected for that shape.

Dropping result facts because an existence proof is too aggressive is a failure.

Using final dedup as the only set mechanism is a failure.

Changing aggregate behavior is a failure.

Adding approximate semijoin proofs is a failure.

## 23. Non-Goals

Do not implement aggregate-domain early events.

Do not rewrite Free Join plan representation.

Do not implement COLT.

Do not implement vectorized batches.

Do not add approximate query processing.

Do not change public result types.

## 24. Completion Notes

Document which projection shapes support early emission.

Document fallback shapes.

Keep duplicate-witness tests permanent.

This PRD is the first major shift from full-binding execution to set-result execution.
