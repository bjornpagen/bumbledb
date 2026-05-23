# PRD 03: Aggregate Domain And Public IR Hardening

## 01. Status

Not started.

## 02. Severity

Critical correctness and public API safety.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must understand schema constraints before editing aggregate logic.

The implementer must write invalid-query tests first.

The implementer must not weaken existing aggregate tests.

The implementer must not rely on runtime witness order for aggregate values.

## 04. Dependency Order

PRD 01 should be complete before this PRD.

PRD 02 should be complete before this PRD if direct kernels can execute aggregate-adjacent plans.

PRD 04 depends on this PRD for final aggregate correctness expectations.

PRD 10 depends on this PRD because aggregate pushdown is unsafe without domain proofs.

PRD 15 depends on this PRD because optimizer cost must know aggregate domains are valid.

## 05. Problem Statement

Bumbledb's aggregate contract says aggregates operate over explicit set domains.

The current builder accepts some domains that are not valid set domains for the measured value.

The current check only verifies co-occurrence of measure and domain variables in one relation atom.

Co-occurrence does not imply functional determination.

If multiple facts share the domain values and differ on the measure value, current runtime can pick an arbitrary witness.

That violates set-engine semantics.

Public IR can bypass builder checks entirely.

Execution normalization trusts aggregate metadata from `TypedQuery`.

The engine must validate aggregate domains at the execution boundary.

## 06. Code Map

Primary core files:

- `crates/bumbledb-core/src/query_builder.rs`.
- `crates/bumbledb-core/src/query_ir.rs`.
- `crates/bumbledb-core/src/schema.rs`.

Primary LMDB file:

- `crates/bumbledb-lmdb/src/query.rs`.

Relevant current regions:

- `query_builder.rs:320-352` for weak aggregate-domain check.
- `query_ir.rs:62-113` for public mutable typed query fields.
- `query.rs:7836-7862` for normalization of aggregate find terms.
- `query.rs:7988-8004` for insufficient normalized query validation.
- `query.rs:8370-8418` for late domain dedup at aggregate sink.

## 07. Current Behavior

The builder records variable metadata.

The builder records relation atoms.

The builder accepts aggregate find terms.

For `sum`, `min`, and `max`, it calls `aggregate_measure_has_domain_atom`.

That helper scans relation atoms.

It collects variables in each atom.

It accepts the aggregate if the measure variable and all domain variables appear in the same atom.

It does not inspect unique constraints.

It does not inspect full-fact identity.

It does not prove the measure value is determined by the domain variables.

The runtime later builds a domain key from declared domain variables.

The runtime keeps one aggregate application per group/domain key.

If several complete bindings share a domain key but have different measure values, only the first seen value is used.

That first value depends on execution order, not set semantics.

## 08. Concrete Invalid Query

Relation `Posting(id, account, amount)` has unique key `id`.

Many postings can share the same `account`.

Query asks `sum(amount).over([account])`.

The current builder may accept this because `amount` and `account` appear in `Posting`.

The domain `[account]` does not determine `amount`.

There can be several amounts for one account.

The runtime dedups by account.

Only one amount is applied per account.

The result is neither a proper sum over postings nor a valid account-level measure.

The correct behavior is to reject the query.

The caller should use `sum(amount).over([id])` or another unique domain.

## 09. Concrete Valid Query

Relation `Posting(id, account, amount)` has unique key `id`.

Query asks `sum(amount).over([id])`.

The domain `[id]` determines the full posting fact.

The full posting fact determines `amount`.

The aggregate is valid.

If existential joins duplicate the same posting, domain dedup applies amount once.

This is the intended set-domain behavior.

## 10. Another Valid Query

Relation `Balance(account, currency, amount)` has unique key `[account, currency]`.

Query asks `sum(amount).over([account, currency])`.

The domain contains a declared unique key.

The declared unique key determines the fact.

The fact determines `amount`.

The aggregate is valid.

## 11. Research Context

Set aggregates require a well-defined input set.

Free Join can reduce duplicate witnesses only if the domain is explicit and valid.

Domain validity is a functional-dependency question.

Bumbledb does not yet have general functional dependency descriptors.

The available dependencies are relation set identity and declared unique constraints.

Therefore the first implementation must validate against those two dependency sources.

This conservative rule is stricter than co-occurrence.

This conservative rule is necessary before aggregate pushdown.

## 12. Domain Validity Rules

`count_domain(domain)` requires domain length greater than zero.

`count_domain(domain)` requires every domain variable to be bound by at least one relation atom.

`count_domain(domain)` rejects duplicate domain variables.

`count_distinct(var)` requires the variable to be bound by at least one relation atom.

`count_distinct(var)` uses a single-variable distinct domain.

`sum(value).over(domain)` requires a non-empty domain.

`min(value).over(domain)` requires a non-empty domain.

`max(value).over(domain)` requires a non-empty domain.

`sum`, `min`, and `max` require the value variable to be bound by at least one relation atom.

`sum`, `min`, and `max` require the value variable to be functionally determined by domain.

## 13. Functional Determination Rules

A domain determines a value through an atom if that atom contains the value variable.

A domain determines a value through an atom if the domain contains every variable field in that atom.

A domain determines a value through an atom if the domain contains all variable fields corresponding to a declared unique constraint for that relation.

Input terms and literal terms in an atom can be treated as constants for the purpose of full-fact determination.

Wildcard fields do not add variables to the required domain.

Repeated variables in an atom must be handled consistently.

If a unique constraint field is bound to a literal or input, that field is already fixed.

If a unique constraint field is bound to a variable, that variable must be in the domain.

If no atom proves determination, reject the aggregate.

## 14. Public IR Hardening Rules

Execution must validate public `TypedQuery` values.

Variable IDs in `find` must be in range.

Variable IDs in relation fields must be in range.

Variable IDs in comparison operands must be in range.

Variable IDs in aggregate domains must be in range.

Input IDs in relation fields must be in range.

Input IDs in comparison operands must be in range.

Relation IDs must be in range.

Relation names must match relation IDs.

Field IDs must be in range.

Field names must match field IDs.

Field value types must match schema field types.

Aggregate value type must match measured variable type.

Comparison value type must match operand variable or input types.

Projection variables must be bound by relation atoms.

Aggregate variables must be bound by relation atoms.

## 15. Implementation Plan

Create a reusable aggregate validation helper.

Place shared type-level logic in `bumbledb-core` if it does not need encoded LMDB values.

Place schema-aware execution boundary validation in `bumbledb-lmdb`.

Do not trust builder-only validation.

Build a map from variable ID to variable metadata.

Build a map from variable ID to atom field occurrences.

Build a map from relation atom to its variable-bound fields.

For each aggregate term, validate function-specific domain rules.

For each `sum`, `min`, and `max`, search atoms containing the measured variable.

For each candidate atom, test full-fact domain containment.

For each candidate atom, test declared unique domain containment.

Accept only if at least one proof succeeds.

Return structured errors for invalid domains.

## 16. Builder Changes

Replace `aggregate_measure_has_domain_atom` with a proof-based helper.

Builder validation can use its internal typed clauses.

Builder validation must reject invalid domains before appending to `find`.

Builder validation must reject duplicate domain variables.

Builder validation must reject empty domain for non-global aggregate terms.

Builder validation must retain existing type checks.

Builder tests must cover every accepted and rejected case.

## 17. Execution Changes

Extend `validate_normalized_query` or add pre-normalization typed validation.

Do not rely on already-normalized `VarId` casts without range checks.

Reject public IR mismatches with query errors, not internal panics.

Validate aggregate output plan after normalization.

Validate aggregate domains before plan cache lookup if query shape cache could otherwise store invalid data.

Ensure prepared query normalization cache does not cache invalid normalized queries.

## 18. Required Tests

Builder rejects `sum(amount).over([account])` when `account` is not unique.

Builder accepts `sum(amount).over([posting_id])` when `posting_id` is unique.

Builder accepts full-fact domain without a named unique constraint.

Builder accepts compound unique domain when all unique variables are included.

Builder rejects compound unique domain when one component is missing.

Builder rejects duplicate domain variables.

Builder rejects empty domain for `sum`, `min`, and `max`.

Builder rejects aggregate value type mismatch if constructible in tests.

Execution rejects hand-built public IR with invalid domain.

Execution rejects hand-built public IR with out-of-range variable ID.

Execution rejects hand-built public IR with wrong aggregate value type.

Execution rejects hand-built public IR with relation ID/name mismatch.

Execution accepts valid hand-built public IR.

## 19. Differential Tests

Add reference/LMDB differential tests for valid `count_domain`.

Add reference/LMDB differential tests for valid `count_distinct`.

Add reference/LMDB differential tests for valid `sum` over unique domain.

Add reference/LMDB differential tests for valid `min` over unique domain.

Add reference/LMDB differential tests for valid `max` over unique domain.

Include duplicate existential witnesses in at least one test.

The duplicate witnesses must not change aggregate values.

## 20. Error Requirements

Errors must name the aggregate function.

Errors must name the measured variable when applicable.

Errors must identify the invalid domain reason.

Errors must distinguish unknown variable from invalid functional determination.

Errors must distinguish duplicate domain variable from missing domain variable.

Errors must be stable enough for tests to match by error kind.

Do not assert full error strings unless the project already does so.

## 21. Passing Criteria

Builder and execution reject the same invalid aggregate domain shapes.

Public IR cannot bypass aggregate validation.

No accepted aggregate can depend on arbitrary witness order.

Full-fact domains are accepted.

Declared unique domains are accepted.

Mere co-occurrence domains are rejected.

The global validation gate passes.

The query-focused validation gate passes.

## 22. Failure Modes

Accepting co-occurrence as proof is a failure.

Rejecting valid full-fact domains is a failure.

Trusting `TypedFindTerm::Aggregate.value_type` is a failure.

Letting public IR panic instead of returning an error is a failure.

Using relation declaration order as a substitute for variable binding proof is a failure.

Breaking global count over empty input is a failure.

Breaking grouped aggregate empty input is a failure.

Adding a compatibility flag for loose aggregate domains is a failure.

## 23. Non-Goals

Do not implement aggregate pushdown.

Do not add general functional dependency descriptors.

Do not change aggregate output column naming unless needed for errors.

Do not change encoding of aggregate values.

Do not optimize `seen_domains` memory usage here.

Do not modify benchmark SQL here except tests needed for validation.

## 24. Completion Notes

Update normative docs if the exact domain proof rules are not already spelled out.

Keep invalid-domain tests permanent.

Record any conservative rejection that future functional dependency descriptors could relax.

This PRD is a prerequisite for set-native aggregate execution.
