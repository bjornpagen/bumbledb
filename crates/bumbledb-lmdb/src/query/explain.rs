#![allow(dead_code)]

use bumbledb_core::query_ir::TypedQuery;

use crate::colt::tuple_schemas_for_atom;
use crate::query::cover::{CoverPolicy, ExecutionMode};
use crate::query::free_join::ValidatedFjPlan;
use crate::query::normalize::normalize_query;
use crate::query::planner::{PlanFamily, PlanMode, PlannerSelection, select_plan};
use crate::query::sink::OutputMode;
use crate::{Error, ReadTxn, Result, StorageSchema};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExplainConfig {
    pub(crate) plan_mode: PlanMode,
    pub(crate) execution_mode: ExecutionMode,
    pub(crate) cover_policy: CoverPolicy,
    pub(crate) output_mode: OutputMode,
}

impl Default for ExplainConfig {
    fn default() -> Self {
        Self {
            plan_mode: PlanMode::Default,
            execution_mode: ExecutionMode::Scalar,
            cover_policy: CoverPolicy::DynamicMinKeys,
            output_mode: OutputMode::Materialized,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct QueryPlan {
    selection: PlannerSelection,
    validated: ValidatedFjPlan,
    config: ExplainConfig,
    ght_schemas: Vec<String>,
}

impl QueryPlan {
    pub(crate) fn build(
        txn: &ReadTxn<'_>,
        schema: &StorageSchema,
        query: &TypedQuery,
        config: ExplainConfig,
    ) -> Result<Self> {
        let normalized = normalize_query(schema.descriptor(), query)?;
        let selection = select_plan(txn, schema, &normalized, config.plan_mode.clone())?;
        let validated = selection
            .chosen
            .plan
            .validate(&normalized)
            .map_err(|error| Error::invalid_query(error.to_string()))?;
        let ght_schemas = normalized
            .atoms
            .iter()
            .map(|atom| {
                let schemas = tuple_schemas_for_atom(&normalized, &validated, atom.id)
                    .into_iter()
                    .map(|schema| format!("{:?}", schema.vars()))
                    .collect::<Vec<_>>()
                    .join(" -> ");
                format!(
                    "atom {:?} relation {} GHT schema [{}]",
                    atom.id, atom.relation, schemas
                )
            })
            .collect();
        Ok(Self {
            selection,
            validated,
            config,
            ght_schemas,
        })
    }

    pub(crate) fn explain(&self) -> String {
        let mut out = Vec::new();
        out.push("query execution mode: formal Free Join".to_owned());
        out.push(format!(
            "plan mode: {}",
            plan_family_label(self.selection.chosen.family)
        ));
        out.push(format!("cover policy: {:?}", self.config.cover_policy));
        out.push(format!(
            "execution mode: {}",
            execution_mode_label(self.config.execution_mode)
        ));
        out.push(format!(
            "output mode: {}",
            output_mode_label(self.config.output_mode)
        ));
        out.push(format!(
            "sink mode: {}",
            sink_mode_label(self.config.output_mode)
        ));
        out.push(
            "public output: duplicate-free QueryResultSet; no public aggregate support".to_owned(),
        );
        out.push(format!("free join nodes: {}", self.validated.nodes.len()));
        out.push(format!(
            "subatoms: {}",
            self.validated
                .nodes
                .iter()
                .map(|node| self.validated.node_subatoms(node).len())
                .sum::<usize>()
        ));
        out.push(format!(
            "atom partitions: {}",
            self.validated.atom_partitions.len()
        ));
        out.push(format!(
            "storage_tx_id: {}",
            self.selection.stats.storage_tx_id
        ));
        out.push(format!(
            "schema_fingerprint: {}",
            hex(&self.selection.stats.schema_fingerprint)
        ));
        out.push(
            "source kind: COLT over LMDB base image; optional accelerator entries: 0".to_owned(),
        );
        out.push(
            "base-image cache: snapshot-local keyed by storage tx and schema fingerprint"
                .to_owned(),
        );
        out.push(
            "timings and allocations: available in debug builds and release builds compiled with query-tracing"
                .to_owned(),
        );
        out.push("formal Free Join plan after factorization/selection:".to_owned());
        for node in &self.validated.nodes {
            out.push(format!(
                "node {} available={:?} new={:?} covers={:?}",
                node.id,
                self.validated.node_available_vars(node),
                self.validated.node_new_vars(node),
                self.validated.node_covers(node)
            ));
            for (index, subatom) in self.validated.node_subatoms(node).iter().enumerate() {
                out.push(format!(
                    "  subatom {index}: atom={:?} vars={:?} fields={:?}",
                    subatom.atom,
                    self.validated.subatom_vars(subatom),
                    self.validated.subatom_field_ids(subatom)
                ));
            }
        }
        out.extend(self.ght_schemas.iter().cloned());
        out.push(format!(
            "candidate costs: {:?}",
            self.selection
                .candidates
                .iter()
                .map(|candidate| (candidate.family, candidate.cost))
                .collect::<Vec<_>>()
        ));
        out.join("\n")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TraceSpan {
    pub(crate) phase: &'static str,
}

pub(crate) fn summarize_trace_json(spans: &[TraceSpan]) -> String {
    let mut phases = std::collections::BTreeMap::new();
    for span in spans {
        *phases.entry(span.phase).or_insert(0usize) += 1;
    }
    let fields = phases
        .into_iter()
        .map(|(phase, count)| format!("\"{}\":{}", phase, count))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"trace_phases\":{{{fields}}}}}")
}

fn plan_family_label(family: PlanFamily) -> &'static str {
    match family {
        PlanFamily::Singleton => "formal singleton-subatom Free Join plan",
        PlanFamily::BinaryDerived => "binary-derived Free Join plan",
        PlanFamily::FactoredBinary => "factored Free Join plan",
        PlanFamily::InjectedBinary => "injected binary-derived Free Join plan",
    }
}

fn execution_mode_label(mode: ExecutionMode) -> String {
    match mode {
        ExecutionMode::Scalar => "scalar".to_owned(),
        ExecutionMode::Vectorized { batch_size } => format!("vectorized batch_size={batch_size}"),
    }
}

fn output_mode_label(mode: OutputMode) -> &'static str {
    match mode {
        OutputMode::Materialized => "materialized set",
        OutputMode::Factorized => "internal factorized",
    }
}

fn sink_mode_label(mode: OutputMode) -> &'static str {
    match mode {
        OutputMode::Materialized => "projection result-set sink",
        OutputMode::Factorized => "internal factorized projection sink",
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
#[path = "explain_tests.rs"]
mod tests;
