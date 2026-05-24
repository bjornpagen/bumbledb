use std::collections::BTreeMap;

use crate::query::binary2fj::{binary2fj, factor_plan};
use crate::query::free_join::{FjNode, FjPlan, FjPlanError, FjSubatom};
use crate::query::model::{AtomOccurrenceId, NormalizedQuery, NormalizedTerm};
use crate::query::planner::{BinaryPlan, deterministic_binary_plan};
use crate::query::trace::{QueryTrace, TraceCounters, TracePhase};
use crate::{Error, ReadTxn, Result, StorageSchema};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PlanFamily {
    FactoredBinary,
    Singleton,
    BinaryDerived,
    InjectedBinary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PlanMode {
    Default,
    ForceBinaryDerived,
    ForceFactoredBinary,
    ForceSingleton,
    InjectedBinary(BinaryPlan),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlanCandidate {
    pub(crate) family: PlanFamily,
    pub(crate) plan: FjPlan,
    pub(crate) cost: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannerSelection {
    pub(crate) chosen: PlanCandidate,
    pub(crate) candidates: Vec<PlanCandidate>,
    pub(crate) stats: PlannerStats,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannerStats {
    pub(crate) storage_tx_id: u64,
    pub(crate) schema_fingerprint: [u8; 32],
    pub(crate) relations: Vec<PlannerRelationStats>,
    pub(crate) skew_ratio: u64,
    pub(crate) projection_width: usize,
    pub(crate) accelerator_entries: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannerRelationStats {
    pub(crate) atom: AtomOccurrenceId,
    pub(crate) relation: String,
    pub(crate) relation_fact_count: u64,
    pub(crate) estimated_source_rows: u64,
    pub(crate) field_distinct_estimate: BTreeMap<usize, u64>,
    pub(crate) prefix_distinct_estimate: Vec<PrefixDistinctEstimate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PrefixDistinctEstimate {
    pub(crate) fields: Vec<usize>,
    pub(crate) distinct_estimate: u64,
}

pub(crate) fn select_plan(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    mode: PlanMode,
) -> Result<PlannerSelection> {
    let mut trace = QueryTrace::new();
    select_plan_with_trace(txn, schema, query, mode, &mut trace)
}

pub(crate) fn select_plan_with_trace(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    mode: PlanMode,
    trace: &mut QueryTrace,
) -> Result<PlannerSelection> {
    let stats = collect_planner_stats_with_trace(txn, schema, query, trace)?;
    let candidate_span = trace.start_span(TracePhase::PlanSelect, "generate plan candidates");
    let mut candidates = candidates_for_mode(query, mode, trace)?;
    if let Some(span) = candidate_span {
        trace.finish_span(span, TraceCounters::default());
    }
    let scoring_span = trace.start_span(TracePhase::PlanSelect, "score plan candidates");
    for candidate in &mut candidates {
        candidate.cost = score_candidate(candidate, &stats);
    }
    candidates.sort_by_key(|candidate| (candidate.cost, candidate.family));
    if let Some(span) = scoring_span {
        trace.finish_span(span, TraceCounters::default());
    }
    let chosen = candidates
        .first()
        .cloned()
        .ok_or_else(|| Error::invalid_query("planner produced no valid candidates"))?;
    Ok(PlannerSelection {
        chosen,
        candidates,
        stats,
    })
}

pub(crate) fn generate_plan_candidates(query: &NormalizedQuery) -> Result<Vec<PlanCandidate>> {
    let mut trace = QueryTrace::new();
    generate_plan_candidates_with_trace(query, &mut trace)
}

pub(crate) fn generate_plan_candidates_with_trace(
    query: &NormalizedQuery,
    trace: &mut QueryTrace,
) -> Result<Vec<PlanCandidate>> {
    let binary = deterministic_binary_plan(query).map_err(invalid_plan)?;
    binary.validate(query).map_err(invalid_plan)?;
    let binary_fj = binary2fj(query, &binary).map_err(invalid_plan)?;
    let (factored, rewrite_trace) = factor_plan(query, &binary_fj).map_err(invalid_plan)?;
    for step in rewrite_trace.steps {
        let span = crate::query_trace_span!(
            trace,
            TracePhase::PlanSelect,
            "factorization from={} to={} outcome={:?} reason={}",
            step.from_node,
            step.to_node,
            step.outcome,
            step.reason
        );
        if let Some(span) = span {
            trace.finish_span(span, TraceCounters::default());
        }
    }
    let singleton = singleton_plan(query).map_err(invalid_plan)?;
    Ok(vec![
        candidate(PlanFamily::BinaryDerived, binary_fj, query)?,
        candidate(PlanFamily::FactoredBinary, factored, query)?,
        candidate(PlanFamily::Singleton, singleton, query)?,
    ])
}

fn candidates_for_mode(
    query: &NormalizedQuery,
    mode: PlanMode,
    trace: &mut QueryTrace,
) -> Result<Vec<PlanCandidate>> {
    let generated = generate_plan_candidates_with_trace(query, trace)?;
    Ok(match mode {
        PlanMode::Default => generated,
        PlanMode::ForceBinaryDerived => only_family(generated, PlanFamily::BinaryDerived),
        PlanMode::ForceFactoredBinary => only_family(generated, PlanFamily::FactoredBinary),
        PlanMode::ForceSingleton => only_family(generated, PlanFamily::Singleton),
        PlanMode::InjectedBinary(binary) => injected_candidates(query, binary, generated)?,
    })
}

fn only_family(candidates: Vec<PlanCandidate>, family: PlanFamily) -> Vec<PlanCandidate> {
    candidates
        .into_iter()
        .filter(|candidate| candidate.family == family)
        .collect()
}

fn injected_candidates(
    query: &NormalizedQuery,
    binary: BinaryPlan,
    generated: Vec<PlanCandidate>,
) -> Result<Vec<PlanCandidate>> {
    let mut candidates = Vec::new();
    if binary.validate(query).is_ok()
        && let Ok(plan) = binary2fj(query, &binary)
    {
        candidates.push(candidate(PlanFamily::InjectedBinary, plan, query)?);
    }
    candidates.extend(generated);
    Ok(candidates)
}

fn candidate(family: PlanFamily, plan: FjPlan, query: &NormalizedQuery) -> Result<PlanCandidate> {
    plan.validate(query).map_err(invalid_plan)?;
    Ok(PlanCandidate {
        family,
        plan,
        cost: 0,
    })
}

fn singleton_plan(query: &NormalizedQuery) -> std::result::Result<FjPlan, FjPlanError> {
    let mut nodes = Vec::new();
    for atom in &query.atoms {
        if atom.variable_tuple.is_empty() {
            nodes.push(FjNode {
                id: nodes.len(),
                subatoms: vec![FjSubatom {
                    atom: atom.id,
                    vars: Vec::new(),
                    field_ids: Vec::new(),
                }],
            });
        }
    }
    for variable in 0..query.variables.len() {
        let subatoms: Vec<_> = query
            .atoms
            .iter()
            .filter_map(|atom| {
                field_id_for_variable(query, atom.id, variable).map(|field_id| (atom.id, field_id))
            })
            .map(|(atom, field_id)| FjSubatom {
                atom,
                vars: vec![variable],
                field_ids: vec![field_id],
            })
            .collect();
        if !subatoms.is_empty() {
            nodes.push(FjNode {
                id: nodes.len(),
                subatoms,
            });
        }
    }
    let plan = FjPlan {
        nodes,
        query_variables: query.variables.len(),
    };
    plan.validate(query)?;
    Ok(plan)
}

fn field_id_for_variable(
    query: &NormalizedQuery,
    atom: AtomOccurrenceId,
    variable: usize,
) -> Option<usize> {
    query.atoms[atom.0]
        .fields
        .iter()
        .find_map(|field| match field.term {
            NormalizedTerm::Variable(bound) if bound == variable => Some(field.field_id),
            _ => None,
        })
}

fn collect_planner_stats(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
) -> Result<PlannerStats> {
    let mut trace = QueryTrace::new();
    collect_planner_stats_with_trace(txn, schema, query, &mut trace)
}

fn collect_planner_stats_with_trace(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    trace: &mut QueryTrace,
) -> Result<PlannerStats> {
    let stats_span = trace.start_span(TracePhase::PlannerStats, "collect planner stats");
    let mut relations = Vec::new();
    for atom in &query.atoms {
        let relation_span = crate::query_trace_span!(
            trace,
            TracePhase::PlannerStats,
            "planner relation={} atom={:?}",
            atom.relation,
            atom.id
        );
        let field_ids = atom
            .fields
            .iter()
            .filter(|field| matches!(field.term, NormalizedTerm::Variable(_)))
            .map(|field| field.field_id)
            .collect::<Vec<_>>();
        let relation_fact_count = txn.relation_fact_count(schema, &atom.relation)?;
        relations.push(relation_stats(
            atom.id,
            &atom.relation,
            relation_fact_count,
            &field_ids,
            &atom.variable_tuple,
        ));
        if let Some(span) = relation_span {
            trace.finish_span(span, TraceCounters::default());
        }
    }
    let max_rows = relations
        .iter()
        .map(|r| r.estimated_source_rows)
        .max()
        .unwrap_or(0);
    let min_rows = relations
        .iter()
        .map(|r| r.estimated_source_rows)
        .filter(|rows| *rows > 0)
        .min()
        .unwrap_or(1);
    let stats = PlannerStats {
        storage_tx_id: txn.storage_tx_id()?,
        schema_fingerprint: schema.descriptor().fingerprint().0,
        relations,
        skew_ratio: max_rows / min_rows.max(1),
        projection_width: query.find.len(),
        accelerator_entries: 0,
    };
    if let Some(span) = stats_span {
        trace.finish_span(span, TraceCounters::default());
    }
    Ok(stats)
}

fn relation_stats(
    atom: AtomOccurrenceId,
    relation: &str,
    relation_fact_count: u64,
    field_ids: &[usize],
    variable_tuple: &[usize],
) -> PlannerRelationStats {
    let estimated_source_rows = relation_fact_count;
    let field_distinct_estimate = field_ids
        .iter()
        .copied()
        .map(|field_id| {
            (
                field_id,
                conservative_distinct_estimate(relation_fact_count),
            )
        })
        .collect();
    PlannerRelationStats {
        atom,
        relation: relation.to_owned(),
        relation_fact_count,
        estimated_source_rows,
        field_distinct_estimate,
        prefix_distinct_estimate: prefix_distinct_estimate(
            field_ids,
            variable_tuple,
            relation_fact_count,
        ),
    }
}

fn prefix_distinct_estimate(
    field_ids: &[usize],
    variable_tuple: &[usize],
    relation_fact_count: u64,
) -> Vec<PrefixDistinctEstimate> {
    (1..=variable_tuple.len().min(field_ids.len()))
        .map(|width| PrefixDistinctEstimate {
            fields: field_ids.iter().copied().take(width).collect(),
            distinct_estimate: conservative_distinct_estimate(relation_fact_count),
        })
        .collect()
}

fn conservative_distinct_estimate(row_count: u64) -> u64 {
    row_count.max(1)
}

fn score_candidate(candidate: &PlanCandidate, stats: &PlannerStats) -> u64 {
    let rows: u64 = stats
        .relations
        .iter()
        .map(|relation| relation.estimated_source_rows)
        .sum();
    let base = match candidate.family {
        PlanFamily::FactoredBinary => 100,
        PlanFamily::Singleton => 200,
        PlanFamily::BinaryDerived => 1_000,
        PlanFamily::InjectedBinary => 300,
    };
    base + rows + candidate.plan.nodes.len() as u64 + stats.projection_width as u64
}

fn invalid_plan(error: impl std::fmt::Display) -> Error {
    Error::invalid_query(error.to_string())
}
