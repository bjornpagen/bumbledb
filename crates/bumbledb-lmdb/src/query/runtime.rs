use std::collections::BTreeMap;
use std::ops::ControlFlow;

use crate::colt::ColtSource;
use crate::query::cover::{CoverPolicy, ExecutionMode, ExecutionStats, choose_cover};
use crate::query::free_join::{FjSubatom, ValidatedFjNode, ValidatedFjPlan};
use crate::query::model::{AtomOccurrenceId, NormalizedQuery};
use crate::query::predicate::{self, PredicateMode};
use crate::query::runtime_frame::{
    SourceUndo, finish_binding_span, finish_probe_span, replace_source, restore_sources, source_for,
};
use crate::query::sink::{Binding, BindingSink, BindingUndo};
use crate::query::trace::{QueryTrace, TraceCounters, TracePhase};
use crate::tuple::{EncodedTupleRef, GhtSource};
use crate::{ReadTxn, Result, StorageSchema};

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
    let mut sources = super::source_build::build_sources(
        txn,
        schema,
        query,
        plan,
        inputs,
        predicate_mode,
        trace,
    )?;
    let mut binding = Binding::new(query.variables.len());
    let mut binding_undo = Vec::new();
    let mut source_undo = Vec::new();
    execute_node(
        0,
        0,
        query,
        plan,
        &mut sources,
        &mut binding,
        &mut binding_undo,
        &mut source_undo,
        txn,
        schema,
        inputs,
        predicate_mode,
        execution_mode,
        cover_policy,
        stats,
        sink,
        trace,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_node<S: BindingSink>(
    node_index: usize,
    depth: usize,
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    sources: &mut BTreeMap<AtomOccurrenceId, ColtSource>,
    binding: &mut Binding,
    binding_undo: &mut Vec<BindingUndo>,
    source_undo: &mut Vec<SourceUndo>,
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    inputs: &crate::InputBindings,
    predicate_mode: PredicateMode,
    execution_mode: ExecutionMode,
    cover_policy: CoverPolicy,
    stats: &mut ExecutionStats,
    sink: &mut S,
    trace: &mut QueryTrace,
) -> Result<()> {
    let node_span = trace.start_span(TracePhase::ExecuteNode, format!("node={node_index}"));
    let Some(node) = plan.nodes.get(node_index) else {
        let result = consume_terminal_binding(query, binding, txn, schema, inputs, sink, trace);
        finish_node_span(trace, node_span, depth);
        return result;
    };
    let cover_span = trace.start_span(TracePhase::CoverChoice, format!("node={node_index}"));
    let cover_index = choose_cover(node, sources, cover_policy, stats)?;
    if let Some(span) = cover_span {
        trace.finish_span(
            span,
            TraceCounters {
                cover_choices: 1,
                ..TraceCounters::default()
            },
        );
    }
    let cover_subatom = &node.subatoms[cover_index];
    let cover_source = source_for(sources, cover_subatom)?;

    let result = if node.new_vars.is_empty() {
        execute_bound_cover(
            node_index,
            depth,
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
            node,
            cover_index,
            execution_mode,
            cover_policy,
            stats,
            sink,
            trace,
        )
    } else {
        match execution_mode {
            ExecutionMode::Scalar => execute_scalar_cover_loop(
                node_index,
                depth,
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
                node,
                cover_index,
                &cover_source,
                cover_subatom,
                cover_policy,
                stats,
                sink,
                trace,
            ),
            ExecutionMode::Vectorized { batch_size } => {
                super::runtime_vectorized::execute_vectorized_cover_loop(
                    node_index,
                    depth,
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
                    node,
                    cover_index,
                    &cover_source,
                    cover_subatom,
                    batch_size,
                    cover_policy,
                    stats,
                    sink,
                    trace,
                )
            }
        }
    };
    finish_node_span(trace, node_span, depth);
    result.map(|_| ())
}

fn consume_terminal_binding<S: BindingSink>(
    query: &NormalizedQuery,
    binding: &Binding,
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    inputs: &crate::InputBindings,
    sink: &mut S,
    trace: &mut QueryTrace,
) -> Result<()> {
    if !predicate::binding_satisfies(txn, schema.descriptor(), query, binding, inputs)? {
        return Ok(());
    }
    let sink_span = trace.start_span(TracePhase::SinkConsume, "consume projection binding");
    let result = sink.consume(query, binding);
    if let Some(span) = sink_span {
        let inserted = result.as_ref().is_ok_and(|stats| stats.inserted);
        trace.finish_span(
            span,
            TraceCounters {
                sink_consumes: usize::from(result.is_ok()) as u64,
                projection_duplicates_suppressed: u64::from(result.is_ok() && !inserted),
                ..TraceCounters::default()
            },
        );
    }
    result.map(|_| ())
}

fn finish_node_span(
    trace: &mut QueryTrace,
    span: Option<crate::query::trace::TraceSpanId>,
    depth: usize,
) {
    if let Some(span) = span {
        trace.finish_span(
            span,
            TraceCounters {
                recursive_node_entries: 1,
                max_recursion_depth: depth as u64,
                frame_pushes: 1,
                frame_pops: 1,
                ..TraceCounters::default()
            },
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_bound_cover<S: BindingSink>(
    node_index: usize,
    depth: usize,
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    sources: &mut BTreeMap<AtomOccurrenceId, ColtSource>,
    binding: &mut Binding,
    binding_undo: &mut Vec<BindingUndo>,
    source_undo: &mut Vec<SourceUndo>,
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    inputs: &crate::InputBindings,
    predicate_mode: PredicateMode,
    node: &ValidatedFjNode,
    cover_index: usize,
    execution_mode: ExecutionMode,
    cover_policy: CoverPolicy,
    stats: &mut ExecutionStats,
    sink: &mut S,
    trace: &mut QueryTrace,
) -> Result<()> {
    let cover_subatom = &node.subatoms[cover_index];
    let cover_source = source_for(sources, cover_subatom)?;
    let source_mark = source_undo.len();
    if cover_subatom.vars.is_empty() && !cover_source.has_child_level() {
        if cover_source.is_empty() {
            return Ok(());
        }
    } else {
        let key = super::runtime_keys::key_from_binding(query, binding, cover_subatom)?;
        let Some(child) = cover_source.get_traced(
            key.as_ref(),
            trace,
            format!("cover get node={node_index} atom={:?}", cover_subatom.atom),
        ) else {
            return Ok(());
        };
        replace_source(sources, cover_subatom.atom, child, source_undo)?;
        trace.add_counters(&TraceCounters {
            source_replacements: 1,
            source_frame_changes: 1,
            ..TraceCounters::default()
        });
    }
    let result = if probe_siblings(
        query,
        node,
        cover_index,
        binding,
        sources,
        source_undo,
        trace,
    )? {
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
            execution_mode,
            cover_policy,
            stats,
            sink,
            trace,
        )
    } else {
        Ok(())
    };
    restore_sources(sources, source_undo, source_mark);
    result
}

#[allow(clippy::too_many_arguments)]
fn execute_scalar_cover_loop<S: BindingSink>(
    node_index: usize,
    depth: usize,
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    sources: &mut BTreeMap<AtomOccurrenceId, ColtSource>,
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
    cover_subatom: &FjSubatom,
    cover_policy: CoverPolicy,
    stats: &mut ExecutionStats,
    sink: &mut S,
    trace: &mut QueryTrace,
) -> Result<()> {
    cover_source.try_for_each_tuple_traced(
        trace,
        format!("cover iter node={node_index} atom={:?}", cover_subatom.atom),
        |tuple, trace| {
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
            if !bound {
                restore_sources(sources, source_undo, source_mark);
                binding.undo_to(binding_undo, binding_mark);
                return Ok(ControlFlow::Continue(()));
            }
            let result = if probe_siblings(
                query,
                node,
                cover_index,
                binding,
                sources,
                source_undo,
                trace,
            )? {
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
                    ExecutionMode::Scalar,
                    cover_policy,
                    stats,
                    sink,
                    trace,
                )
            } else {
                Ok(())
            };
            restore_sources(sources, source_undo, source_mark);
            binding.undo_to(binding_undo, binding_mark);
            result?;
            Ok(ControlFlow::Continue(()))
        },
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn bind_cover_tuple(
    query: &NormalizedQuery,
    sources: &mut BTreeMap<AtomOccurrenceId, ColtSource>,
    binding: &mut Binding,
    binding_undo: &mut Vec<BindingUndo>,
    source_undo: &mut Vec<SourceUndo>,
    cover_source: &ColtSource,
    cover_subatom: &FjSubatom,
    tuple: EncodedTupleRef<'_>,
    trace: &mut QueryTrace,
) -> Result<bool> {
    let span = trace.start_span(
        TracePhase::BindingExtend,
        format!("cover atom={:?}", cover_subatom.atom),
    );
    let extend = binding.extend_from_tuple(cover_source.vars(), tuple, query, binding_undo)?;
    if !extend.accepted {
        finish_binding_span(trace, span, extend.writes, extend.conflicts, 0);
        return Ok(false);
    }
    let mut source_replacements = 0;
    if cover_source.has_child_level() {
        let Some(child) = cover_source.get_traced(
            tuple,
            trace,
            format!("cover child atom={:?}", cover_subatom.atom),
        ) else {
            finish_binding_span(
                trace,
                span,
                extend.writes,
                extend.conflicts,
                source_replacements,
            );
            return Ok(false);
        };
        replace_source(sources, cover_subatom.atom, child, source_undo)?;
        source_replacements += 1;
    }
    finish_binding_span(
        trace,
        span,
        extend.writes,
        extend.conflicts,
        source_replacements,
    );
    Ok(true)
}

fn probe_siblings(
    query: &NormalizedQuery,
    node: &ValidatedFjNode,
    cover_index: usize,
    binding: &Binding,
    sources: &mut BTreeMap<AtomOccurrenceId, ColtSource>,
    source_undo: &mut Vec<SourceUndo>,
    trace: &mut QueryTrace,
) -> Result<bool> {
    for (index, subatom) in node.subatoms.iter().enumerate() {
        if index == cover_index {
            continue;
        }
        let span = trace.start_span(
            TracePhase::ProbeSibling,
            format!("node={} atom={:?}", node.id, subatom.atom),
        );
        let source = source_for(sources, subatom)?;
        let key = super::runtime_keys::key_from_binding(query, binding, subatom)?;
        let Some(child) = source.get_traced(
            key.as_ref(),
            trace,
            format!("sibling get node={} atom={:?}", node.id, subatom.atom),
        ) else {
            finish_probe_span(trace, span, true);
            return Ok(false);
        };
        replace_source(sources, subatom.atom, child, source_undo)?;
        finish_probe_span(trace, span, false);
    }
    Ok(true)
}
