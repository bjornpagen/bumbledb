use std::collections::BTreeMap;

use crate::base_image::field_scope_for_plan;
use crate::colt::{ColtSource, tuple_schemas_for_atom};
use crate::query::cover::{CoverPolicy, ExecutionMode, ExecutionStats, choose_cover};
use crate::query::free_join::{FjSubatom, ValidatedFjNode, ValidatedFjPlan};
use crate::query::model::{AtomOccurrenceId, NormalizedQuery};
use crate::query::predicate::{self, PredicateMode};
use crate::query::sink::{Binding, BindingSink};
use crate::tuple::{EncodedTuple, GhtSource, TupleError, TupleField, TupleSchema};
use crate::{Error, ReadTxn, Result, StorageSchema};

#[allow(clippy::too_many_arguments)]
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
    let sources = build_sources(txn, schema, query, plan, inputs, predicate_mode)?;
    execute_node(
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
    )
}

fn build_sources(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    inputs: &crate::InputBindings,
    predicate_mode: PredicateMode,
) -> Result<BTreeMap<AtomOccurrenceId, ColtSource>> {
    let mut scopes = field_scope_for_plan(plan);
    let mut sources = BTreeMap::new();
    for atom in &query.atoms {
        let filters = predicate::source_filters_for_atom(
            txn,
            schema.descriptor(),
            query,
            atom,
            inputs,
            predicate_mode,
        )?;
        scopes.entry(atom.id).or_default().extend(
            filters
                .iter()
                .filter_map(crate::colt::SourceFilter::field_id),
        );
        let field_ids = scopes.get(&atom.id).into_iter().flatten().copied();
        let image = txn.relation_base_image(schema, &atom.relation, field_ids)?;
        let tuple_schemas = tuple_schemas_for_atom(query, plan, atom.id);
        if tuple_schemas.is_empty() {
            return Err(Error::invalid_query(format!(
                "atom occurrence {:?} has no Free Join source schema",
                atom.id
            )));
        }
        sources.insert(
            atom.id,
            ColtSource::new_filtered(atom.id, image, tuple_schemas, filters),
        );
    }
    Ok(sources)
}

#[allow(clippy::too_many_arguments)]
fn execute_node<S: BindingSink>(
    node_index: usize,
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
) -> Result<()> {
    let Some(node) = plan.nodes.get(node_index) else {
        if predicate::binding_satisfies(txn, schema.descriptor(), query, &binding.values, inputs)? {
            return sink.consume(query, binding);
        }
        return Ok(());
    };
    let cover_index = choose_cover(node, sources, cover_policy, stats)?;
    let cover_subatom = &node.subatoms[cover_index];
    let cover_source = source_for(sources, cover_subatom)?;

    if node.new_vars.is_empty() {
        return execute_bound_cover(
            node_index,
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
        );
    }

    match execution_mode {
        ExecutionMode::Scalar => execute_scalar_cover_loop(
            node_index,
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
        ),
        ExecutionMode::Vectorized { batch_size } => execute_vectorized_cover_loop(
            node_index,
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
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_bound_cover<S: BindingSink>(
    node_index: usize,
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
) -> Result<()> {
    let cover_subatom = &node.subatoms[cover_index];
    let cover_source = source_for(sources, cover_subatom)?;
    let mut next_sources = sources.clone();
    if cover_subatom.vars.is_empty() && !cover_source.has_child_level() {
        if cover_source.is_empty() {
            return Ok(());
        }
    } else {
        let key = key_from_binding(query, binding, cover_subatom)?;
        let Some(child) = cover_source.get(&key) else {
            return Ok(());
        };
        next_sources.insert(cover_subatom.atom, child);
    }
    if probe_siblings(query, node, cover_index, binding, &mut next_sources)? {
        execute_node(
            node_index + 1,
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
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn execute_scalar_cover_loop<S: BindingSink>(
    node_index: usize,
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
) -> Result<()> {
    for tuple in cover_source.iter() {
        let Some((next_binding, mut next_sources)) =
            bind_cover_tuple(query, sources, binding, cover_source, cover_subatom, &tuple)?
        else {
            continue;
        };
        if probe_siblings(query, node, cover_index, &next_binding, &mut next_sources)? {
            execute_node(
                node_index + 1,
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
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn execute_vectorized_cover_loop<S: BindingSink>(
    node_index: usize,
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
) -> Result<()> {
    let batch_size = batch_size.max(1);
    stats.vectorized.batch_size = batch_size;
    for batch in cover_source.iter_batch(batch_size) {
        if batch.is_empty() {
            continue;
        }
        stats.vectorized.batches += 1;
        stats.vectorized.input_tuples += batch.len();
        let mut survivors = Vec::new();
        for tuple in batch {
            if let Some(entry) =
                bind_cover_tuple(query, sources, binding, cover_source, cover_subatom, &tuple)?
            {
                survivors.push(entry);
            } else {
                stats.vectorized.failed_tuples += 1;
            }
        }
        for (index, subatom) in node.subatoms.iter().enumerate() {
            if index == cover_index || survivors.is_empty() {
                continue;
            }
            let mut next_survivors = Vec::new();
            for (candidate_binding, mut candidate_sources) in survivors {
                stats.vectorized.probe_calls += 1;
                let source = source_for(&candidate_sources, subatom)?;
                let key = key_from_binding(query, &candidate_binding, subatom)?;
                if let Some(child) = source.get(&key) {
                    candidate_sources.insert(subatom.atom, child);
                    next_survivors.push((candidate_binding, candidate_sources));
                } else {
                    stats.vectorized.failed_tuples += 1;
                }
            }
            survivors = next_survivors;
        }
        stats.vectorized.survivor_tuples += survivors.len();
        for (next_binding, next_sources) in survivors {
            execute_node(
                node_index + 1,
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
            )?;
        }
    }
    Ok(())
}

fn bind_cover_tuple(
    query: &NormalizedQuery,
    sources: &BTreeMap<AtomOccurrenceId, ColtSource>,
    binding: &Binding,
    cover_source: &ColtSource,
    cover_subatom: &FjSubatom,
    tuple: &EncodedTuple,
) -> Result<Option<(Binding, BTreeMap<AtomOccurrenceId, ColtSource>)>> {
    let Some(next_binding) = binding.extend_from_tuple(cover_source.vars(), tuple, query)? else {
        return Ok(None);
    };
    let mut next_sources = sources.clone();
    if cover_source.has_child_level() {
        let Some(child) = cover_source.get(tuple) else {
            return Ok(None);
        };
        next_sources.insert(cover_subatom.atom, child);
    }
    Ok(Some((next_binding, next_sources)))
}

fn probe_siblings(
    query: &NormalizedQuery,
    node: &ValidatedFjNode,
    cover_index: usize,
    binding: &Binding,
    sources: &mut BTreeMap<AtomOccurrenceId, ColtSource>,
) -> Result<bool> {
    for (index, subatom) in node.subatoms.iter().enumerate() {
        if index == cover_index {
            continue;
        }
        let source = source_for(sources, subatom)?;
        let key = key_from_binding(query, binding, subatom)?;
        let Some(child) = source.get(&key) else {
            return Ok(false);
        };
        sources.insert(subatom.atom, child);
    }
    Ok(true)
}

fn source_for(
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

fn key_from_binding(
    query: &NormalizedQuery,
    binding: &Binding,
    subatom: &FjSubatom,
) -> Result<EncodedTuple> {
    let schema = tuple_schema_for_vars(query, &subatom.vars)?;
    schema
        .tuple_from_bindings(&binding.values)
        .map_err(tuple_error)
}

fn tuple_schema_for_vars(query: &NormalizedQuery, vars: &[usize]) -> Result<TupleSchema> {
    let fields = vars
        .iter()
        .map(|variable| {
            let value_type = &query
                .variables
                .get(*variable)
                .ok_or_else(|| Error::invalid_query(format!("unknown variable {variable}")))?
                .value_type;
            TupleField::new(*variable, None, value_type.encoded_width()).map_err(tuple_error)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(TupleSchema::new(fields))
}

fn tuple_error(error: TupleError) -> Error {
    Error::corrupt(error.to_string())
}
