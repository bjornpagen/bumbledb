# BumbleDB: structural algebra implementation request

**Prepared:** 2026-09-11  
**Audience:** the next agent implementing changes in BumbleDB  
**Status:** technical implementation brief; this document does not report completed implementation.

## 1. Objective and scope

Reduce application construction code and repeated schema proofs by extending BumbleDB's existing structural relational model. The motivating consumer is Emuroll, whose tax calculations, filing evidence, and calendar coverage already use native BumbleDB facts and constraints.

Implement, in priority order:

1. Constructive interval queries: intersection, difference, and scalar measurement, composable with existing query stages and `pack`.
2. A structural authoring helper for exhaustive closed alternatives, lowered to existing keys and containment laws.
3. A narrowly scoped native row-equation constraint, sharing the existing scalar semantics.
4. A bounded exact integer multiply/divide operation with explicit rounding.

Also deliver a **design-only proposal** for grouped signed conservation equations. That larger admission feature is not part of the required engine implementation in this request. Specify its dependency and final-state semantics; do not quietly implement an arbitrary query-assertion framework while adding row equations.

The priorities indicate implementation order. Complete and verify each layer before building on it. Names shown below are semantic descriptions or explicitly marked pseudocode, not claims about the current public API. Select concrete public spellings consistent with the repository.

## 2. Non-negotiable architectural boundaries

### 2.1 Structural typing only

BumbleDB must remain structurally typed. Do not add database-level nominal identities, unit tags, dimension registries, currency types, epoch types, domain brands, or hidden application-specific compatibility rules.

The database may distinguish `i64`, `u64`, `f64`, their interval shapes, fixed-width refinements, and existing closed-relation structure. It must not distinguish two otherwise compatible integer fields because an application calls one cents and another days.

Units, epochs, business identity brands, and semantic refinements belong to the host language. Existing host-only newtype conveniences need not be removed. Do not introduce new nominal compatibility into persisted descriptors, structural query checking, or constraint judgment. Existing closed-roster and relational carrier checks remain authoritative; this request does not weaken them.

An explicit scalar measure and an interval width may be compared when their structural result kinds agree. The caller owns the application's unit discipline. Do not reintroduce the previously removed scalar-versus-duration prohibition as a substitute for a unit system.

### 2.2 One native meaning

- Extend the existing native interval, scalar, query, and admission machinery. Do not create a parallel JavaScript evaluator or an Emuroll-specific execution path.
- Pure TypeScript authoring must remain native-runtime-free. Database work remains Effect-native, scoped, and lazy.
- Preserve set semantics, explicit contribution identity, checked arithmetic, complete query results, immutable candidate admission, and the existing expected-state protocol.
- Do not add null or empty interval values. Absence of a derived segment is represented by absence of an answer row.
- Do not hide approximation, arithmetic failures, missing evidence, or unsupported shapes behind defaults.
- Avoid compatibility shims and duplicate public mechanisms. Preserve durable data through the repository's explicit versioning and migration contracts; a hard API cutover does not authorize destroying history.
- Prefer representation changes that delete an actual construction or proof burden. A wrapper that relocates the same branching without improving a checked contract is insufficient.

### 2.3 Explicit exclusions

This request does not add Gregorian calendar arithmetic, holiday rules, recurrence languages, business-day calendars, tax-specific scalar operators, workflow engines, general ordered folds, general rational/decimal storage types, automatic database maintenance triggers, or aggregate recursion.

Emuroll's calendar compiler can continue expanding reviewed policy into bounded calendar facts. Its deposit carryover calculation can remain an explicit ordered computation. Existing joins, Allen predicates, negation, `pack`, and numeric aggregates should be used where sufficient.

## 3. Repositories and observed baseline

Inspect the current checkout and applicable instructions before implementation. These revisions identify the source inspected for this request, not a requirement to reset either repository:

| Repository | Local path | Inspected commit |
| --- | --- | --- |
| BumbleDB | `/Users/bjorn/Documents/bumbledb` | `79ddf5832a5249f9f9db0b9c8153a65a53084d74` |
| Emuroll | `/Users/bjorn/Documents/emuroll` | `93815947c53432e7ecac1b184fc49a77b51b21bf` |

Both working trees were clean when inspected. BumbleDB's TypeScript packages identified the 1.2.1 surface; Effect was pinned to `4.0.0-rc.112`. Recheck these facts before changing dependencies or generated outputs.

Emuroll uses local package links to the sibling BumbleDB build. Rebuilds can temporarily remove linked output directories. Coordinate with any active work in that checkout before rebuilding; do not reset or overwrite another task's changes.

### 3.1 Concrete consumer friction

Paths in this table are relative to `/Users/bjorn/Documents/emuroll`.

| Location | Observed responsibility | Desired improvement |
| --- | --- | --- |
| `src/core/values.ts`, `intersection` | Host construction of overlapping intervals | Native derived segment |
| `src/payroll.ts`, calculation preparation | Host iteration over bands; insertion of `BasisSlice`; copied widths | Derived slices and measures from existing inputs |
| `src/schema.ts`, `CalculationWageBase`, `CalculationBasis`, `BasisSlice` laws | Opposing capacity ceilings prove a scalar equals interval width | Derive the width, or express the retained scalar's equality directly |
| `src/calculations.ts`, `assessmentProjection` | Explicit contribution identity, input scoping, weighted sums, rounding expression | Preserve these semantics through new operators |
| `src/reconciliation.ts`, `paymentEquation` | Signed financial equality computed in application code | Motivating case for the grouped-equation design only |
| `src/payments.ts` and `src/work.ts` | Reconciliation checked at command planning and completion projection | Future native conservation constraint; no premature removal of these checks |
| `src/schema.ts`, submission/assessment variants | Repeated closed discriminator and payload wiring | Structural authoring helper |
| `src/policy/calendar.ts` | Reviewed calendar expansion and following-business-day selection | Remains application policy; not an upstream calendar project |

Do not use the live Emuroll database or private evidence for fixtures. Reproduce the relevant shapes with synthetic data in BumbleDB. Do not perform an Emuroll schema migration, alter payroll records, or replace backups as part of this upstream implementation request.

### 3.2 Relevant upstream entry points

Paths below are relative to `/Users/bjorn/Documents/bumbledb`; follow current ownership if code moves.

- `crates/bumbledb-theory/src/interval.rs`: checked intervals, endpoint conventions, discrete duration, dense length.
- `crates/bumbledb-theory/src/schema.rs` and `crates/bumbledb-theory/src/schema/spec.rs`: structural schema representation and descriptor lowering.
- `crates/bumbledb/src/scalar.rs`: shared typed scalar expressions and native evaluation.
- `crates/bumbledb/src/ir/`, `crates/bumbledb-query/`: query validation, representation, notation, rendering, and lowering.
- `crates/bumbledb/src/exec/`, especially `exec/sink/aggregate/`: query execution and exact accumulation.
- `crates/bumbledb/src/schema/`: compilation, fingerprints, admission, diagnostics, and dependency scheduling.
- `ts/src/fields.ts`, `scalar.ts`, `selection.ts`, `statements.ts`, `schema.ts`: TypeScript structural authoring.
- `ts/src/query/compute.ts`, `find.ts`, `scope.ts`, `lower.ts`, `description.ts`, `parse-ir.ts`: typed query construction, derived stages, and description round trips.
- `ts-log/` and the native log implementation: schema migration and durable-history integration where affected.
- `lean/Bumbledb/`, `lean/conformance/`, `scripts/battery.sh`: existing semantic specification and conformance workflow.
- `ts/COOKBOOK.md`, `docs/cookbook.md`: existing modeling recipes; verify prose against current compiled tests rather than treating old limitations as immutable policy.

## 4. Workstream A: constructive interval operations

### 4.1 Required mathematical semantics

For two valid half-open intervals `A = [a,b)` and `B = [c,d)` of the same structural element kind:

```text
intersection(A,B) = {[max(a,c), min(b,d))} if max(a,c) < min(b,d)
                  = {} otherwise

difference(A,B)   = the maximal nonempty half-open segments of A outside B
```

An intersection produces zero or one segment. A binary difference produces zero, one, or two segments. Outputs contain no empty segments, duplicates, or overlaps. A subtraction that does not intersect `A` returns `A`; subtracting a containing interval returns no rows.

Examples:

```text
[0,10) intersect [3,7) = {[3,7)}
[0,3)  intersect [3,7) = {}
[0,10) minus     [3,7) = {[0,3), [7,10)}
[0,10) minus     [0,10) = {}
[0,10) minus     [10,20) = {[0,10)}
```

No enumeration of represented points is permitted. Complexity for binary construction is bounded by endpoint comparison and construction, independent of interval width.

The query output is a relation of segments. Do not add array-valued interval fields or encode an empty result as `[x,x)`, null, a sentinel record, or a failed scalar operation.

### 4.2 Structural result types

- Order-only operations must cover the existing `interval(i64)`, `interval(u64)`, and `interval(f64)` families using their existing endpoint semantics.
- Operands must have the same element kind. Mixed signed/unsigned or integer/float operations require explicit existing conversions, if those conversions are available; do not promote implicitly.
- The constructed result is a general interval of that element kind. Fixed-width inputs do not justify an unchecked fixed-width result. Losing an unproven width refinement is correct.
- Preserve the existing canonical treatment of floating endpoints, including NaN refusal and signed-zero normalization. Use ordering, not epsilon comparisons.
- Do not change structural compatibility because a field has an application-specific name.

### 4.3 Measurement and unboundedness

Expose measurement in query expressions, including expressions in imported derived stages:

```text
measure(interval(i64)) -> u64, when bounded
measure(interval(u64)) -> u64, when bounded
measure(interval(f64)) -> f64, using existing native dense-length semantics
```

Reuse the existing interval implementation's exact meanings. Integer maximum endpoints are reserved ray endpoints, not ordinary finite terminal points. Subtracting the encoded endpoints of a ray and calling that its finite width is incorrect.

For a bounded signed integer interval, compute the exact nonnegative width without an intermediate signed overflow. For dense intervals, retain the existing once-rounded endpoint-difference behavior and distinguish unboundedness from finite-length overflow.

Intersection and difference may validly return rays or other supported unbounded dense intervals. Measurement of an unbounded result must fail through a typed evaluation result; it must not become zero, the maximum integer, infinity, or an omitted contribution. In particular, a finite interval intersected with a ray can have a valid finite measure.

### 4.4 Query composition and execution

The public syntax may use a checked derived binding or a small relation-producing operator. It must lower to native query execution. Merely returning a JavaScript function that walks collected rows does not meet this requirement.

For each incoming binding `b`, a segment-producing operation extends `b` with each resulting segment. Zero resulting segments eliminate that binding. Multiple segment-producing operations compose relationally; independent outputs form their ordinary combinations, not positional zips.

Inputs must be fully bound at the operation's stage. Do not introduce inverse solving for unknown endpoints. These operations must compose in nonrecursive query stages, including further joins, Allen predicates, anti-joins, and aggregates over a subsequent stage. Extending the recursive grammar is out of scope.

Keep result ordering unspecified. Maximal/disjoint output describes the represented set, not a delivery-order promise. `pack` retains its existing coalescing semantics; do not introduce another union/coalescing evaluator.

Preserve stage boundaries around partial operations. For example, construct or filter a finite overlap before measuring it; the optimizer must not speculatively measure an unrelated unbounded input. Existing query failures must not be masked by filtering final output after a failing interior has already evaluated.

### 4.5 Synthetic consumer acceptance case

Model an earning interval and a partitioned marginal schedule:

```text
earning: [80,140)
band A:  [0,100),   numerator 1
band B:  [100,200), numerator 2
common denominator: 10

derived contributions:
  band A overlap [80,100),  width 20, weighted amount 20
  band B overlap [100,140), width 40, weighted amount 80

weighted total: 100
rounded amount: 10
```

These are synthetic arithmetic units and rates, not actual tax policy. Produce this answer in a native query from stored earning and band facts without inserting `BasisSlice` facts or copied widths.

Retain a contribution key such as `(calculation, band)` or an existing source identity through intermediate projections. Add a case with two distinct equal contributions and prove both count. Do not change the database to bag semantics or guess identity from amount equality.

This demonstration proves query capability. It does not prove that Emuroll can discard historical inputs, applied-policy references, or evidence. Document the remaining consumer migration work separately.

## 5. Workstream B: exhaustive closed alternatives

### 5.1 Authoring contract

Provide a pure structural helper for an existing parent relation, a closed discriminator, and one payload relation per discriminator arm. Reuse ordinary declared relations and existing closed rosters. The helper should produce inspectable ordinary statements; it is not a new native storage or admission primitive.

The declared structure must establish:

1. The parent has a declared key `K`.
2. The discriminator references the specified closed roster.
3. Each arm's child has a declared key corresponding to `K`.
4. For each handle `h`, parent keys selected by `tag = h` equal the keys of that arm's payload relation.
5. Every handle is covered exactly once by the declaration.

For a parent row, these laws prove exactly one matching payload: the selected child must exist, other arm children are excluded by their reverse containment, and each child key ensures uniqueness.

The helper must accept structurally compatible, independently constructed declarations as the current SDK does. Do not depend on object identity, declaration creation order outside the existing schema contract, hidden symbols, class instances, or a nominal registry.

### 5.2 Lowering and limits

Lower to the existing key, closed-reference, selection, and bidirectional containment forms. Make generated ordering deterministic and provenance inspectable. Equivalent manual expansion with the same ordered declarations must yield the same native descriptor/fingerprint. Do not broaden the repository's global statement-order equivalence contract as a side effect.

Keep payload relationships explicit. A certified-mail payload referencing a mailing record still needs its ordinary ownership and evidence laws. The helper cannot infer business ownership, evidence sufficiency, string validity, or correspondence of unrelated payload fields merely from an arm label.

Reject incomplete or duplicate arm maps, unknown handles, invalid parent/child projections, and structurally incompatible discriminator declarations. Define how callers supply existing keys without emitting redundant duplicate key declarations; avoid a hidden global deduplication pass.

Expose a structural inferred TypeScript union if it can be derived directly from the declaration. Do not add another serializer, record format, or runtime union interpreter solely to produce a nicer TypeScript type. The stored representation remains the existing relations.

### 5.3 Required acceptance cases

Use synthetic variants such as `Imported`, `Electronic`, and `Postal`.

- Every valid arm admits with its matching payload.
- Missing payload, extra wrong-arm payload, unknown parent, and mismatched parent key refuse.
- Switching an arm and replacing its payload in one final-state change admits; call ordering must not become semantic.
- Equivalent independently constructed descriptors work; conflicting descriptors with the same names refuse through normal structural checks.
- Manual and helper-authored schemas have equal normalized descriptors under the same ordering.
- TypeScript rejects incomplete arm maps and preserves arm-specific payload fields.

## 6. Workstream C: native row equations

### 6.1 Deliberately small first implementation

Add a schema statement meaning:

```text
for every row r in a declared relation, optionally restricted by existing literal selection:
    evaluate(left, r) = evaluate(right, r)
```

The initial admitted equality result types are matching `i64` or matching `u64`. Expressions may use existing checked scalar arithmetic, explicit exact casts, and the new interval measurement where its result kind is admitted. Floating equality, general predicates, quantified joins, aggregates, path traversal, callbacks, and recursive assertions are outside this first constraint form.

Illustrative pseudocode, not an existing API:

```text
RowEquation(Slice, field("cents"), measure(field("span")))
```

Bind field references against the row's declared structure and compile both sides into the existing native scalar machinery. There must not be one interpretation for query computations and another for schema equations. Source-field leaves can remain unresolved during pure authoring, but schema binding must type-check the entire expression before any database rows are considered.

### 6.2 Admission and failure contract

- An empty relation does not excuse an invalid field reference or an ill-typed expression.
- An equation holds vacuously over no selected rows; existence is supplied by ordinary containment/cardinality laws, not inferred from arithmetic.
- A false equation rejects the candidate as an invariant violation with the authored statement's identity and a useful row witness.
- A row-local arithmetic domain failure must make admission fail closed with a distinguishable typed reason. Specify its mapping to the repository's invariant versus operational-error result channels; never turn it into a false successful predicate or discard the row.
- Deleting the offending row, or changing the selector so the final row is no longer selected, is judged against the final state. Intermediate mutation order is irrelevant.
- `judge` and `apply` use the same law and diagnostics. Preserve the existing distinction between an admissible judgment and a later successful commit under the expected-state witness.
- Schema adoption/migration must check all existing selected rows under a newly introduced equation. Ordinary writes must check all affected final rows, including selection-changing replacements.

This row-local form does not need cross-relation invalidation: references are local by construction. Do not claim it solves payment reconciliation across liability and adjustment relations.

### 6.3 Why retain this primitive if widths become derived?

Derived values should normally remain derived. The equation is useful when a value is independently supplied or intentionally retained as an observation and must agree with a structural calculation. It is not a reason to materialize every computable field.

For the synthetic slice case, demonstrate both choices: a query-derived width, and a separately retained observed width constrained to agree. Show that the latter replaces the current opposing self-capacity proof without changing its accepted bounded integer cases.

## 7. Workstream D: exact integer multiply/divide and rounding

### 7.1 Bounded scalar operation

Add one canonical operation whose denotation is:

```text
mulDiv(a, b, d, rounding) = round_according_to_mode((a * b) / d)
```

The spelling is provisional. Its semantic contract is required:

- `a`, `b`, and `d` have one matching structural integer kind, `i64` or `u64`; the result has that kind.
- `d` must be strictly positive. Zero or negative denominators fail with a typed reason.
- Evaluate the product exactly in sufficient internal width before dividing. Check the public result range after rounding. Two 64-bit operands admit a bounded wide implementation; no new persisted big-integer or rational field type is needed.
- Keep old multiply and divide semantics unchanged. An explicitly authored checked multiplication may still overflow; do not silently replace arbitrary expression trees with the new operation unless equivalence includes error behavior.
- Share the operation across existing scalar authoring/binding/evaluation paths. Do not add a JavaScript arithmetic fallback.

Use a small explicit rounding roster: `TowardZero`, `NearestTiesAwayFromZero`, and `NearestTiesToEven`. All names must have specified negative-value behavior. Additional modes need a demonstrated consumer, not speculation.

For integer exact product `p`, positive divisor `d`, and `abs(p) = q*d + r`, use exact comparison of `r` with `d-r` to decide below/above/tie without doubling into an overflow. Apply sign and tie rules explicitly. A zero remainder must not increment the quotient.

Examples:

```text
 5 / 2: toward zero = 2; nearest ties away = 3; nearest ties even = 2
-5 / 2: toward zero = -2; nearest ties away = -3; nearest ties even = -2
 7 / 2: nearest ties even = 4
-7 / 2: nearest ties even = -4
```

Boundary cases must include `u64::MAX * 2 / 2` returning `u64::MAX`, and `i64::MIN * -1 / 2` returning `2^62`; their final answers fit even though a same-width intermediate multiply would fail. `i64::MIN * -1 / 1` must fail because its final result does not fit `i64`.

### 7.2 Honest limits for tax composition

This operation can replace an overflow-prone rounding formula applied to a representable weighted total, for example `mulDiv(weightedTotal, 1, denominator, mode)`.

It does not by itself make an arbitrarily large sum of arbitrary products representable. Existing scalar multiplication before aggregation still has its own checked result range; existing exact sums still finalize into their declared result kind. Preserve and document those boundaries.

The synthetic consumer query must sum contributions before its final rounding. Do not replace that with separately rounded slices. A general exact rational aggregate is outside this request.

## 8. Required design appendix: grouped signed conservation equations

Produce a concrete design proposal, with alternatives and dependency implications, for:

```text
for each declared reconciliation R:
    observedAmount(R)
      = sum(allocatedLiabilities(R)) + sum(evidencedAdjustments(R))
```

The motivating relations are structural and generic. Domain evidence and authorization remain ordinary independently constrained facts. A zero arithmetic difference does not establish that a payment happened, a form was submitted, or a credit was authorized.

Resolve these questions explicitly in the appendix:

1. **Anchoring:** which base relation enumerates groups, including groups with zero contributions? An inner join must not silently remove an unbalanced parent.
2. **Identity and multiplicity:** which keys identify contributions? How are intentional joins distinguished from fanout that repeats amounts? Ordinary relational projection can collapse equal values if identity is discarded.
3. **Empty groups:** the additive identity can be zero for a declared arithmetic group; absent employer evidence or absent source observations must remain absent. Do not use this as a general fallback policy.
4. **Dependency tracking:** source inserts/deletes, group-key changes, parent changes, upstream computed liability changes, and deletions of the final contributor must all invalidate the affected groups.
5. **Signed arithmetic:** do not reuse the monotonicity assumptions of unsigned capacity scheduling. An insertion can reduce a sum, and a deletion can increase it.
6. **Evaluation:** final-state equations, wide deterministic sums, range checks, and partial scalar operations need one specified interpretation. Changes that balance only as a whole must admit atomically.
7. **Read reuse:** how can reports and admission reference the same declared expression without introducing separately maintained operational totals?
8. **Versioned inputs:** binding to an applied immutable policy version is materially different from implicitly following the newest mutable catalog row. Preserve explicit dependencies.
9. **Surface limits:** compare a dedicated grouped equation to a restricted nonrecursive violation relation. Account for schema/query layering, parameter binding, negation, compilation dependencies, and diagnostics; do not assume an unrestricted query assertion is cheap.
10. **Durability and maintenance:** describe schema fingerprints, adoption validation, history replay, migration behavior, and a measurable invalidation strategy.

Clearly distinguish what the row-equation implementation establishes from what this future grouped feature would establish. Keep current application reconciliation guards until the larger invariant is actually enforced.

## 9. Cross-layer implementation requirements

For every new engine-level node or statement, audit all owners of the representation. Follow the existing shared structural specification; do not patch only the TypeScript facade.

Required coverage includes, where applicable:

- Pure TypeScript construction and inferred result/parameter types.
- Shared structural schema or query descriptions, native binding, and compile-time rejection.
- Native plan/execution and the existing numerical environment.
- Query description import/export, rendering/parsing, prepared execution, and nonrecursive composition.
- Structural fingerprints, persisted schema metadata, migration generation/adoption, and durable command replay.
- Diagnostic attribution, typed operational errors, invariant failures, and resource cleanup on failure/interruption.
- Corresponding public Rust authoring paths and notation where that construct is exposed; no accepted spelling with silently different behavior.
- Package exports, generated declarations, and packaged-consumer compilation. The inspected upstream revision specifically addressed portable exported query types; do not regress that surface.
- Lean definitions and executable conformance where the extension changes represented semantics. Do not claim a theorem proves new operations until the theorem and correspondence cases actually cover them.

The closed-alternative helper should stop at ordinary statement lowering and therefore avoid adding a new persisted native kind. New row equations do alter the schema vocabulary: use explicit format/version handling rather than silently reinterpreting old descriptors. Untouched schemas must retain their existing meaning and identity unless an intentional migration contract says otherwise.

Before extending any wire grammar, identify its current versioning rule and add tests for supported old data and rejected unknown new data. A parser that ignores unknown fields or operations is not acceptable.

## 10. Validation matrix

Use independent mathematical oracles and counterexamples. Do not validate a new evaluator solely by running its own implementation through a second facade.

| Area | Required evidence |
| --- | --- |
| Interval algebra | Cover all Allen relationships, equality, adjacency, disjointness, containment, negative endpoints, extrema, rays, fixed-width inputs, and existing dense endpoint cases |
| Algebraic properties | Intersection commutativity/idempotence; difference disjoint from subtrahend; reconstruct `A` by coalescing its difference and intersection; bounded integer measures add under that disjoint decomposition |
| Set semantics | Distinct equal contributions both count when keyed; projection still deduplicates by its actual head; multiple generated segments retain source scope |
| Composition | Derived result kinds survive imported stages and description round trips; partial operations respect declared stage scope |
| Structural typing | Independently constructed equivalent declarations work; mixed structural numeric kinds refuse; no new brand/unit registry appears in IR or descriptors |
| Closed alternatives | Missing/wrong/doubled payloads refuse, complete atomic arm switch admits, helper/manual descriptors agree |
| Row equations | Empty-schema type errors, false equality, measurement errors, selection transitions, atomic replacement, adoption over old rows, judge/apply equivalence |
| Exact quotient | Every rounding mode, both signs, exact quotient, both sides of ties, denominator errors, intermediate-overflow/final-fit cases, final overflow |
| Durability | Schema/IR round trips, explicit version handling, affected log/migration/replay paths, unchanged meanings for preexisting constructs |
| Consumer usefulness | Native synthetic marginal-band calculation with no inserted slices/copied widths; explicit evidence and selected policy remain modeled |

Finite-grid enumeration is a useful independent oracle for integer interval tests only; it must not become the production implementation. Dense tests should use exact endpoint ordering and the established native measure oracle. Do not assert exact additivity of already-rounded floating lengths: that law applies to exact/bounded integer measurements, not arbitrary sums of binary64 results.

Add focused performance measurements for generated segment cardinality, query stage composition, equation admission on affected rows, and the scalar operation. Compare against the existing equivalent workload where one exists. Report measurements with source revision, environment, dataset, and methodology; make no unsupported speedup claims.

Run the repository's applicable full correctness battery and packaged-consumer checks after focused tests pass. Follow the current release procedure if publication is separately requested. Implementing this request alone does not require publishing an npm release or modifying private consumer data.

## 11. Completion criteria and handoff

The implementation handoff must contain:

1. The final public API and precise semantics for all four implementation workstreams.
2. Working native synthetic examples demonstrating the intended reduction in construction code and schema wiring.
3. Required focused tests, applicable full checks, and transparent reporting of any unrun environment-dependent checks.
4. Updated authoritative documentation/examples that agree with compiled behavior; stale claims about host-only arithmetic or interval construction corrected where affected.
5. The grouped-equation design appendix, explicitly labeled unimplemented.
6. A compact Emuroll adoption note identifying replaceable code/facts, required migration/evidence preservation, and checks that must remain. No live adoption is performed in this request.
7. A clean, reviewable change set with scratch files, generated junk, and real company data excluded.

Do not report success merely because a TypeScript helper compiles. The interval and equation capabilities must execute and enforce their native semantics; the helper must lower to existing laws; the arithmetic must be exact within its specified range; and all four must preserve BumbleDB's structural foundation.
