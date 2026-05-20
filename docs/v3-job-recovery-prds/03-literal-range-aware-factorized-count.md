# PRD 03: Literal And Range Aware Factorized Count

## Goal

Recover the `job_q09_voice_us_actor` regression by extending generic factorized count to support literal atoms and range predicates.

This must not reintroduce JOB-specific relation names. It should generalize the old `aggregate_pushdown` behavior into a structural count plan.

## Explicit Non-Goals

- No backwards compatibility with the old `aggregate_pushdown` implementation shape.
- No migration or aliasing for old plan internals.
- No relation-name-specific count shortcut to recreate previous behavior.
- No compatibility branch for old Datalog aggregate lowering.
- No preserving old bad planner choices for stability.

## Regression Being Fixed

Current JOB 10k:

```text
job_q09_voice_us_actor: 212223us BDB vs 3450us SQLite
runtime: pure_lftj
rows: 1
```

Old scale-10000 artifact:

```text
job_q09_voice_us_actor: ~903us BDB
chosen_plan: aggregate_pushdown
```

The current generic factorized count rejects this shape because it contains literals and range predicates:

```rust
if !query.inputs.is_empty() || !query.predicates.is_empty() {
    return Ok(false);
}

if query.atoms.iter().any(|atom| {
    atom.fields.iter().any(|field| matches!(
        field.term,
        NormTerm::Input(_) | NormTerm::Literal(_)
    ))
}) {
    return Ok(false);
}
```

## Current Code Anchors

- `crates/bumbledb-lmdb/src/query.rs`
- `try_execute_direct_count_query`
- `try_execute_factorized_count`
- `bridge_factorized_count_plan`
- `PlanCounters::factorized_counted_bindings`
- `DirectKernelKind::CountOnly`
- `RelationIndexImage::prefix_count`
- `RelationIndexImage::entries_with_prefix`
- `query_access.rs`

## Required Plan Concept

Add a structural `FactorizedCountPlan` for global count queries.

Recommended shape:

```rust
struct FactorizedCountPlan {
    drivers: Vec<FactorizedDriver>,
    guards: Vec<FactorizedGuard>,
    multipliers: Vec<FactorizedMultiplier>,
    predicates: Vec<FactorizedPredicate>,
}

struct FactorizedDriver {
    atom: AtomId,
    relation: RelationId,
    output_variable: VarId,
    access: AccessId,
}

struct FactorizedGuard {
    atom: AtomId,
    relation: RelationId,
    prefix_terms: Vec<NormTerm>,
}

struct FactorizedMultiplier {
    atom: AtomId,
    relation: RelationId,
    prefix_terms: Vec<NormTerm>,
}
```

The exact representation may differ. The key is that the executor should count compatible bindings without materializing the full join.

## Supported Query Shape In This PRD

Support global count queries with:

- one aggregate term: `count(?x)`
- no group variables
- relation atoms only plus comparison predicates
- literal/input fields inside atoms
- range predicates on one variable
- non-recursive positive joins
- no disjunction
- no negation

This should cover q09.

## Required Algorithm

### Step 1: Build Static Candidate Domains

Reuse PRD 02 static candidate set infrastructure.

For q09, this means:

```text
CompanyName(country_code = "[us]") -> company candidates
Name(gender = "m") -> person candidates
RoleType(role = "actor") -> role candidates
Title(production_year 2005..2015) -> movie candidates
```

### Step 2: Choose Driver Variable

Choose the lowest estimated candidate domain among variables participating in count-relevant joins.

Use exact candidate set size where available. Otherwise use planner stats.

### Step 3: Count By Prefix Products

For each driver value:

- apply guard existence probes
- compute multiplier counts through prefix counts
- multiply independent factors
- add to total

If factors are not independent, the plan must reject and fall back to normal execution.

### Step 4: Handle Predicates

Support simple range predicates on a variable when they can be evaluated before counting.

For q09:

```text
?year >= 2005
?year <= 2015
```

These predicates are attached to `Title.production_year`, and the Title relation binds `movie` and `year`. The factorized planner can either:

- seed `movie` candidates from a `Title` range index
- or treat the Title atom as a guard from movie -> year range

Either is acceptable if performance recovers.

## Rejection Rules

The factorized planner must reject safely when:

- aggregate is not global count
- counted variable dependency graph is cyclic in a way factorization cannot prove independent
- non-count aggregate exists
- predicates cannot be pushed into driver or guard evaluation
- required access paths are missing
- candidate sets exceed a safe memory budget

## Required Explain/Diagnostics

Explain output should identify:

```text
direct_kernel kind=CountOnly target=factorized_count
factorized driver=<relation.field>
factorized guards=<n>
factorized multipliers=<n>
```

Counters should include:

```text
direct_kernel_probes
direct_kernel_rows
factorized_counted_bindings
```

## Required Tests

- Literal-filtered factorized count over serial joins.
- Literal-filtered factorized count over enum joins.
- Range-filtered factorized count.
- Mixed literal + range count matching materialized execution.
- Rejection for unsafe cyclic factorization.
- q09 uses factorized count on JOB 10k.
- q09 output matches SQLite.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- JOB 10k benchmark completes.
- `job_q09_voice_us_actor` Bumbledb avg is under `3000us` on JOB 10k.
- `job_q09_voice_us_actor` beats SQLite on JOB 10k.
- No JOB relation names appear in engine query code.

## Completion Criteria

- q09 regression is fixed generically.
- Factorized count supports literal and range constraints.
- Existing non-JOB correctness and gates still pass.
- This PRD is deleted and committed after passing.
