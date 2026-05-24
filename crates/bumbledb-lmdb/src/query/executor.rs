use bumbledb_core::query_ir::TypedQuery;

use crate::diagnostics::set_allocation_tracking_enabled;
use crate::query::cover::{CoverPolicy, ExecutionMode, ExecutionStats};
use crate::query::free_join::{FjPlan, ValidatedFjPlan};
use crate::query::model::NormalizedQuery;
use crate::query::normalize::normalize_query;
use crate::query::planner::{PlanMode, select_plan};
use crate::query::predicate::PredicateMode;
use crate::query::run::execute_validated_plan;
#[cfg(test)]
use crate::query::sink::CountingSink;
use crate::query::sink::ProjectionSink;
#[cfg(test)]
use crate::query::sink::{FactorizedProjectionSink, OutputMode, OutputStats};
use crate::query::trace::{
    ExecutionModePublic, ProfiledQueryResult, QueryExecutionOptions, QueryTrace,
    QueryTraceMetadata, TraceCounters, TracePhase,
};
use crate::{Error, InputBindings, QueryResultSet, ReadTxn, Result, StorageSchema};

pub(crate) fn execute_query(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    inputs: &InputBindings,
) -> Result<QueryResultSet> {
    Ok(
        execute_query_profiled(txn, schema, query, inputs, QueryExecutionOptions::default())?
            .result,
    )
}

pub(crate) fn execute_query_profiled(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    inputs: &InputBindings,
    options: QueryExecutionOptions,
) -> Result<ProfiledQueryResult> {
    set_allocation_tracking_enabled(options.allocation_tracking);
    let mut trace = QueryTrace::new(options.tracing);

    let normalize_span = trace.start_span(TracePhase::Normalize, "normalize query");
    let normalized = normalize_query(schema.descriptor(), query)?;
    validate_supported(&normalized, inputs)?;
    if let Some(span) = normalize_span {
        trace.finish_span(span, TraceCounters::default());
    }

    let plan_span = trace.start_span(TracePhase::PlanSelect, "select Free Join plan");
    let selection = select_plan(txn, schema, &normalized, PlanMode::Default)?;
    let family = selection.chosen.family;
    let plan = validate_plan(&selection.chosen.plan, &normalized)?;
    trace.metadata = QueryTraceMetadata {
        selected_plan_family: format!("{family:?}"),
        node_count: plan.nodes.len(),
        cover_policy: "DynamicMinKeys".to_owned(),
        execution_mode: execution_mode_label(options.execution_mode),
        output_mode: "Materialized".to_owned(),
    };
    if let Some(span) = plan_span {
        trace.finish_span(span, TraceCounters::default());
    }

    let mut sink = ProjectionSink::new(txn);
    let mut stats = ExecutionStats::default();
    let execution_span = trace.start_span(TracePhase::ExecuteNode, "execute Free Join plan");
    execute_validated_plan(
        txn,
        schema,
        &normalized,
        &plan,
        inputs,
        PredicateMode::Pushdown,
        execution_mode_from_public(options.execution_mode),
        CoverPolicy::DynamicMinKeys,
        &mut stats,
        &mut sink,
    )?;
    if let Some(span) = execution_span {
        trace.finish_span(span, TraceCounters::default());
    }

    let sink_span = trace.start_span(TracePhase::SinkFinish, "finish projection sink");
    let result = sink.finish(&normalized)?;
    if let Some(span) = sink_span {
        trace.finish_span(span, TraceCounters::default());
    }
    Ok(ProfiledQueryResult { result, trace })
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
        &InputBindings::new(),
        PredicateMode::Pushdown,
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
    let plan = selected_plan(txn, schema, &normalized, PlanMode::Default)?;
    let mut sink = ProjectionSink::new(txn);
    let mut stats = ExecutionStats::default();
    execute_validated_plan(
        txn,
        schema,
        &normalized,
        &plan,
        &InputBindings::new(),
        PredicateMode::Pushdown,
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
    execute_plan_with_mode_and_predicate_for_test(
        txn,
        schema,
        query,
        plan,
        execution_mode,
        cover_policy,
        PredicateMode::Pushdown,
    )
}

#[cfg(test)]
pub(crate) fn execute_query_with_predicate_mode_for_test(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    inputs: &InputBindings,
    predicate_mode: PredicateMode,
) -> Result<(QueryResultSet, ExecutionStats)> {
    let normalized = normalize_query(schema.descriptor(), query)?;
    validate_supported(&normalized, inputs)?;
    let plan = selected_plan(txn, schema, &normalized, PlanMode::Default)?;
    let mut sink = ProjectionSink::new(txn);
    let mut stats = ExecutionStats::default();
    execute_validated_plan(
        txn,
        schema,
        &normalized,
        &plan,
        inputs,
        predicate_mode,
        ExecutionMode::Scalar,
        CoverPolicy::DynamicMinKeys,
        &mut stats,
        &mut sink,
    )?;
    Ok((sink.finish(&normalized)?, stats))
}

#[cfg(test)]
pub(crate) fn execute_plan_with_mode_and_predicate_for_test(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    plan: &FjPlan,
    execution_mode: ExecutionMode,
    cover_policy: CoverPolicy,
    predicate_mode: PredicateMode,
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
        &InputBindings::new(),
        predicate_mode,
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
    let plan = selected_plan(txn, schema, &normalized, PlanMode::Default)?;
    let mut sink = CountingSink::default();
    let mut stats = ExecutionStats::default();
    execute_validated_plan(
        txn,
        schema,
        &normalized,
        &plan,
        &InputBindings::new(),
        PredicateMode::Pushdown,
        ExecutionMode::Scalar,
        CoverPolicy::DynamicMinKeys,
        &mut stats,
        &mut sink,
    )?;
    Ok(sink.count)
}

#[cfg(test)]
pub(crate) fn execute_query_with_plan_mode_for_test(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    plan_mode: PlanMode,
) -> Result<QueryResultSet> {
    let normalized = normalize_query(schema.descriptor(), query)?;
    validate_supported(&normalized, &InputBindings::new())?;
    let plan = selected_plan(txn, schema, &normalized, plan_mode)?;
    let mut sink = ProjectionSink::new(txn);
    let mut stats = ExecutionStats::default();
    execute_validated_plan(
        txn,
        schema,
        &normalized,
        &plan,
        &InputBindings::new(),
        PredicateMode::Pushdown,
        ExecutionMode::Scalar,
        CoverPolicy::DynamicMinKeys,
        &mut stats,
        &mut sink,
    )?;
    sink.finish(&normalized)
}

#[cfg(test)]
pub(crate) fn execute_query_with_output_mode_for_test(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
    output_mode: OutputMode,
) -> Result<(QueryResultSet, OutputStats)> {
    let normalized = normalize_query(schema.descriptor(), query)?;
    validate_supported(&normalized, &InputBindings::new())?;
    let plan = selected_plan(txn, schema, &normalized, PlanMode::Default)?;
    let mut execution_stats = ExecutionStats::default();
    match output_mode {
        OutputMode::Materialized => {
            let mut sink = ProjectionSink::new(txn);
            execute_validated_plan(
                txn,
                schema,
                &normalized,
                &plan,
                &InputBindings::new(),
                PredicateMode::Pushdown,
                ExecutionMode::Scalar,
                CoverPolicy::DynamicMinKeys,
                &mut execution_stats,
                &mut sink,
            )?;
            sink.finish_with_stats(&normalized)
        }
        OutputMode::Factorized => {
            let mut sink = FactorizedProjectionSink::new(txn);
            execute_validated_plan(
                txn,
                schema,
                &normalized,
                &plan,
                &InputBindings::new(),
                PredicateMode::Pushdown,
                ExecutionMode::Scalar,
                CoverPolicy::DynamicMinKeys,
                &mut execution_stats,
                &mut sink,
            )?;
            sink.finish(&normalized)
        }
    }
}

fn validate_plan(plan: &FjPlan, query: &NormalizedQuery) -> Result<ValidatedFjPlan> {
    plan.validate(query)
        .map_err(|error| Error::invalid_query(error.to_string()))
}

#[cfg(test)]
fn selected_plan(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    mode: PlanMode,
) -> Result<ValidatedFjPlan> {
    let plan = select_plan(txn, schema, query, mode)?.chosen.plan;
    validate_plan(&plan, query)
}

fn execution_mode_from_public(mode: ExecutionModePublic) -> ExecutionMode {
    match mode {
        ExecutionModePublic::Scalar => ExecutionMode::Scalar,
        ExecutionModePublic::Vectorized { batch_size } => ExecutionMode::Vectorized { batch_size },
    }
}

fn execution_mode_label(mode: ExecutionModePublic) -> String {
    match mode {
        ExecutionModePublic::Scalar => "Scalar".to_owned(),
        ExecutionModePublic::Vectorized { batch_size } => {
            format!("Vectorized(batch_size={batch_size})")
        }
    }
}

fn validate_supported(query: &NormalizedQuery, inputs: &InputBindings) -> Result<()> {
    for input in &query.inputs {
        if inputs.value(&input.name).is_none() {
            return Err(Error::invalid_query(format!(
                "missing input {}",
                input.name
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "profiled_tests.rs"]
mod profiled_tests;

#[cfg(test)]
#[path = "vectorized_tests.rs"]
mod vectorized_tests;

#[cfg(test)]
#[path = "predicate_tests.rs"]
mod predicate_tests;

#[cfg(test)]
#[path = "factorized_output_tests.rs"]
mod factorized_output_tests;
