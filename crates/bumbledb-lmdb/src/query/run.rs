use crate::query::cover::{CoverPolicy, ExecutionMode, ExecutionStats};
use crate::query::free_join::ValidatedFjPlan;
use crate::query::model::NormalizedQuery;
use crate::query::predicate::PredicateMode;
use crate::query::sink::BindingSink;
use crate::query::trace::QueryTrace;
use crate::{ReadTxn, Result, StorageSchema};

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(super) fn execute_validated_plan<S: BindingSink>(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    inputs: &crate::InputBindings,
    predicate_mode: PredicateMode,
    execution_mode: ExecutionMode,
    cover_policy: CoverPolicy,
    stats: &mut ExecutionStats,
    sink: &mut S,
) -> Result<()> {
    let mut trace = QueryTrace::disabled();
    execute_validated_plan_with_trace(
        txn,
        schema,
        query,
        plan,
        inputs,
        predicate_mode,
        execution_mode,
        cover_policy,
        stats,
        sink,
        &mut trace,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_validated_plan_with_trace<S: BindingSink>(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    inputs: &crate::InputBindings,
    predicate_mode: PredicateMode,
    execution_mode: ExecutionMode,
    cover_policy: CoverPolicy,
    stats: &mut ExecutionStats,
    sink: &mut S,
    trace: &mut QueryTrace,
) -> Result<()> {
    super::runtime::execute_validated_plan_with_trace(
        txn,
        schema,
        query,
        plan,
        inputs,
        predicate_mode,
        execution_mode,
        cover_policy,
        stats,
        sink,
        trace,
    )
}
