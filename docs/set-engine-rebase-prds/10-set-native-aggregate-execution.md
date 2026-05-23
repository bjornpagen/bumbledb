# PRD 10: Set-Native Aggregate Execution

## 01. Status

Not started.

## 02. Severity

High performance and aggregate architecture.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must complete PRD 03 before beginning.

The implementer must complete PRD 04 before trusting tests and benchmarks.

The implementer must not change aggregate semantics to fit implementation convenience.

The implementer must add counters proving domain-level execution.

## 04. Dependency Order

PRD 03 is mandatory.

PRD 04 is mandatory.

PRD 09 should be complete first because early projection and early aggregate events share execution concepts.

PRD 11 can proceed after this PRD or in parallel if plan representation changes do not touch aggregate sinks.

PRD 15 depends on this PRD for real aggregate pushdown candidates.

## 05. Problem Statement

Aggregate execution currently waits for complete bindings.

Only after full binding does the aggregate sink build group keys and domain keys.

The sink maintains `seen_domains` sets to suppress duplicate domain events.

This is correct only if PRD 03 domain validation is fixed.

Even then, it does unnecessary work.

If an aggregate domain is determined earlier than full binding, the engine should not enumerate irrelevant existential suffixes.

Aggregate execution should operate on explicit domain events.

It should not be a full-binding cleanup operation.

## 06. Code Map

Primary files:

- `crates/bumbledb-lmdb/src/query.rs`.
- `crates/bumbledb-lmdb/src/free_join.rs`.
- `crates/bumbledb-lmdb/src/query_access.rs` if semijoin helpers are extended.
- `crates/bumbledb-test-support/src/reference.rs` for validation expectations.

Relevant current regions:

- `query.rs:8370-8418` for aggregate group/domain sink state.
- `query.rs:8431-8475` for aggregate finish.
- `query.rs:8531-8656` for aggregate state application and finish.
- `query.rs:7207-7217` for nominal aggregate pushdown candidate.
- `query.rs:7308-7312` for candidate ranking.
- `query.rs:7404-7445` for estimates that do not implement real aggregate pushdown.

## 07. Existing Behavior

Planner builds output plan with aggregate terms.

LFTJ binds every variable in variable order.

At full depth, aggregate sink receives complete binding.

Aggregate sink computes group key from group variables.

Aggregate sink computes domain key from aggregate domain variables.

Aggregate sink checks whether the domain key was already seen for the group and aggregate ordinal.

If not seen, aggregate state applies the measured value.

This means duplicate existential witnesses are deduped late.

This means irrelevant suffix variables can still be fully enumerated.

## 08. Concrete Waste Case

Relation `Posting(posting, account, amount)` has unique `posting`.

Relation `Tag(posting, tag)` contains many tags per posting.

Query groups by `account` and sums `amount` over domain `[posting]` while requiring at least one tag.

Each posting can have many tags.

The aggregate should apply `amount` once per posting.

Current execution can enumerate every tag witness.

The sink sees the same posting domain repeatedly and suppresses duplicates.

Set-native execution should prove tag existence and apply posting once.

## 09. Desired Semantics

Aggregates operate over explicit domain sets.

Each group/domain/aggregate ordinal combination contributes at most once.

`count_domain` counts domain keys.

`count_distinct` counts distinct measured values.

`sum` applies one measured value per domain key.

`min` sees one measured value per domain key.

`max` sees one measured value per domain key.

Existential suffixes should be semijoin filters once group, domain, and measure are determined.

## 10. Research Context

Free Join can execute partial plans and probes before full binding.

Aggregate-domain events are a natural payload of a Free Join node.

Once the aggregate payload is determined, remaining variables are often existential.

Factorized databases avoid expanding repeated suffixes when the consumer can operate on a factor.

Bumbledb's explicit aggregate domains let the engine avoid suffix expansion safely.

This is one of the largest set-engine performance opportunities.

## 11. Definitions

Group vars are projected variables before aggregate terms in aggregate output.

Domain vars are aggregate term domain variables.

Measure var is the aggregate term measured variable.

Aggregate event is a candidate contribution to an aggregate state.

Aggregate event key is group key plus domain key plus aggregate ordinal.

Aggregate event depth is the earliest depth where group vars, domain vars, and measure var are bound.

Existential suffix is remaining query work after an event is determined.

## 12. Invariants

No aggregate event may apply unless the full query has at least one valid extension.

No aggregate event may apply more than once for the same group/domain/ordinal.

No valid aggregate event may be omitted.

`count_domain` must not decode unused measure values.

`count_distinct` must use distinct measured values as its domain.

`sum`, `min`, and `max` rely on PRD 03 functional determination.

Aggregate output ordering remains canonical.

Global empty count behavior remains correct.

Grouped empty aggregate behavior remains correct.

## 13. Implementation Plan

Add aggregate execution metadata per term.

Compute event depth for each term.

Compute group key availability depth.

Compute domain key availability depth.

Compute measure availability depth.

At each LFTJ depth, identify aggregate terms whose event data is available.

Before applying an event, prove remaining suffix existence if not at full depth.

If proof succeeds, apply event and stop exploring deeper for that event when safe.

If proof cannot be built safely, fall back to full-depth aggregate sink.

Preserve late sink as fallback initially.

Add counters distinguishing early events from full-depth events.

## 14. Semijoin Proof Requirements

The proof must account for remaining atoms.

The proof must account for predicates involving remaining variables.

The proof must account for repeated variable constraints.

The proof must account for literals and inputs.

The proof must never return true without checking required existence.

The proof may return unknown and force fallback.

The proof may reuse projection semijoin helpers from PRD 09.

The proof must be deterministic.

## 15. Aggregate State Changes

Separate aggregate event handling from full binding handling.

Add a method like `apply_event(group_key, domain_key, measure_value)`.

Do not require complete `EncodedBinding` for count-domain events.

Do not decode measured values for count-domain.

For sum/min/max, use encoded measure value from binding or column reference.

Keep overflow behavior identical.

Keep decimal scale behavior identical.

Keep min/max ordering identical.

## 16. Pushdown Candidate Changes

The historical aggregate pushdown candidate was nominal and must not be reintroduced without a distinct implementation.

It must become real or be removed.

If implemented, it must use event-depth metadata.

If implemented, it must have different cost estimates from pure LFTJ.

If not implemented in this PRD, remove the candidate from optimizer traces.

Do not leave a fake candidate with worse tie rank.

## 17. Required Tests

Aggregate over domain with many existential witnesses applies once per domain.

Counters prove fewer completed bindings than full witness count.

`count_domain` avoids decoding non-domain measure values.

`count_distinct` dedups measured values correctly.

`sum` over unique domain applies once per domain key.

`min` over unique domain applies once per domain key.

`max` over unique domain applies once per domain key.

Fallback path remains correct when early proof cannot be built.

Global empty count remains correct.

Grouped empty aggregate remains correct.

## 18. Required Golden Tests

Ledger aggregate examples remain correct.

Joinstress aggregate examples remain correct.

TPC-H subset aggregate examples remain correct.

IMDb/JOB aggregate examples remain correct.

Any aggregate golden with duplicate witnesses should assert domain-event counters if deterministic.

## 19. Required Diagnostics

Add `aggregate_early_event_attempts`.

Add `aggregate_early_events_applied`.

Add `aggregate_early_events_duplicate`.

Add `aggregate_early_event_semijoin_failures`.

Add `aggregate_early_event_fallbacks`.

Keep `aggregate_emit_calls` for full-depth fallback.

Expose domain-event counts in benchmark JSON after PRD 16.

## 20. Benchmark Requirements

Add focused aggregate benchmark with many existential duplicates per domain.

Benchmark must validate exact aggregate values.

Benchmark must report completed bindings.

Benchmark must report aggregate early events applied.

Benchmark must report duplicate domain events avoided.

Add a focused gate for reduced witness completion after this PRD.

## 21. Passing Criteria

At least one aggregate shape applies events before full binding.

Aggregate results remain exact.

Aggregate event counters prove reduced witness work on focused fixture.

Fake aggregate pushdown candidates are absent or implemented for real.

No aggregate domain validity rule from PRD 03 is weakened.

The global validation gate passes.

The query-focused validation gate passes.

## 22. Failure Modes

Applying aggregate events without proving suffix existence is a failure.

Applying the same domain event twice is a failure.

Omitting a valid domain event is a failure.

Using full binding for every tested aggregate shape is a failure unless explicitly scoped as fallback.

Leaving fake aggregate pushdown in traces is a failure.

Changing aggregate overflow behavior is a failure.

## 23. Non-Goals

Do not add new aggregate functions.

Do not add approximate aggregates.

Do not change public aggregate API.

Do not implement vectorized execution.

Do not implement COLT.

Do not change storage layout.

## 24. Completion Notes

Document supported early aggregate shapes.

Document fallback shapes.

Keep duplicate-domain aggregate tests permanent.

This PRD completes the set-native output execution foundation started by PRD 09.
