use bumbledb_core::query_ir::TypedQuery;

use crate::query::binary2fj::{binary2fj, factor_plan};
use crate::query::cover::{CoverPolicy, ExecutionMode, ExecutionStats};
use crate::query::free_join::{FjPlan, ValidatedFjPlan};
use crate::query::model::NormalizedQuery;
use crate::query::normalize::normalize_query;
use crate::query::planner::deterministic_binary_plan;
use crate::query::run::execute_validated_plan;
#[cfg(test)]
use crate::query::sink::CountingSink;
use crate::query::sink::ProjectionSink;
use crate::{Error, InputBindings, QueryResultSet, ReadTxn, Result, StorageSchema};

pub(crate) fn execute_query(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    inputs: &InputBindings,
) -> Result<QueryResultSet> {
    let normalized = normalize_query(schema.descriptor(), query)?;
    validate_supported(&normalized, inputs)?;
    let plan = default_plan(&normalized)?;
    let mut sink = ProjectionSink::new(txn);
    let mut stats = ExecutionStats::default();
    execute_validated_plan(
        txn,
        schema,
        &normalized,
        &plan,
        ExecutionMode::Scalar,
        CoverPolicy::DynamicMinKeys,
        &mut stats,
        &mut sink,
    )?;
    sink.finish(&normalized)
}

#[cfg(test)]
pub(crate) fn execute_plan_for_test(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    plan: &FjPlan,
) -> Result<QueryResultSet> {
    let normalized = normalize_query(schema.descriptor(), query)?;
    validate_supported(&normalized, &InputBindings::new())?;
    let validated = validate_plan(plan, &normalized)?;
    let mut sink = ProjectionSink::new(txn);
    let mut stats = ExecutionStats::default();
    execute_validated_plan(
        txn,
        schema,
        &normalized,
        &validated,
        ExecutionMode::Scalar,
        CoverPolicy::DynamicMinKeys,
        &mut stats,
        &mut sink,
    )?;
    sink.finish(&normalized)
}

#[cfg(test)]
pub(crate) fn execute_plan_with_policy_for_test(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    plan: &FjPlan,
    cover_policy: CoverPolicy,
) -> Result<(QueryResultSet, ExecutionStats)> {
    execute_plan_with_mode_for_test(
        txn,
        schema,
        query,
        plan,
        ExecutionMode::Scalar,
        cover_policy,
    )
}

#[cfg(test)]
pub(crate) fn execute_query_with_mode_for_test(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    execution_mode: ExecutionMode,
) -> Result<(QueryResultSet, ExecutionStats)> {
    let normalized = normalize_query(schema.descriptor(), query)?;
    validate_supported(&normalized, &InputBindings::new())?;
    let plan = default_plan(&normalized)?;
    let mut sink = ProjectionSink::new(txn);
    let mut stats = ExecutionStats::default();
    execute_validated_plan(
        txn,
        schema,
        &normalized,
        &plan,
        execution_mode,
        CoverPolicy::DynamicMinKeys,
        &mut stats,
        &mut sink,
    )?;
    Ok((sink.finish(&normalized)?, stats))
}

#[cfg(test)]
pub(crate) fn execute_plan_with_mode_for_test(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    plan: &FjPlan,
    execution_mode: ExecutionMode,
    cover_policy: CoverPolicy,
) -> Result<(QueryResultSet, ExecutionStats)> {
    let normalized = normalize_query(schema.descriptor(), query)?;
    validate_supported(&normalized, &InputBindings::new())?;
    let validated = validate_plan(plan, &normalized)?;
    let mut sink = ProjectionSink::new(txn);
    let mut stats = ExecutionStats::default();
    execute_validated_plan(
        txn,
        schema,
        &normalized,
        &validated,
        execution_mode,
        cover_policy,
        &mut stats,
        &mut sink,
    )?;
    Ok((sink.finish(&normalized)?, stats))
}

#[cfg(test)]
pub(crate) fn count_bindings_for_test(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
) -> Result<usize> {
    let normalized = normalize_query(schema.descriptor(), query)?;
    validate_supported(&normalized, &InputBindings::new())?;
    let plan = default_plan(&normalized)?;
    let mut sink = CountingSink::default();
    let mut stats = ExecutionStats::default();
    execute_validated_plan(
        txn,
        schema,
        &normalized,
        &plan,
        ExecutionMode::Scalar,
        CoverPolicy::DynamicMinKeys,
        &mut stats,
        &mut sink,
    )?;
    Ok(sink.count)
}

fn default_plan(query: &NormalizedQuery) -> Result<ValidatedFjPlan> {
    let binary = deterministic_binary_plan(query).map_err(invalid_plan)?;
    binary.validate(query).map_err(invalid_plan)?;
    let fj = binary2fj(query, &binary).map_err(invalid_plan)?;
    let (factored, _trace) = factor_plan(query, &fj).map_err(invalid_plan)?;
    validate_plan(&factored, query)
}

fn validate_plan(plan: &FjPlan, query: &NormalizedQuery) -> Result<ValidatedFjPlan> {
    plan.validate(query).map_err(invalid_plan)
}

fn validate_supported(query: &NormalizedQuery, inputs: &InputBindings) -> Result<()> {
    if !query.inputs.is_empty() || !inputs.is_empty() {
        return Err(Error::unavailable("runtime query inputs", "PRD 15"));
    }
    if !query.comparisons.is_empty() {
        return Err(Error::unavailable("query comparisons", "PRD 15"));
    }
    if query
        .atoms
        .iter()
        .any(|atom| !atom.source_predicates.is_empty())
    {
        return Err(Error::unavailable("source predicates", "PRD 15"));
    }
    Ok(())
}

fn invalid_plan(error: impl std::fmt::Display) -> Error {
    Error::invalid_query(error.to_string())
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "vectorized_tests.rs"]
mod vectorized_tests;
