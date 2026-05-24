use std::collections::{BTreeMap, BTreeSet};

use bumbledb_core::query_ir::{TypedFindTerm, TypedQuery};

use crate::base_image::field_scope_for_plan;
use crate::colt::{ColtSource, tuple_schemas_for_atom};
use crate::query::binary2fj::{binary2fj, factor_plan};
use crate::query::cover::{CoverPolicy, ExecutionStats, choose_cover};
use crate::query::free_join::{FjPlan, FjSubatom, ValidatedFjNode, ValidatedFjPlan};
use crate::query::model::{AtomOccurrenceId, NormalizedQuery};
use crate::query::normalize::normalize_query;
use crate::query::planner::deterministic_binary_plan;
use crate::storage_v5;
use crate::tuple::{EncodedTuple, GhtSource, TupleError, TupleField, TupleSchema};
use crate::{
    Error, InputBindings, QueryResultSet, ReadTxn, Result, ResultColumn, ResultFact, StorageSchema,
};

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

fn execute_validated_plan<S: BindingSink>(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    cover_policy: CoverPolicy,
    stats: &mut ExecutionStats,
    sink: &mut S,
) -> Result<()> {
    let sources = build_sources(txn, schema, query, plan)?;
    execute_node(
        0,
        query,
        plan,
        &sources,
        &Binding::default(),
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
) -> Result<BTreeMap<AtomOccurrenceId, ColtSource>> {
    let scopes = field_scope_for_plan(plan);
    let mut sources = BTreeMap::new();
    for atom in &query.atoms {
        let field_ids = scopes.get(&atom.id).into_iter().flatten().copied();
        let image = txn.relation_base_image(schema, &atom.relation, field_ids)?;
        let tuple_schemas = tuple_schemas_for_atom(query, plan, atom.id);
        if tuple_schemas.is_empty() {
            return Err(Error::invalid_query(format!(
                "atom occurrence {:?} has no Free Join source schema",
                atom.id
            )));
        }
        sources.insert(atom.id, ColtSource::new(atom.id, image, tuple_schemas));
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
    cover_policy: CoverPolicy,
    stats: &mut ExecutionStats,
    sink: &mut S,
) -> Result<()> {
    let Some(node) = plan.nodes.get(node_index) else {
        return sink.consume(query, binding);
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
            node,
            cover_index,
            cover_policy,
            stats,
            sink,
        );
    }

    for tuple in cover_source.iter() {
        let Some(next_binding) = binding.extend_from_tuple(cover_source.vars(), &tuple, query)?
        else {
            continue;
        };
        let mut next_sources = sources.clone();
        if cover_source.has_child_level() {
            let Some(child) = cover_source.get(&tuple) else {
                continue;
            };
            next_sources.insert(cover_subatom.atom, child);
        }
        if probe_siblings(query, node, cover_index, &next_binding, &mut next_sources)? {
            execute_node(
                node_index + 1,
                query,
                plan,
                &next_sources,
                &next_binding,
                cover_policy,
                stats,
                sink,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn execute_bound_cover<S: BindingSink>(
    node_index: usize,
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    sources: &BTreeMap<AtomOccurrenceId, ColtSource>,
    binding: &Binding,
    node: &ValidatedFjNode,
    cover_index: usize,
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
            cover_policy,
            stats,
            sink,
        )?;
    }
    Ok(())
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

fn tuple_error(error: TupleError) -> Error {
    Error::corrupt(error.to_string())
}

#[derive(Clone, Debug, Default)]
struct Binding {
    values: BTreeMap<usize, Vec<u8>>,
}

impl Binding {
    fn extend_from_tuple(
        &self,
        vars: &[usize],
        tuple: &EncodedTuple,
        query: &NormalizedQuery,
    ) -> Result<Option<Self>> {
        let mut next = self.clone();
        let mut offset = 0;
        for variable in vars {
            let width = query.variables[*variable].value_type.encoded_width();
            let Some(bytes) = tuple.bytes().get(offset..offset + width) else {
                return Err(Error::corrupt("cover tuple width is too short"));
            };
            match next.values.get(variable) {
                Some(existing) if existing != bytes => return Ok(None),
                Some(_) => {}
                None => {
                    next.values.insert(*variable, bytes.to_vec());
                }
            }
            offset += width;
        }
        if offset != tuple.bytes().len() {
            return Err(Error::corrupt("cover tuple width has trailing bytes"));
        }
        Ok(Some(next))
    }
}

trait BindingSink {
    fn consume(&mut self, query: &NormalizedQuery, binding: &Binding) -> Result<()>;
}

struct ProjectionSink<'txn, 'env> {
    txn: &'txn ReadTxn<'env>,
    facts: BTreeSet<ResultFact>,
}

impl<'txn, 'env> ProjectionSink<'txn, 'env> {
    fn new(txn: &'txn ReadTxn<'env>) -> Self {
        Self {
            txn,
            facts: BTreeSet::new(),
        }
    }

    fn finish(self, query: &NormalizedQuery) -> Result<QueryResultSet> {
        Ok(QueryResultSet::new(
            result_columns(query)?,
            self.facts.into_iter().collect(),
        ))
    }
}

impl BindingSink for ProjectionSink<'_, '_> {
    fn consume(&mut self, query: &NormalizedQuery, binding: &Binding) -> Result<()> {
        let mut fact = Vec::with_capacity(query.find.len());
        for term in &query.find {
            match term {
                TypedFindTerm::Variable { variable } => {
                    let value_type = &query.variables[*variable].value_type;
                    let bytes = binding.values.get(variable).ok_or_else(|| {
                        Error::corrupt(format!("projection variable {variable} is unbound"))
                    })?;
                    fact.push(storage_v5::decode_value(self.txn, value_type, bytes)?);
                }
            }
        }
        self.facts.insert(fact);
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
struct CountingSink {
    count: usize,
}

#[cfg(test)]
impl BindingSink for CountingSink {
    fn consume(&mut self, _query: &NormalizedQuery, _binding: &Binding) -> Result<()> {
        self.count += 1;
        Ok(())
    }
}

fn result_columns(query: &NormalizedQuery) -> Result<Vec<ResultColumn>> {
    query
        .find
        .iter()
        .map(|term| match term {
            TypedFindTerm::Variable { variable } => query
                .variables
                .get(*variable)
                .map(|variable| ResultColumn::Variable(variable.name.clone()))
                .ok_or_else(|| Error::invalid_query(format!("unknown projection {variable}"))),
        })
        .collect()
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;
