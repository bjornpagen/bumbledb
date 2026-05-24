use crate::colt::{ColtSource, KeyScratch};
use crate::query::cover::{CoverPolicy, ExecutionMode, ExecutionStats};
use crate::query::free_join::{ValidatedFjNode, ValidatedFjPlan, ValidatedFjSubatom};
use crate::query::model::NormalizedQuery;
use crate::query::predicate::PredicateMode;
use crate::query::runtime::{bind_cover_tuple, execute_node};
use crate::query::runtime_frame::{
    SourceStore, SourceUndo, replace_source, restore_sources, source_for,
};
use crate::query::runtime_keys::key_from_binding_by_bound_widths_with_scratch;
use crate::query::sink::{Binding, BindingSink, BindingUndo};
use crate::query::trace::QueryTrace;
use crate::tuple::TupleCursor;
use crate::{ReadTxn, Result, StorageSchema};

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_vectorized_cover_loop<S: BindingSink>(
    node_index: usize,
    depth: usize,
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    sources: &mut SourceStore,
    binding: &mut Binding,
    binding_undo: &mut Vec<BindingUndo>,
    source_undo: &mut Vec<SourceUndo>,
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    inputs: &crate::InputBindings,
    predicate_mode: PredicateMode,
    node: &ValidatedFjNode,
    cover_index: usize,
    cover_source: &ColtSource,
    cover_subatom: &ValidatedFjSubatom,
    batch_size: usize,
    cover_policy: CoverPolicy,
    stats: &mut ExecutionStats,
    sink: &mut S,
    trace: &mut QueryTrace,
) -> Result<()> {
    let batch_size = batch_size.max(1);
    stats.vectorized.batch_size = batch_size;
    let mut cursor = TupleCursor::default();
    loop {
        let batch = cover_source.fill_batch_traced(
            &mut cursor,
            batch_size,
            trace,
            lazy_label("cover batch", node_index, cover_subatom.atom),
        );
        let exhausted = batch.exhausted;
        if batch.is_empty() {
            if exhausted {
                break;
            }
            continue;
        }
        stats.vectorized.batches += 1;
        stats.vectorized.input_tuples += batch.len();
        for tuple in batch.iter() {
            let binding_mark = Binding::undo_mark(binding_undo);
            let source_mark = source_undo.len();
            let bound = bind_cover_tuple(
                query,
                sources,
                binding,
                binding_undo,
                source_undo,
                cover_source,
                cover_subatom,
                tuple,
                trace,
            )?;
            let mut survived = false;
            let result = if bound
                && probe_vectorized_survivor(
                    node_index,
                    plan,
                    node,
                    cover_index,
                    binding,
                    sources,
                    source_undo,
                    stats,
                    trace,
                )? {
                survived = true;
                execute_node(
                    node_index + 1,
                    depth + 1,
                    query,
                    plan,
                    sources,
                    binding,
                    binding_undo,
                    source_undo,
                    txn,
                    schema,
                    inputs,
                    predicate_mode,
                    ExecutionMode::Vectorized { batch_size },
                    cover_policy,
                    stats,
                    sink,
                    trace,
                )
            } else {
                Ok(())
            };
            if survived {
                stats.vectorized.survivor_tuples += 1;
            } else {
                stats.vectorized.failed_tuples += 1;
            }
            restore_sources(sources, source_undo, source_mark);
            binding.undo_to(binding_undo, binding_mark);
            result?;
        }
        if exhausted {
            break;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn probe_vectorized_survivor(
    node_index: usize,
    plan: &ValidatedFjPlan,
    node: &ValidatedFjNode,
    cover_index: usize,
    binding: &Binding,
    sources: &mut SourceStore,
    source_undo: &mut Vec<SourceUndo>,
    stats: &mut ExecutionStats,
    trace: &mut QueryTrace,
) -> Result<bool> {
    for (index, subatom) in plan.node_subatoms(node).iter().enumerate() {
        if index == cover_index {
            continue;
        }
        stats.vectorized.probe_calls += 1;
        let source = source_for(sources, plan, subatom)?;
        let mut key_scratch = KeyScratch::new();
        let key = key_from_binding_by_bound_widths_with_scratch(
            binding,
            plan.subatom_vars(subatom),
            &mut key_scratch,
        )?;
        if let Some(child) = source.get_traced(
            key,
            trace,
            lazy_label("vector probe", node_index, subatom.atom),
        ) {
            replace_source(sources, subatom.atom, child, source_undo)?;
        } else {
            return Ok(false);
        }
    }
    Ok(true)
}

fn lazy_label(
    prefix: &'static str,
    node: usize,
    atom: crate::query::model::AtomOccurrenceId,
) -> String {
    if crate::query::trace::QUERY_TRACING_ENABLED {
        format!("{prefix} node={node} atom={atom:?}")
    } else {
        String::new()
    }
}
