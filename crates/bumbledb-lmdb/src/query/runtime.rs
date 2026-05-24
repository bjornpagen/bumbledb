use std::collections::BTreeMap;
use std::ops::ControlFlow;

use crate::colt::ColtSource;
use crate::query::cover::{CoverPolicy, ExecutionMode, ExecutionStats, choose_cover};
use crate::query::free_join::{FjSubatom, ValidatedFjNode, ValidatedFjPlan};
use crate::query::model::{AtomOccurrenceId, NormalizedQuery};
use crate::query::predicate::{self, PredicateMode};
use crate::query::sink::{Binding, BindingSink};
use crate::query::trace::{QueryTrace, TraceCounters, TracePhase};
use crate::tuple::{EncodedTupleRef, GhtSource};
use crate::{Error, ReadTxn, Result, StorageSchema};

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
    let sources = super::source_build::build_sources(
        txn,
        schema,
        query,
        plan,
        inputs,
        predicate_mode,
        trace,
    )?;
    execute_node(
        0,
        0,
        query,
        plan,
        &sources,
        &Binding::default(),
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
    sources: &BTreeMap<AtomOccurrenceId, ColtSource>,
    binding: &Binding,
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
    result
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
    if !predicate::binding_satisfies(txn, schema.descriptor(), query, &binding.values, inputs)? {
        return Ok(());
    }
    let sink_span = trace.start_span(TracePhase::SinkConsume, "consume projection binding");
    let result = sink.consume(query, binding);
    if let Some(span) = sink_span {
        trace.finish_span(
            span,
            TraceCounters {
                sink_consumes: usize::from(result.is_ok()) as u64,
                decoded_values: query.find.len() as u64,
                ..TraceCounters::default()
            },
        );
    }
    result
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
    sources: &BTreeMap<AtomOccurrenceId, ColtSource>,
    binding: &Binding,
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
    let mut next_sources = sources.clone();
    trace.add_counters(&TraceCounters {
        source_frame_changes: 1,
        ..TraceCounters::default()
    });
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
        next_sources.insert(cover_subatom.atom, child);
    }
    if probe_siblings(query, node, cover_index, binding, &mut next_sources, trace)? {
        execute_node(
            node_index + 1,
            depth + 1,
            query,
            plan,
            &next_sources,
            binding,
            txn,
            schema,
            inputs,
            predicate_mode,
            execution_mode,
            cover_policy,
            stats,
            sink,
            trace,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn execute_scalar_cover_loop<S: BindingSink>(
    node_index: usize,
    depth: usize,
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    sources: &BTreeMap<AtomOccurrenceId, ColtSource>,
    binding: &Binding,
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
            let Some((next_binding, mut next_sources)) = bind_cover_tuple(
                query,
                sources,
                binding,
                cover_source,
                cover_subatom,
                tuple,
                trace,
            )?
            else {
                return Ok(ControlFlow::Continue(()));
            };
            if probe_siblings(
                query,
                node,
                cover_index,
                &next_binding,
                &mut next_sources,
                trace,
            )? {
                execute_node(
                    node_index + 1,
                    depth + 1,
                    query,
                    plan,
                    &next_sources,
                    &next_binding,
                    txn,
                    schema,
                    inputs,
                    predicate_mode,
                    ExecutionMode::Scalar,
                    cover_policy,
                    stats,
                    sink,
                    trace,
                )?;
            }
            Ok(ControlFlow::Continue(()))
        },
    )
}

pub(super) type Candidate = (Binding, BTreeMap<AtomOccurrenceId, ColtSource>);

pub(super) fn bind_cover_tuple(
    query: &NormalizedQuery,
    sources: &BTreeMap<AtomOccurrenceId, ColtSource>,
    binding: &Binding,
    cover_source: &ColtSource,
    cover_subatom: &FjSubatom,
    tuple: EncodedTupleRef<'_>,
    trace: &mut QueryTrace,
) -> Result<Option<(Binding, BTreeMap<AtomOccurrenceId, ColtSource>)>> {
    let span = trace.start_span(
        TracePhase::BindingExtend,
        format!("cover atom={:?}", cover_subatom.atom),
    );
    let Some(next_binding) = binding.extend_from_tuple(cover_source.vars(), tuple, query)? else {
        finish_binding_span(trace, span, 1, 0);
        return Ok(None);
    };
    let mut next_sources = sources.clone();
    let mut source_changes = 1;
    if cover_source.has_child_level() {
        let Some(child) = cover_source.get_traced(
            tuple,
            trace,
            format!("cover child atom={:?}", cover_subatom.atom),
        ) else {
            finish_binding_span(trace, span, 1, source_changes);
            return Ok(None);
        };
        next_sources.insert(cover_subatom.atom, child);
        source_changes += 1;
    }
    finish_binding_span(trace, span, 1, source_changes);
    Ok(Some((next_binding, next_sources)))
}

fn finish_binding_span(
    trace: &mut QueryTrace,
    span: Option<crate::query::trace::TraceSpanId>,
    binding_copies: u64,
    source_frame_changes: u64,
) {
    if let Some(span) = span {
        trace.finish_span(
            span,
            TraceCounters {
                binding_copies,
                source_frame_changes,
                ..TraceCounters::default()
            },
        );
    }
}

fn probe_siblings(
    query: &NormalizedQuery,
    node: &ValidatedFjNode,
    cover_index: usize,
    binding: &Binding,
    sources: &mut BTreeMap<AtomOccurrenceId, ColtSource>,
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
        sources.insert(subatom.atom, child);
        finish_probe_span(trace, span, false);
    }
    Ok(true)
}

fn finish_probe_span(
    trace: &mut QueryTrace,
    span: Option<crate::query::trace::TraceSpanId>,
    missed: bool,
) {
    if let Some(span) = span {
        trace.finish_span(
            span,
            TraceCounters {
                probe_calls: 1,
                probe_misses: u64::from(missed),
                source_frame_changes: u64::from(!missed),
                ..TraceCounters::default()
            },
        );
    }
}

pub(super) fn source_for(
    sources: &BTreeMap<AtomOccurrenceId, ColtSource>,
    subatom: &FjSubatom,
) -> Result<ColtSource> {
    let source = sources
        .get(&subatom.atom)
        .cloned()
        .ok_or_else(|| Error::corrupt(format!("missing source for atom {:?}", subatom.atom)))?;
    if source.atom() != Some(subatom.atom) || source.vars() != subatom.vars.as_slice() {
        return Err(Error::corrupt(format!(
            "source schema mismatch for atom {:?}",
            subatom.atom
        )));
    }
    Ok(source)
}
