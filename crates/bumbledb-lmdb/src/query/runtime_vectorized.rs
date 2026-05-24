use std::collections::BTreeMap;

use crate::colt::ColtSource;
use crate::query::cover::{CoverPolicy, ExecutionMode, ExecutionStats};
use crate::query::free_join::{FjSubatom, ValidatedFjNode, ValidatedFjPlan};
use crate::query::model::{AtomOccurrenceId, NormalizedQuery};
use crate::query::predicate::PredicateMode;
use crate::query::runtime::{Candidate, bind_cover_tuple, execute_node, source_for};
use crate::query::runtime_keys::key_from_binding_placeholder;
use crate::query::sink::{Binding, BindingSink};
use crate::query::trace::{QueryTrace, TraceCounters};
use crate::tuple::GhtSource;
use crate::{ReadTxn, Result, StorageSchema};

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_vectorized_cover_loop<S: BindingSink>(
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
    batch_size: usize,
    cover_policy: CoverPolicy,
    stats: &mut ExecutionStats,
    sink: &mut S,
    trace: &mut QueryTrace,
) -> Result<()> {
    let batch_size = batch_size.max(1);
    stats.vectorized.batch_size = batch_size;
    for batch in cover_source.iter_batch(batch_size) {
        if batch.is_empty() {
            continue;
        }
        stats.vectorized.batches += 1;
        stats.vectorized.input_tuples += batch.len();
        trace.add_counters(&TraceCounters {
            batches_yielded: 1,
            tuples_yielded: batch.len() as u64,
            ..TraceCounters::default()
        });
        let survivors = bind_vectorized_batch(
            query,
            sources,
            binding,
            cover_source,
            cover_subatom,
            batch,
            stats,
            trace,
        )?;
        let survivors =
            probe_vectorized_survivors(node_index, node, cover_index, survivors, stats, trace)?;
        stats.vectorized.survivor_tuples += survivors.len();
        for (next_binding, next_sources) in survivors {
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
                ExecutionMode::Vectorized { batch_size },
                cover_policy,
                stats,
                sink,
                trace,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn bind_vectorized_batch(
    query: &NormalizedQuery,
    sources: &BTreeMap<AtomOccurrenceId, ColtSource>,
    binding: &Binding,
    cover_source: &ColtSource,
    cover_subatom: &FjSubatom,
    batch: Vec<crate::tuple::EncodedTuple>,
    stats: &mut ExecutionStats,
    trace: &mut QueryTrace,
) -> Result<Vec<Candidate>> {
    let mut survivors = Vec::new();
    for tuple in batch {
        if let Some(entry) = bind_cover_tuple(
            query,
            sources,
            binding,
            cover_source,
            cover_subatom,
            &tuple,
            trace,
        )? {
            survivors.push(entry);
        } else {
            stats.vectorized.failed_tuples += 1;
        }
    }
    Ok(survivors)
}

fn probe_vectorized_survivors(
    node_index: usize,
    node: &ValidatedFjNode,
    cover_index: usize,
    mut survivors: Vec<Candidate>,
    stats: &mut ExecutionStats,
    trace: &mut QueryTrace,
) -> Result<Vec<Candidate>> {
    for (index, subatom) in node.subatoms.iter().enumerate() {
        if index == cover_index || survivors.is_empty() {
            continue;
        }
        let mut next_survivors = Vec::new();
        for (candidate_binding, mut candidate_sources) in survivors {
            stats.vectorized.probe_calls += 1;
            let source = source_for(&candidate_sources, subatom)?;
            let key = key_from_binding_placeholder(&candidate_binding, subatom)?;
            if let Some(child) = source.get_traced(
                &key,
                trace,
                format!("vector probe node={node_index} atom={:?}", subatom.atom),
            ) {
                candidate_sources.insert(subatom.atom, child);
                next_survivors.push((candidate_binding, candidate_sources));
            } else {
                stats.vectorized.failed_tuples += 1;
            }
        }
        survivors = next_survivors;
    }
    Ok(survivors)
}
