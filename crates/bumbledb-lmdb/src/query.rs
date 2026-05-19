use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Instant;

use smallvec::SmallVec;

use bumbledb_core::datalog::{
    AggregateFunction, ComparisonOperator, Literal, TypedClause, TypedComparison, TypedFindTerm,
    TypedLiteral, TypedOperand, TypedQuery, TypedRelationAtom, TypedTerm,
};
use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::schema::{CurrentIndexLayout, IndexKind, ValueType};

use crate::{
    AccessId, AggregatePlan, AggregateTerm, AtomId, EncodedOwned, Error, FieldId, FreeJoinPlan,
    HashTrieIndex, IndexSpec, LinearIter, NodeId, NodeImpl, OutputPlan, PayloadDemand,
    PlanEstimates, PlanNode, PrefixProbe, PrefixRows, ProjectPlan, ReadTxn, RelationImage,
    RelationStats, Result, RowId, SortedTrieIndex, StorageSchema, SubAtom, TrieIter, Value, VarId,
};

use crate::QueryImageCacheDiagnostics;
use crate::allocation::{self, ALLOCATION_SIZE_CLASS_COUNT, AllocationDelta};
use crate::planner_stats::{
    OptimizerFieldStats, OptimizerIndexStats, OptimizerRelationStats, PlannerStatsCacheDiagnostics,
};

/// Query input bindings keyed by input name without `$`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputBindings {
    values: BTreeMap<String, Value>,
}

impl InputBindings {
    /// Creates empty input bindings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates input bindings from key/value pairs.
    pub fn from_values(values: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Self {
        Self {
            values: values
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
        }
    }

    fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }

    /// Returns a bound input value by name.
    pub fn value(&self, name: &str) -> Option<&Value> {
        self.values.get(name)
    }
}

/// Dense input ID inside a normalized query.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputId(pub u16);

/// Dense predicate ID inside a normalized query.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PredicateId(pub u16);

/// Executor-friendly normalized Datalog query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedQuery {
    /// Dense variables used by this query.
    pub vars: Vec<NormVar>,
    /// Dense inputs used by this query.
    pub inputs: Vec<NormInput>,
    /// Relation atoms in clause order.
    pub atoms: Vec<NormAtom>,
    /// Normalized comparison predicates.
    pub predicates: Vec<NormPredicate>,
    /// Output plan used by sinks.
    pub output: OutputPlan,
    /// Original find-term order after normalization.
    pub find: Vec<NormFindTerm>,
}

/// Normalized variable metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormVar {
    /// Dense variable ID.
    pub id: VarId,
    /// Source variable name without `?`.
    pub name: String,
    /// Logical value type.
    pub value_type: ValueType,
}

/// Normalized input metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormInput {
    /// Dense input ID.
    pub id: InputId,
    /// Source input name without `$`.
    pub name: String,
    /// Logical value type.
    pub value_type: ValueType,
}

/// Normalized relation atom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormAtom {
    /// Dense atom ID in relation-clause order.
    pub id: AtomId,
    /// Dense relation ID in schema declaration order.
    pub relation: crate::RelationId,
    /// Relation name, retained for diagnostics and image lookup.
    pub relation_name: String,
    /// Normalized atom fields.
    pub fields: Vec<NormAtomField>,
}

/// Normalized atom field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormAtomField {
    /// Dense field ID in relation declaration order.
    pub field: FieldId,
    /// Field name, retained for diagnostics and access-path lookup.
    pub field_name: String,
    /// Bound normalized term.
    pub term: NormTerm,
    /// Logical field value type.
    pub value_type: ValueType,
}

/// Normalized atom term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormTerm {
    /// Variable reference.
    Var(VarId),
    /// Input reference.
    Input(InputId),
    /// Encoded literal.
    Literal(EncodedOwned),
    /// Wildcard.
    Wildcard,
}

/// Normalized comparison predicate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormPredicate {
    /// Dense predicate ID in comparison-clause order.
    pub id: PredicateId,
    /// Binary operands.
    pub operands: [NormOperand; 2],
    /// Comparison operation.
    pub op: ComparisonOperator,
    /// Logical comparison value type.
    pub value_type: ValueType,
    /// Earliest variable-order depth where this predicate can be evaluated.
    pub earliest_depth: Option<usize>,
}

/// Normalized comparison operand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormOperand {
    /// Variable reference.
    Var(VarId),
    /// Input reference.
    Input(InputId),
    /// Encoded literal.
    Literal(EncodedOwned),
}

/// Normalized output term in source find order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormFindTerm {
    /// Projected variable.
    Variable { variable: VarId },
    /// Aggregate over a variable.
    Aggregate {
        /// Aggregate function.
        function: AggregateFunction,
        /// Aggregated variable.
        variable: VarId,
        /// Aggregate operand type.
        value_type: ValueType,
    },
}

#[derive(Clone, Debug)]
struct EncodedInputs {
    values: Vec<EncodedOwned>,
}

impl EncodedInputs {
    fn get(&self, input: InputId) -> Option<&EncodedOwned> {
        self.values.get(input.0 as usize)
    }
}

/// Query execution output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryOutput {
    /// Result columns in projection order.
    pub columns: Vec<ResultColumn>,
    /// Result rows in unspecified order.
    pub rows: Vec<Vec<Value>>,
    /// Physical plan and counters.
    pub plan: QueryPlan,
}

impl QueryOutput {
    /// Renders a human-readable explain plan for this executed query.
    pub fn explain(&self) -> String {
        self.plan.explain()
    }
}

/// Result column metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResultColumn {
    /// Projected variable.
    Variable(String),
    /// Aggregate over a variable.
    Aggregate {
        /// Aggregate function.
        function: AggregateFunction,
        /// Variable name.
        variable: String,
    },
}

/// Physical query plan summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryPlan {
    /// Deterministic variable ordering optimizer output.
    pub variable_order: Vec<String>,
    /// Estimated work for variables in execution order.
    pub variable_estimates: Vec<VariableEstimate>,
    /// Physical index recommendations for predicates not served by leading indexes.
    pub missing_indexes: Vec<MissingIndexRecommendation>,
    /// Optimizer candidates and chosen cost key.
    pub optimizer: OptimizerTrace,
    /// Query image cache diagnostics after acquiring this query image.
    pub query_image_cache: QueryImageCacheDiagnostics,
    /// Planner statistics cache diagnostics after planning.
    pub planner_stats: PlannerStatsCacheDiagnostics,
    /// Node-level estimated and observed row/candidate counts.
    pub node_rows: Vec<NodeRowEstimate>,
    /// Node-level execution summaries.
    pub node_timings: Vec<QueryNodeTiming>,
    /// Free Join physical plan IR.
    pub free_join: FreeJoinPlan,
    /// Runtime implementation used for this execution.
    pub runtime_kind: QueryRuntimeKind,
    /// Coarse query phase timings.
    pub timings: QueryTimings,
    /// Allocation summary for this query, disabled by default.
    pub allocations: QueryAllocationStats,
    /// Execution counters.
    pub counters: PlanCounters,
    /// True when multiple relation atoms are evaluated as one indexed multiway search.
    pub uses_indexed_multiway_join: bool,
}

impl QueryPlan {
    /// Renders this physical plan and its current execution counters.
    pub fn explain(&self) -> String {
        let mut out = String::new();
        out.push_str("QueryPlan\n");
        out.push_str(&format!("variable_order: {:?}\n", self.variable_order));
        out.push_str(&format!("runtime_kind: {:?}\n", self.runtime_kind));
        out.push_str(&format!(
            "uses_indexed_multiway_join: {}\n",
            self.uses_indexed_multiway_join
        ));
        out.push_str("timings:\n");
        out.push_str(&format!(
            "  query_timing total_micros={} validate_inputs_micros={} normalize_micros={} encode_inputs_micros={} query_image_micros={} plan_micros={} lftj_build_micros={} hash_index_micros={} execute_micros={} lftj_execute_micros={} hash_execute_micros={} sink_emit_micros={} sink_finish_micros={} decode_micros={}\n",
            self.timings.total_micros,
            self.timings.validate_inputs_micros,
            self.timings.normalize_micros,
            self.timings.encode_inputs_micros,
            self.timings.query_image_micros,
            self.timings.plan_micros,
            self.timings.lftj_build_micros,
            self.timings.hash_index_micros,
            self.timings.execute_micros,
            self.timings.lftj_execute_micros,
            self.timings.hash_execute_micros,
            self.timings.sink_emit_micros,
            self.timings.sink_finish_micros,
            self.timings.decode_micros
        ));
        out.push_str("allocations:\n");
        out.push_str(&format!(
            "  allocation_summary enabled={} alloc_calls={} dealloc_calls={} realloc_calls={} bytes_allocated={} bytes_deallocated={} net_bytes={} current_live_bytes={} peak_live_bytes={}\n",
            self.allocations.enabled,
            self.allocations.alloc_calls,
            self.allocations.dealloc_calls,
            self.allocations.realloc_calls,
            self.allocations.bytes_allocated,
            self.allocations.bytes_deallocated,
            self.allocations.net_bytes,
            self.allocations.current_live_bytes,
            self.allocations.peak_live_bytes
        ));
        self.allocations
            .validate_inputs
            .write_explain(&mut out, "validate_inputs");
        self.allocations
            .normalize
            .write_explain(&mut out, "normalize");
        self.allocations
            .encode_inputs
            .write_explain(&mut out, "encode_inputs");
        self.allocations
            .query_image
            .write_explain(&mut out, "query_image");
        self.allocations.plan.write_explain(&mut out, "plan");
        self.allocations
            .lftj_build
            .write_explain(&mut out, "lftj_build");
        self.allocations
            .hash_index
            .write_explain(&mut out, "hash_index");
        self.allocations.execute.write_explain(&mut out, "execute");
        self.allocations
            .sink_finish
            .write_explain(&mut out, "sink_finish");
        out.push_str("variable_estimates:\n");
        for estimate in &self.variable_estimates {
            out.push_str(&format!(
                "  variable_estimate name={} estimated_candidates={} static_constraints={} bound_constraints={} relation_constraints={} access={} reason={}\n",
                estimate.variable,
                estimate.estimated_candidates,
                estimate.static_constraints,
                estimate.bound_constraints,
                estimate.relation_constraints,
                estimate.access,
                estimate.reason
            ));
        }
        if !self.missing_indexes.is_empty() {
            out.push_str("missing_indexes:\n");
            for missing in &self.missing_indexes {
                out.push_str(&format!(
                    "  missing_index relation={} fields={:?} reason={}\n",
                    missing.relation, missing.fields, missing.reason
                ));
            }
        }
        out.push_str("optimizer:\n");
        out.push_str(&format!(
            "  query_image_cache cached_images={} hits={} misses={} builds={} build_micros={}\n",
            self.query_image_cache.cached_images,
            self.query_image_cache.hits,
            self.query_image_cache.misses,
            self.query_image_cache.builds,
            self.query_image_cache.build_micros
        ));
        out.push_str(&format!(
            "  planner_stats cached_relations={} hits={} misses={} builds={} build_micros={}\n",
            self.planner_stats.cached_relations,
            self.planner_stats.hits,
            self.planner_stats.misses,
            self.planner_stats.builds,
            self.planner_stats.build_micros
        ));
        out.push_str(&format!("  chosen_plan: {}\n", self.optimizer.chosen));
        for candidate in &self.optimizer.candidates {
            out.push_str(&format!(
                "  candidate_plan name={} selected={} estimated_micros={} memory_bytes={} materialization_penalty={} tie_breaker={} rejected_reason={} impls={:?}\n",
                candidate.name,
                candidate.selected,
                candidate.cost.estimated_micros,
                candidate.cost.memory_bytes,
                candidate.cost.materialization_penalty,
                candidate.cost.tie_breaker,
                candidate.rejected_reason,
                candidate.implementations
            ));
        }
        out.push_str(&format!(
            "free_join_estimates: output_rows={} iterator_ops={} hash_build_rows={} hash_probe_rows={} materialized_values={} memory_bytes={} actual_output_rows={}\n",
            self.free_join.estimates.output_rows,
            self.free_join.estimates.iterator_ops,
            self.free_join.estimates.hash_build_rows,
            self.free_join.estimates.hash_probe_rows,
            self.free_join.estimates.materialized_values,
            self.free_join.estimates.memory_bytes,
            self.counters.output_rows
        ));
        out.push_str("free_join_plan:\n");
        for node in &self.free_join.nodes {
            out.push_str(&format!(
                "  free_join_node id={} impl={:?} bind_vars={:?} subatoms={}\n",
                node.id.0,
                node.implementation,
                node.bind_vars.iter().map(|var| var.0).collect::<Vec<_>>(),
                node.subatoms.len()
            ));
            if let Some(rows) = self.node_rows.get(node.id.0 as usize) {
                out.push_str(&format!(
                    "    node_rows variable={} estimated_rows={} actual_rows={}\n",
                    rows.variable, rows.estimated_rows, rows.actual_rows
                ));
            }
            if let Some(timing) = self.node_timings.get(node.id.0 as usize) {
                out.push_str(&format!(
                    "    node_timing node={} impl={:?} estimated_rows={} actual_rows={} execute_micros={}\n",
                    timing.node.0,
                    timing.implementation,
                    timing.estimated_rows,
                    timing.actual_rows,
                    timing.execute_micros
                ));
            }
            for subatom in &node.subatoms {
                out.push_str(&format!(
                    "    free_join_subatom atom={} relation={} fields={:?} vars={:?} access={}\n",
                    subatom.atom_id.0,
                    subatom.relation.0,
                    subatom
                        .fields
                        .iter()
                        .map(|field| field.0)
                        .collect::<Vec<_>>(),
                    subatom.vars.iter().map(|var| var.0).collect::<Vec<_>>(),
                    subatom.access.0
                ));
            }
        }
        out.push_str("counters:\n");
        out.push_str(&format!("  cursor_seeks: {}\n", self.counters.cursor_seeks));
        out.push_str(&format!("  rows_scanned: {}\n", self.counters.rows_scanned));
        out.push_str(&format!("  rows_matched: {}\n", self.counters.rows_matched));
        out.push_str(&format!(
            "  bindings_yielded: {}\n",
            self.counters.bindings_yielded
        ));
        out.push_str(&format!(
            "  comparisons_evaluated: {}\n",
            self.counters.comparisons_evaluated
        ));
        out.push_str(&format!(
            "  comparisons_failed: {}\n",
            self.counters.comparisons_failed
        ));
        out.push_str(&format!(
            "  aggregate_groups: {}\n",
            self.counters.aggregate_groups
        ));
        out.push_str(&format!(
            "  trie_intersections: {}\n",
            self.counters.trie_intersections
        ));
        out.push_str(&format!(
            "  variable_candidates: {}\n",
            self.counters.variable_candidates
        ));
        out.push_str(&format!(
            "  decoded_values: {}\n",
            self.counters.decoded_values
        ));
        out.push_str(&format!(
            "  dictionary_reverse_lookups: {}\n",
            self.counters.dictionary_reverse_lookups
        ));
        out.push_str(&format!(
            "  encoded_comparisons_evaluated: {}\n",
            self.counters.encoded_comparisons_evaluated
        ));
        out.push_str(&format!(
            "  decoded_comparisons_evaluated: {}\n",
            self.counters.decoded_comparisons_evaluated
        ));
        out.push_str(&format!(
            "  materialized_output_values: {}\n",
            self.counters.materialized_output_values
        ));
        out.push_str(&format!("  trie_open: {}\n", self.counters.trie_open));
        out.push_str(&format!("  trie_up: {}\n", self.counters.trie_up));
        out.push_str(&format!("  trie_next: {}\n", self.counters.trie_next));
        out.push_str(&format!("  trie_seek: {}\n", self.counters.trie_seek));
        out.push_str(&format!(
            "  trie_key_reads: {}\n",
            self.counters.trie_key_reads
        ));
        out.push_str(&format!(
            "  sorted_trie_cache_hits: {}\n",
            self.counters.sorted_trie_cache_hits
        ));
        out.push_str(&format!(
            "  sorted_trie_cache_misses: {}\n",
            self.counters.sorted_trie_cache_misses
        ));
        out.push_str(&format!(
            "  sorted_trie_builds: {}\n",
            self.counters.sorted_trie_builds
        ));
        out.push_str(&format!(
            "  sorted_trie_build_micros: {}\n",
            self.counters.sorted_trie_build_micros
        ));
        out.push_str(&format!(
            "  atom_temp_relation_builds: {}\n",
            self.counters.atom_temp_relation_builds
        ));
        out.push_str(&format!(
            "  atom_temp_relation_source_rows: {}\n",
            self.counters.atom_temp_relation_source_rows
        ));
        out.push_str(&format!(
            "  atom_temp_relation_rows: {}\n",
            self.counters.atom_temp_relation_rows
        ));
        out.push_str(&format!(
            "  hash_index_builds: {}\n",
            self.counters.hash_index_builds
        ));
        out.push_str(&format!(
            "  hash_index_build_rows: {}\n",
            self.counters.hash_index_build_rows
        ));
        out.push_str(&format!(
            "  hash_probe_calls: {}\n",
            self.counters.hash_probe_calls
        ));
        out.push_str(&format!(
            "  hash_probe_hits: {}\n",
            self.counters.hash_probe_hits
        ));
        out.push_str(&format!(
            "  hash_probe_misses: {}\n",
            self.counters.hash_probe_misses
        ));
        out.push_str(&format!(
            "  hash_rows_returned: {}\n",
            self.counters.hash_rows_returned
        ));
        out.push_str(&format!(
            "  hash_distinct_emits: {}\n",
            self.counters.hash_distinct_emits
        ));
        out.push_str(&format!("  output_rows: {}\n", self.counters.output_rows));
        out
    }

    fn refresh_node_timings(&mut self) {
        for timing in &mut self.node_timings {
            if let Some(rows) = self.node_rows.get(timing.node.0 as usize) {
                timing.actual_rows = rows.actual_rows;
            }
        }
    }
}

/// Runtime implementation used by one query execution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueryRuntimeKind {
    /// Runtime has not executed yet.
    #[default]
    Unknown,
    /// Sorted trie leapfrog executor.
    Lftj,
    /// Hash probe executor.
    HashProbe,
    /// Free Join fallback for non-pure or mixed node implementations.
    MixedFallback,
    /// Reserved for direct selective kernels.
    DirectKernel,
}

/// Coarse query phase timings in microseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueryTimings {
    /// Inclusive total query execution time.
    pub total_micros: u128,
    /// Input validation time.
    pub validate_inputs_micros: u128,
    /// Query normalization time.
    pub normalize_micros: u128,
    /// Input encoding time.
    pub encode_inputs_micros: u128,
    /// QueryImage acquisition time.
    pub query_image_micros: u128,
    /// Planning time.
    pub plan_micros: u128,
    /// LFTJ atom plan/index preparation time.
    pub lftj_build_micros: u128,
    /// Hash index lookup/build preparation time.
    pub hash_index_micros: u128,
    /// Runtime execution time before sink finish.
    pub execute_micros: u128,
    /// LFTJ recursive execution time.
    pub lftj_execute_micros: u128,
    /// Hash probe execution time.
    pub hash_execute_micros: u128,
    /// Sink emit timing, zero until per-sink emit timing is enabled.
    pub sink_emit_micros: u128,
    /// Sink finalization/materialization time.
    pub sink_finish_micros: u128,
    /// Decode timing, zero until per-decode timing is enabled.
    pub decode_micros: u128,
}

/// Allocation counters for one query phase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocationPhaseStats {
    /// True when allocation profiling was enabled.
    pub enabled: bool,
    /// Allocation calls observed.
    pub alloc_calls: u64,
    /// Deallocation calls observed.
    pub dealloc_calls: u64,
    /// Reallocation calls observed.
    pub realloc_calls: u64,
    /// Bytes allocated.
    pub bytes_allocated: u64,
    /// Bytes deallocated.
    pub bytes_deallocated: u64,
    /// Net bytes allocated minus deallocated.
    pub net_bytes: i128,
    /// Current live byte delta after the phase.
    pub current_live_bytes: u64,
    /// Peak live bytes observed.
    pub peak_live_bytes: u64,
    /// Allocation calls by size class.
    pub size_class_allocs: [u64; ALLOCATION_SIZE_CLASS_COUNT],
}

impl From<AllocationDelta> for AllocationPhaseStats {
    fn from(delta: AllocationDelta) -> Self {
        Self {
            enabled: delta.enabled,
            alloc_calls: delta.alloc_calls,
            dealloc_calls: delta.dealloc_calls,
            realloc_calls: delta.realloc_calls,
            bytes_allocated: delta.bytes_allocated,
            bytes_deallocated: delta.bytes_deallocated,
            net_bytes: delta.net_bytes,
            current_live_bytes: delta.current_live_bytes,
            peak_live_bytes: delta.peak_live_bytes,
            size_class_allocs: delta.size_class_allocs,
        }
    }
}

impl AllocationPhaseStats {
    fn write_explain(self, out: &mut String, phase: &str) {
        let _ = writeln!(
            out,
            "  allocation_phase phase={} enabled={} alloc_calls={} dealloc_calls={} realloc_calls={} bytes_allocated={} bytes_deallocated={} net_bytes={} current_live_bytes={} peak_live_bytes={}",
            phase,
            self.enabled,
            self.alloc_calls,
            self.dealloc_calls,
            self.realloc_calls,
            self.bytes_allocated,
            self.bytes_deallocated,
            self.net_bytes,
            self.current_live_bytes,
            self.peak_live_bytes
        );
    }
}

/// Allocation summary for one query execution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueryAllocationStats {
    /// True when allocation profiling was enabled.
    pub enabled: bool,
    /// Allocation calls observed.
    pub alloc_calls: u64,
    /// Deallocation calls observed.
    pub dealloc_calls: u64,
    /// Reallocation calls observed.
    pub realloc_calls: u64,
    /// Bytes allocated.
    pub bytes_allocated: u64,
    /// Bytes deallocated.
    pub bytes_deallocated: u64,
    /// Net bytes allocated minus deallocated.
    pub net_bytes: i128,
    /// Current live byte delta after the query.
    pub current_live_bytes: u64,
    /// Peak live bytes observed.
    pub peak_live_bytes: u64,
    /// Allocation calls by size class.
    pub size_class_allocs: [u64; ALLOCATION_SIZE_CLASS_COUNT],
    /// Total query allocation delta.
    pub total: AllocationPhaseStats,
    /// Input validation allocation delta.
    pub validate_inputs: AllocationPhaseStats,
    /// Query normalization allocation delta.
    pub normalize: AllocationPhaseStats,
    /// Input encoding allocation delta.
    pub encode_inputs: AllocationPhaseStats,
    /// QueryImage acquisition allocation delta.
    pub query_image: AllocationPhaseStats,
    /// Planning allocation delta.
    pub plan: AllocationPhaseStats,
    /// LFTJ build allocation delta.
    pub lftj_build: AllocationPhaseStats,
    /// Hash index allocation delta.
    pub hash_index: AllocationPhaseStats,
    /// Runtime execution allocation delta.
    pub execute: AllocationPhaseStats,
    /// LFTJ execution allocation delta.
    pub lftj_execute: AllocationPhaseStats,
    /// Hash execution allocation delta.
    pub hash_execute: AllocationPhaseStats,
    /// Sink finalization allocation delta.
    pub sink_finish: AllocationPhaseStats,
}

impl QueryAllocationStats {
    fn with_total(mut self, total: AllocationPhaseStats) -> Self {
        self.enabled = total.enabled;
        self.alloc_calls = total.alloc_calls;
        self.dealloc_calls = total.dealloc_calls;
        self.realloc_calls = total.realloc_calls;
        self.bytes_allocated = total.bytes_allocated;
        self.bytes_deallocated = total.bytes_deallocated;
        self.net_bytes = total.net_bytes;
        self.current_live_bytes = total.current_live_bytes;
        self.peak_live_bytes = total.peak_live_bytes;
        self.size_class_allocs = total.size_class_allocs;
        self.total = total;
        self
    }
}

/// Node-level execution timing and counter summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryNodeTiming {
    /// Dense Free Join node ID.
    pub node: NodeId,
    /// Runtime implementation selected for the node.
    pub implementation: NodeImpl,
    /// Variables bound by this node.
    pub bind_vars: Vec<VarId>,
    /// Estimated rows/candidates for this node.
    pub estimated_rows: u64,
    /// Observed accepted candidates for this node.
    pub actual_rows: u64,
    /// Coarse node execution time, zero until node-level timing is enabled.
    pub execute_micros: u128,
}

/// Optimizer estimate for one variable in execution order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableEstimate {
    /// Variable name without `?`.
    pub variable: String,
    /// Estimated candidate domain size at the point this variable is bound.
    pub estimated_candidates: u64,
    /// Input/literal/comparison constraints available before binding this variable.
    pub static_constraints: usize,
    /// Already-bound variable constraints available before binding this variable.
    pub bound_constraints: usize,
    /// Number of relation atoms constraining this variable.
    pub relation_constraints: usize,
    /// Stats-backed access path used for the estimate.
    pub access: String,
    /// Human-readable stats explanation for the chosen variable order step.
    pub reason: String,
}

/// Physical index recommendation emitted by the planner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingIndexRecommendation {
    /// Relation name.
    pub relation: String,
    /// Suggested leading fields.
    pub fields: Vec<String>,
    /// Why the planner wants this index.
    pub reason: String,
}

/// Optimizer trace for one planned query.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OptimizerTrace {
    /// Chosen candidate name.
    pub chosen: String,
    /// Candidate plans considered by the optimizer.
    pub candidates: Vec<PlanCandidate>,
}

/// One optimizer candidate and its stable cost key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanCandidate {
    /// Stable candidate name.
    pub name: String,
    /// Node implementations in plan order.
    pub implementations: Vec<NodeImpl>,
    /// Stable cost key used for ordering.
    pub cost: CostKey,
    /// True for the selected candidate.
    pub selected: bool,
    /// Top-level rejection reason for non-selected candidates.
    pub rejected_reason: String,
}

/// Stable optimizer cost key.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CostKey {
    /// Estimated execution time in microseconds.
    pub estimated_micros: u64,
    /// Estimated extra memory footprint in bytes.
    pub memory_bytes: usize,
    /// Penalty for materializing output values or intermediate payload.
    pub materialization_penalty: u64,
    /// Stable deterministic tie-breaker.
    pub tie_breaker: String,
}

/// Estimated and observed rows/candidates for one Free Join node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeRowEstimate {
    /// Dense node ID.
    pub node: NodeId,
    /// Variable bound by this node.
    pub variable: String,
    /// Estimated rows/candidates for this node.
    pub estimated_rows: u64,
    /// Observed accepted candidates for this node.
    pub actual_rows: u64,
}

/// Execution counters for the Free Join query executor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanCounters {
    /// Number of encoded index scan openings.
    pub cursor_seeks: u64,
    /// Number of encoded index entries inspected.
    pub rows_scanned: u64,
    /// Number of encoded index entries accepted by currently bound constraints.
    pub rows_matched: u64,
    /// Number of complete encoded bindings yielded before projection/aggregation.
    pub bindings_yielded: u64,
    /// Number of comparison predicates evaluated.
    pub comparisons_evaluated: u64,
    /// Number of comparison predicate failures.
    pub comparisons_failed: u64,
    /// Number of aggregate groups produced.
    pub aggregate_groups: u64,
    /// Number of final output rows.
    pub output_rows: u64,
    /// Number of variable-domain intersections performed.
    pub trie_intersections: u64,
    /// Number of candidate variable values produced after intersection.
    pub variable_candidates: u64,
    /// Number of logical values decoded for comparisons/projection/aggregation.
    pub decoded_values: u64,
    /// Number of string/bytes dictionary reverse lookups caused by decoding.
    pub dictionary_reverse_lookups: u64,
    /// Number of comparison predicates evaluated directly on encoded bytes.
    pub encoded_comparisons_evaluated: u64,
    /// Number of comparison predicates evaluated after logical decoding.
    pub decoded_comparisons_evaluated: u64,
    /// Number of final logical output values materialized.
    pub materialized_output_values: u64,
    /// Number of trie iterator open operations.
    pub trie_open: u64,
    /// Number of trie iterator up operations.
    pub trie_up: u64,
    /// Number of trie iterator next operations.
    pub trie_next: u64,
    /// Number of trie iterator seek operations.
    pub trie_seek: u64,
    /// Number of trie iterator key reads.
    pub trie_key_reads: u64,
    /// Number of sorted trie cache hits while preparing query atom indexes.
    pub sorted_trie_cache_hits: u64,
    /// Number of sorted trie cache misses while preparing query atom indexes.
    pub sorted_trie_cache_misses: u64,
    /// Number of sorted trie builds while preparing query atom indexes.
    pub sorted_trie_builds: u64,
    /// Total sorted trie build time while preparing query atom indexes.
    pub sorted_trie_build_micros: u64,
    /// Number of temporary atom relation images built on cache misses.
    pub atom_temp_relation_builds: u64,
    /// Number of source rows inspected while building temporary atom relations.
    pub atom_temp_relation_source_rows: u64,
    /// Number of rows retained in temporary atom relations.
    pub atom_temp_relation_rows: u64,
    /// Number of hash trie indexes built for hash probe execution.
    pub hash_index_builds: u64,
    /// Number of source rows used to build hash indexes.
    pub hash_index_build_rows: u64,
    /// Number of hash prefix probe calls.
    pub hash_probe_calls: u64,
    /// Number of hash prefix probes that found at least one row.
    pub hash_probe_hits: u64,
    /// Number of hash prefix probes that found no rows.
    pub hash_probe_misses: u64,
    /// Number of row IDs returned from hash prefix probes.
    pub hash_rows_returned: u64,
    /// Number of bindings emitted by hash probe nodes.
    pub hash_distinct_emits: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct EncodedValue {
    value_type: ValueType,
    encoded: EncodedOwned,
}

impl EncodedValue {
    fn new(value_type: ValueType, encoded: EncodedOwned) -> Self {
        Self {
            value_type,
            encoded,
        }
    }

    fn from_owned(value_type: ValueType, value: &EncodedOwned) -> Self {
        Self {
            value_type,
            encoded: value.clone(),
        }
    }

    fn from_bytes(value_type: ValueType, bytes: &[u8]) -> Result<Self> {
        Ok(Self {
            encoded: encoded_owned_for_width(value_type.encoded_width(), bytes)?,
            value_type,
        })
    }

    fn as_bytes(&self) -> &[u8] {
        self.encoded.as_bytes()
    }
}

#[derive(Clone, Debug)]
struct EncodedBinding {
    values: SmallVec<[Option<EncodedValue>; 8]>,
}

impl EncodedBinding {
    fn new(variable_count: usize) -> Self {
        Self {
            values: std::iter::repeat_with(|| None)
                .take(variable_count)
                .collect(),
        }
    }

    fn get(&self, variable: usize) -> Option<&EncodedValue> {
        self.values[variable].as_ref()
    }

    fn bind(&mut self, variable: usize, value: EncodedValue) -> bool {
        match &self.values[variable] {
            Some(existing) => existing.encoded == value.encoded,
            None => {
                self.values[variable] = Some(value);
                true
            }
        }
    }

    fn unbind(&mut self, variable: usize) {
        self.values[variable] = None;
    }
}

#[derive(Clone, Debug)]
struct ExecutionPlan {
    variable_order_ids: Vec<usize>,
    relation_atoms: Vec<NormAtom>,
    comparisons: Vec<NormPredicate>,
    summary: QueryPlan,
}

#[derive(Clone, Debug)]
struct PlannerStats {
    relations: BTreeMap<String, Arc<OptimizerRelationStats>>,
}

impl PlannerStats {
    fn collect(
        schema: &StorageSchema,
        image: &crate::QueryImage,
        atoms: &[&NormAtom],
    ) -> Result<Self> {
        let mut relations = BTreeMap::new();
        for atom in atoms {
            if relations.contains_key(&atom.relation_name) {
                continue;
            }
            let relation = image
                .relations()
                .get(atom.relation.0 as usize)
                .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
            relations.insert(
                atom.relation_name.clone(),
                image.planner_relation_stats(schema, relation)?,
            );
        }
        Ok(Self { relations })
    }

    fn relation_rows(&self, relation: &str) -> u64 {
        self.relations
            .get(relation)
            .map(|stats| stats.rows as u64)
            .unwrap_or(1)
            .max(1)
    }

    fn field_stats(&self, relation: &str, field: &str) -> Option<&OptimizerFieldStats> {
        self.relations.get(relation)?.fields.get(field)
    }

    fn index_stats(&self, relation: &str, index: &str) -> Option<&OptimizerIndexStats> {
        self.relations.get(relation)?.indexes.get(index)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VariableCost {
    variable: usize,
    estimated_candidates: u64,
    static_constraints: usize,
    bound_constraints: usize,
    relation_constraints: usize,
    degree: usize,
    access: String,
    reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AccessEstimate {
    relation: String,
    index: String,
    access: AccessId,
    estimated_rows: u64,
    prefix_len: usize,
    current_is_next: bool,
    distinct: usize,
    avg_fanout: u64,
    max_fanout: usize,
    variable_distinct: usize,
    has_min: bool,
    has_max: bool,
    heavy_hitters: usize,
}

impl AccessEstimate {
    fn access_label(&self) -> String {
        format!("{}.{}", self.relation, self.index)
    }

    fn reason(&self) -> String {
        format!(
            "stats(prefix_len={},prefix_distinct={},avg_fanout={},max_fanout={},variable_distinct={},min={},max={},heavy_hitters={})",
            self.prefix_len,
            self.distinct,
            self.avg_fanout,
            self.max_fanout,
            self.variable_distinct,
            self.has_min,
            self.has_max,
            self.heavy_hitters
        )
    }
}

struct LftjAtomPlan {
    variables: Vec<usize>,
    trie: Arc<SortedTrieIndex>,
    row_count: usize,
}

struct LftjRuntime<'a> {
    participants_by_variable: Vec<SmallParticipants>,
    iters: Vec<crate::SortedTrieIter<'a>>,
}

struct HashAtomIndex {
    node_id: usize,
    atom_id: usize,
    index: Arc<HashTrieIndex>,
    fields: Vec<FieldId>,
}

type SmallParticipants = SmallVec<[usize; 4]>;
type SmallEncodedPrefix = SmallVec<[EncodedOwned; 8]>;
type SmallEncodedRefs<'a> = SmallVec<[crate::EncodedRef<'a>; 8]>;
type SmallEncodedRow = SmallVec<[EncodedValue; 8]>;

impl<'env> ReadTxn<'env> {
    /// Executes a typed positive Datalog query against current indexes.
    #[tracing::instrument(name = "bumbledb.query.execute", skip_all, fields(vars = query.variables.len(), clauses = query.clauses.len(), inputs = query.inputs.len()))]
    pub fn execute_query(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
    ) -> Result<QueryOutput> {
        let total_start = Instant::now();
        let total_alloc_start = allocation::snapshot();
        let mut timings = QueryTimings::default();
        let mut allocations = QueryAllocationStats::default();

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        {
            let _span = tracing::debug_span!("bumbledb.query.validate_inputs").entered();
            validate_inputs(query, inputs)?;
        }
        timings.validate_inputs_micros = elapsed_micros(phase_start);
        allocations.validate_inputs = allocation_delta_since(phase_alloc_start);

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let mut normalized = {
            let _span = tracing::debug_span!(
                "bumbledb.query.normalize",
                vars = query.variables.len(),
                clauses = query.clauses.len()
            )
            .entered();
            normalize_query(self, schema, query)?
        };
        timings.normalize_micros = elapsed_micros(phase_start);
        allocations.normalize = allocation_delta_since(phase_alloc_start);

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let encoded_inputs = {
            let _span = tracing::debug_span!(
                "bumbledb.query.encode_inputs",
                inputs = normalized.inputs.len()
            )
            .entered();
            encode_inputs(self, &normalized, inputs)?
        };
        timings.encode_inputs_micros = elapsed_micros(phase_start);
        allocations.encode_inputs = allocation_delta_since(phase_alloc_start);

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let image = {
            let _span = tracing::debug_span!("bumbledb.query.image").entered();
            self.query_images.get_or_build(self, schema)?
        };
        timings.query_image_micros = elapsed_micros(phase_start);
        allocations.query_image = allocation_delta_since(phase_alloc_start);

        let query_image_cache = self.query_images.diagnostics();
        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let mut plan = plan_query(schema, &mut normalized, image.as_ref(), query_image_cache)?;
        timings.plan_micros = elapsed_micros(phase_start);
        allocations.plan = allocation_delta_since(phase_alloc_start);
        plan.summary.timings = timings;
        plan.summary.allocations = allocations;
        tracing::debug!(variable_order = ?plan.summary.variable_order, nodes = plan.summary.free_join.nodes.len(), "free join query planned");
        let mut sink = OutputSink::new(&plan.summary.free_join.output);

        let execute_start = Instant::now();
        let execute_alloc_start = allocation::snapshot();
        execute_free_join(
            image.as_ref(),
            self,
            schema,
            &normalized,
            &encoded_inputs,
            &mut plan,
            &mut sink,
        )?;
        plan.summary.timings.execute_micros = elapsed_micros(execute_start);
        plan.summary.allocations.execute = allocation_delta_since(execute_alloc_start);

        let columns = result_columns(&normalized);
        let sink_finish_start = Instant::now();
        let sink_finish_alloc_start = allocation::snapshot();
        let rows = {
            let _span = tracing::debug_span!("bumbledb.query.sink.finish").entered();
            sink.finish(self, &normalized, &mut plan.summary.counters)?
        };
        plan.summary.timings.sink_finish_micros = elapsed_micros(sink_finish_start);
        plan.summary.allocations.sink_finish = allocation_delta_since(sink_finish_alloc_start);
        plan.summary.counters.output_rows = rows.len() as u64;
        if has_aggregate(&normalized) {
            plan.summary.counters.aggregate_groups = rows.len() as u64;
        }
        plan.summary.timings.total_micros = elapsed_micros(total_start);
        let total_alloc = allocation_delta_since(total_alloc_start);
        plan.summary.allocations = plan.summary.allocations.with_total(total_alloc);
        plan.summary.refresh_node_timings();
        tracing::debug!(?plan.summary.counters, "free join query executed");
        Ok(QueryOutput {
            columns,
            rows,
            plan: plan.summary,
        })
    }
}

fn elapsed_micros(start: Instant) -> u128 {
    start.elapsed().as_micros()
}

fn allocation_delta_since(start: allocation::AllocationSnapshot) -> AllocationPhaseStats {
    allocation::delta(start, allocation::snapshot()).into()
}

fn execute_free_join<'txn, 'query, S: TupleSink>(
    image: &crate::QueryImage,
    txn: &ReadTxn<'txn>,
    schema: &StorageSchema,
    query: &'query NormalizedQuery,
    inputs: &EncodedInputs,
    plan: &mut ExecutionPlan,
    sink: &mut S,
) -> Result<()> {
    let _span = tracing::debug_span!(
        "bumbledb.query.free_join.dispatch",
        nodes = plan.summary.free_join.nodes.len()
    )
    .entered();
    if plan
        .summary
        .free_join
        .nodes
        .iter()
        .all(|node| node.implementation == NodeImpl::HashProbe && node.bind_vars.len() == 1)
    {
        plan.summary.runtime_kind = QueryRuntimeKind::HashProbe;
        return execute_hash_probe(image, txn, schema, query, inputs, plan, sink);
    }
    plan.summary.runtime_kind = if plan.summary.free_join.is_pure_lftj() {
        QueryRuntimeKind::Lftj
    } else {
        QueryRuntimeKind::MixedFallback
    };
    execute_lftj(image, txn, query, inputs, plan, sink)
}

fn execute_hash_probe<'txn, 'query, S: TupleSink>(
    image: &crate::QueryImage,
    txn: &ReadTxn<'txn>,
    schema: &StorageSchema,
    query: &'query NormalizedQuery,
    inputs: &EncodedInputs,
    plan: &mut ExecutionPlan,
    sink: &mut S,
) -> Result<()> {
    let build_start = Instant::now();
    let build_alloc_start = allocation::snapshot();
    let atom_indexes = {
        let _span = tracing::debug_span!(
            "bumbledb.query.hash.build_indexes",
            atoms = plan.relation_atoms.len()
        )
        .entered();
        build_hash_atom_indexes(image, schema, plan)?
    };
    plan.summary.timings.hash_index_micros = plan
        .summary
        .timings
        .hash_index_micros
        .saturating_add(elapsed_micros(build_start));
    plan.summary.allocations.hash_index = allocation_delta_since(build_alloc_start);

    let execute_start = Instant::now();
    let execute_alloc_start = allocation::snapshot();
    let result = {
        let _span =
            tracing::debug_span!("bumbledb.query.hash.execute", variables = query.vars.len())
                .entered();
        let _sink_emit_span = tracing::debug_span!("bumbledb.query.sink.emit").entered();
        let participants_by_variable =
            hash_participants_by_variable(query.vars.len(), &plan.relation_atoms);
        let mut executor = HashProbeExecutor {
            image,
            txn,
            query,
            inputs,
            plan,
            atom_indexes,
            participants_by_variable,
            binding: EncodedBinding::new(query.vars.len()),
            sink,
        };
        if !executor.static_atoms_pass()? {
            Ok(())
        } else {
            executor.execute(0)
        }
    };
    plan.summary.timings.hash_execute_micros = plan
        .summary
        .timings
        .hash_execute_micros
        .saturating_add(elapsed_micros(execute_start));
    plan.summary.allocations.hash_execute = allocation_delta_since(execute_alloc_start);
    result
}

fn build_hash_atom_indexes(
    image: &crate::QueryImage,
    schema: &StorageSchema,
    plan: &mut ExecutionPlan,
) -> Result<Vec<HashAtomIndex>> {
    let mut out = Vec::new();
    let mut requested = BTreeSet::new();
    let subatoms = plan
        .summary
        .free_join
        .nodes
        .iter()
        .flat_map(|node| {
            node.subatoms.iter().map(move |subatom| {
                (
                    node.id.0 as usize,
                    subatom.atom_id.0 as usize,
                    subatom.access,
                    subatom.fields.clone(),
                )
            })
        })
        .collect::<Vec<_>>();
    for (node_id, atom_id, access, bind_fields) in subatoms {
        if !requested.insert((node_id, atom_id, access.0)) {
            continue;
        }
        let atom = &plan.relation_atoms[atom_id];
        let layout = layout_by_access(schema, atom, access)?;
        let mut fields = layout
            .leading_fields
            .iter()
            .map(|field_name| {
                atom.fields
                    .iter()
                    .find(|field| &field.field_name == field_name)
                    .map(|field| field.field)
                    .ok_or_else(|| Error::unknown_field(&atom.relation_name, field_name))
            })
            .collect::<Result<Vec<_>>>()?;
        for field in bind_fields {
            if !fields.contains(&field) {
                fields.push(field);
            }
        }
        let relation = image
            .relations()
            .get(atom.relation.0 as usize)
            .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
        let key = format!(
            "relation={};access={};fields={:?}",
            atom.relation.0, access.0, fields
        );
        let cached = image.cached_hash_trie(key, || {
            crate::query_image::build_hash_trie_index(
                relation,
                IndexSpec::new(format!("{}_hash", atom.relation_name), fields.clone()),
            )
        })?;
        if !cached.hit {
            plan.summary.counters.hash_index_builds += 1;
            plan.summary.counters.hash_index_build_rows = plan
                .summary
                .counters
                .hash_index_build_rows
                .saturating_add(relation.row_count as u64);
        }
        out.push(HashAtomIndex {
            node_id,
            atom_id,
            index: cached.index,
            fields,
        });
    }
    for atom_id in 0..plan.relation_atoms.len() {
        if !atom_variables(&plan.relation_atoms[atom_id]).is_empty() {
            continue;
        }
        if requested.iter().any(|(_, id, _)| *id == atom_id) {
            continue;
        }
        let atom = &plan.relation_atoms[atom_id];
        let access = AccessId(0);
        let layout = layout_by_access(schema, atom, access)?;
        let fields = layout
            .leading_fields
            .iter()
            .map(|field_name| {
                atom.fields
                    .iter()
                    .find(|field| &field.field_name == field_name)
                    .map(|field| field.field)
                    .ok_or_else(|| Error::unknown_field(&atom.relation_name, field_name))
            })
            .collect::<Result<Vec<_>>>()?;
        let relation = image
            .relations()
            .get(atom.relation.0 as usize)
            .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
        let key = format!(
            "relation={};access={};fields={:?}",
            atom.relation.0, access.0, fields
        );
        let cached = image.cached_hash_trie(key, || {
            crate::query_image::build_hash_trie_index(
                relation,
                IndexSpec::new(format!("{}_hash", atom.relation_name), fields.clone()),
            )
        })?;
        if !cached.hit {
            plan.summary.counters.hash_index_builds += 1;
            plan.summary.counters.hash_index_build_rows = plan
                .summary
                .counters
                .hash_index_build_rows
                .saturating_add(relation.row_count as u64);
        }
        out.push(HashAtomIndex {
            node_id: usize::MAX,
            atom_id,
            index: cached.index,
            fields,
        });
    }
    Ok(out)
}

fn layout_by_access<'a>(
    schema: &'a StorageSchema,
    atom: &NormAtom,
    access: AccessId,
) -> Result<&'a CurrentIndexLayout> {
    schema
        .layouts()
        .iter()
        .find(|layout| layout.relation_id == atom.relation.0 && layout.index_id == access.0)
        .ok_or_else(|| {
            Error::internal(format!(
                "missing access {} for relation {}",
                access.0, atom.relation_name
            ))
        })
}

fn hash_participants_by_variable(
    variable_count: usize,
    atoms: &[NormAtom],
) -> Vec<SmallParticipants> {
    let mut participants = vec![SmallParticipants::new(); variable_count];
    for (atom_id, atom) in atoms.iter().enumerate() {
        for variable in atom_variables(atom) {
            participants[variable].push(atom_id);
        }
    }
    participants
}

fn encoded_refs(prefix: &[EncodedOwned]) -> SmallEncodedRefs<'_> {
    prefix.iter().map(EncodedOwned::as_ref).collect()
}

struct HashProbeExecutor<'txn, 'input, 'query, 'plan, S: TupleSink> {
    image: &'input crate::QueryImage,
    txn: &'input ReadTxn<'txn>,
    query: &'query NormalizedQuery,
    inputs: &'input EncodedInputs,
    plan: &'plan mut ExecutionPlan,
    atom_indexes: Vec<HashAtomIndex>,
    participants_by_variable: Vec<SmallParticipants>,
    binding: EncodedBinding,
    sink: &'plan mut S,
}

impl<S: TupleSink> HashProbeExecutor<'_, '_, '_, '_, S> {
    fn static_atoms_pass(&mut self) -> Result<bool> {
        let static_atoms = self
            .plan
            .relation_atoms
            .iter()
            .enumerate()
            .filter_map(|(atom_id, atom)| atom_variables(atom).is_empty().then_some(atom_id))
            .collect::<SmallParticipants>();
        for atom_id in static_atoms {
            if !self.atom_has_matching_row(usize::MAX, atom_id)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn execute(&mut self, depth: usize) -> Result<()> {
        if depth == self.plan.variable_order_ids.len() {
            if comparisons_ready_pass(
                self.txn,
                &self.plan.comparisons,
                self.query,
                self.inputs,
                &self.binding,
                &mut self.plan.summary.counters,
            )? {
                self.plan.summary.counters.bindings_yielded += 1;
                self.sink.emit(
                    self.txn,
                    self.query,
                    &self.binding,
                    &mut self.plan.summary.counters,
                )?;
            }
            return Ok(());
        }

        let variable = self.plan.variable_order_ids[depth];
        let participants = self.participants(variable);
        if participants.is_empty() {
            return Err(Error::internal(format!(
                "variable {} is not constrained by any hash atom",
                self.query.vars[variable].name
            )));
        }
        let driver = self.choose_driver(depth, &participants)?;
        let index = self.hash_index(depth, driver)?.index.clone();
        let prefix = self.hash_prefix(depth, driver)?;
        let refs = encoded_refs(&prefix);
        let row_count = index.count(&refs);
        self.plan.summary.counters.hash_probe_calls += 1;
        if row_count == 0 {
            self.plan.summary.counters.hash_probe_misses += 1;
            return Ok(());
        }
        self.plan.summary.counters.hash_probe_hits += 1;
        self.plan.summary.counters.hash_rows_returned = self
            .plan
            .summary
            .counters
            .hash_rows_returned
            .saturating_add(row_count as u64);

        let mut emitted = BTreeSet::new();
        for row in index.rows_for_prefix(&refs) {
            self.visit_driver_row(depth, variable, &participants, driver, row, &mut emitted)?;
        }
        Ok(())
    }

    fn visit_driver_row(
        &mut self,
        depth: usize,
        variable: usize,
        participants: &[usize],
        driver: usize,
        row: RowId,
        emitted: &mut BTreeSet<EncodedOwned>,
    ) -> Result<()> {
        if !self.row_satisfies_atom(driver, row)? {
            return Ok(());
        }
        let Some(value) = self.variable_value_from_row(driver, row, variable)? else {
            return Ok(());
        };
        if !emitted.insert(value.encoded.clone()) {
            return Ok(());
        }
        if !self.binding.bind(variable, value) {
            return Ok(());
        }
        let mut keep = true;
        for atom_id in participants {
            if *atom_id == driver {
                continue;
            }
            if !self.atom_has_matching_row(depth, *atom_id)? {
                keep = false;
                break;
            }
        }
        if keep
            && comparisons_ready_pass(
                self.txn,
                &self.plan.comparisons,
                self.query,
                self.inputs,
                &self.binding,
                &mut self.plan.summary.counters,
            )?
        {
            if let Some(rows) = self.plan.summary.node_rows.get_mut(depth) {
                rows.actual_rows = rows.actual_rows.saturating_add(1);
            }
            self.plan.summary.counters.hash_distinct_emits += 1;
            self.execute(depth + 1)?;
        }
        self.binding.unbind(variable);
        Ok(())
    }

    fn participants(&self, variable: usize) -> SmallParticipants {
        self.participants_by_variable
            .get(variable)
            .cloned()
            .unwrap_or_default()
    }

    fn choose_driver(&self, depth: usize, participants: &[usize]) -> Result<usize> {
        let mut best = None;
        for atom_id in participants {
            let count = self.probe_atom_count(depth, *atom_id)?;
            if best.is_none_or(|(_, best_count)| count < best_count) {
                best = Some((*atom_id, count));
            }
        }
        best.map(|(atom_id, _)| atom_id)
            .ok_or_else(|| Error::internal("hash probe node has no driver"))
    }

    fn probe_atom_count(&self, depth: usize, atom_id: usize) -> Result<usize> {
        let index = self.hash_index(depth, atom_id)?;
        let prefix = self.hash_prefix(depth, atom_id)?;
        let refs = encoded_refs(&prefix);
        Ok(index.index.count(&refs))
    }

    fn atom_has_matching_row(&mut self, depth: usize, atom_id: usize) -> Result<bool> {
        let index = self.hash_index(depth, atom_id)?.index.clone();
        let prefix = self.hash_prefix(depth, atom_id)?;
        let refs = encoded_refs(&prefix);
        let row_count = index.count(&refs);
        self.plan.summary.counters.hash_probe_calls += 1;
        if row_count == 0 {
            self.plan.summary.counters.hash_probe_misses += 1;
            return Ok(false);
        }
        self.plan.summary.counters.hash_probe_hits += 1;
        self.plan.summary.counters.hash_rows_returned = self
            .plan
            .summary
            .counters
            .hash_rows_returned
            .saturating_add(row_count as u64);
        let mut found = false;
        let mut error = None;
        index.for_each_row(&refs, |row| match self.row_satisfies_atom(atom_id, row) {
            Ok(true) => {
                found = true;
                false
            }
            Ok(false) => true,
            Err(err) => {
                error = Some(err);
                false
            }
        });
        if let Some(error) = error {
            return Err(error);
        }
        Ok(found)
    }

    fn hash_prefix(&self, depth: usize, atom_id: usize) -> Result<SmallEncodedPrefix> {
        let atom = &self.plan.relation_atoms[atom_id];
        let index = self.hash_index(depth, atom_id)?;
        let mut prefix = SmallVec::new();
        for field in &index.fields {
            let Some(atom_field) = atom
                .fields
                .iter()
                .find(|atom_field| atom_field.field == *field)
            else {
                break;
            };
            match self.term_bound_value(&atom_field.term)? {
                Some(value) => prefix.push(value),
                None => break,
            }
        }
        Ok(prefix)
    }

    fn term_bound_value(&self, term: &NormTerm) -> Result<Option<EncodedOwned>> {
        Ok(match term {
            NormTerm::Var(variable) => self
                .binding
                .get(variable.0 as usize)
                .map(encoded_owned_for_value)
                .transpose()?,
            NormTerm::Input(input) => self.inputs.get(*input).cloned(),
            NormTerm::Literal(value) => Some(value.clone()),
            NormTerm::Wildcard => None,
        })
    }

    fn row_satisfies_atom(&self, atom_id: usize, row: RowId) -> Result<bool> {
        let atom = &self.plan.relation_atoms[atom_id];
        let relation = self.relation(atom)?;
        for field in &atom.fields {
            let bytes = relation
                .encoded_bytes(row, field.field)
                .ok_or_else(|| Error::internal("missing hash probe field"))?;
            match &field.term {
                NormTerm::Var(variable) => {
                    if let Some(bound) = self.binding.get(variable.0 as usize)
                        && bound.as_bytes() != bytes
                    {
                        return Ok(false);
                    }
                }
                NormTerm::Input(input) => {
                    let Some(input) = self.inputs.get(*input) else {
                        return Ok(false);
                    };
                    if input.as_bytes() != bytes {
                        return Ok(false);
                    }
                }
                NormTerm::Literal(value) => {
                    if value.as_bytes() != bytes {
                        return Ok(false);
                    }
                }
                NormTerm::Wildcard => {}
            }
        }
        Ok(true)
    }

    fn variable_value_from_row(
        &self,
        atom_id: usize,
        row: RowId,
        variable: usize,
    ) -> Result<Option<EncodedValue>> {
        let atom = &self.plan.relation_atoms[atom_id];
        let relation = self.relation(atom)?;
        let mut out = None;
        for field in atom
            .fields
            .iter()
            .filter(|field| matches!(field.term, NormTerm::Var(var) if var.0 as usize == variable))
        {
            let bytes = relation
                .encoded_bytes(row, field.field)
                .ok_or_else(|| Error::internal("missing hash probe variable field"))?;
            if let Some(existing) = &out {
                let existing: &EncodedValue = existing;
                if existing.as_bytes() != bytes {
                    return Ok(None);
                }
            } else {
                out = Some(EncodedValue::from_bytes(field.value_type.clone(), bytes)?);
            }
        }
        Ok(out)
    }

    fn relation(&self, atom: &NormAtom) -> Result<&RelationImage> {
        self.plan
            .relation_atoms
            .get(atom.id.0 as usize)
            .ok_or_else(|| Error::internal("missing hash probe atom"))?;
        self.image
            .relations()
            .get(atom.relation.0 as usize)
            .ok_or_else(|| Error::unknown_relation(&atom.relation_name))
    }

    fn hash_index(&self, depth: usize, atom_id: usize) -> Result<&HashAtomIndex> {
        self.atom_indexes
            .iter()
            .find(|index| index.node_id == depth && index.atom_id == atom_id)
            .or_else(|| {
                self.atom_indexes
                    .iter()
                    .find(|index| index.node_id == usize::MAX && index.atom_id == atom_id)
            })
            .ok_or_else(|| Error::internal("missing hash atom index"))
    }
}

fn encoded_owned_for_value(value: &EncodedValue) -> Result<EncodedOwned> {
    Ok(value.encoded.clone())
}

fn encoded_owned_for_width(width: usize, bytes: &[u8]) -> Result<EncodedOwned> {
    match width {
        1 => {
            Ok(EncodedOwned::One(bytes.try_into().map_err(|_| {
                Error::internal("encoded value width mismatch")
            })?))
        }
        8 => {
            Ok(EncodedOwned::Eight(bytes.try_into().map_err(|_| {
                Error::internal("encoded value width mismatch")
            })?))
        }
        16 => {
            Ok(EncodedOwned::Sixteen(bytes.try_into().map_err(|_| {
                Error::internal("encoded value width mismatch")
            })?))
        }
        width => Err(Error::internal(format!(
            "unsupported encoded value width {width}"
        ))),
    }
}

fn execute_lftj<'txn, 'query, S: TupleSink>(
    image: &crate::QueryImage,
    txn: &ReadTxn<'txn>,
    query: &'query NormalizedQuery,
    inputs: &EncodedInputs,
    plan: &mut ExecutionPlan,
    sink: &mut S,
) -> Result<()> {
    let free_join_order = plan
        .summary
        .free_join
        .nodes
        .iter()
        .flat_map(|node| node.bind_vars.iter().map(|var| var.0 as usize))
        .collect::<Vec<_>>();
    if free_join_order != plan.variable_order_ids {
        return Err(Error::internal(
            "free join node order does not match variable order",
        ));
    }
    let build_start = Instant::now();
    let build_alloc_start = allocation::snapshot();
    let atom_plans = {
        let _span = tracing::debug_span!(
            "bumbledb.query.lftj.build",
            atoms = plan.relation_atoms.len()
        )
        .entered();
        build_lftj_atom_plans(
            image,
            query,
            inputs,
            &plan.relation_atoms,
            &plan.variable_order_ids,
            &mut plan.summary.counters,
        )?
    };
    plan.summary.timings.lftj_build_micros = plan
        .summary
        .timings
        .lftj_build_micros
        .saturating_add(elapsed_micros(build_start));
    plan.summary.allocations.lftj_build = allocation_delta_since(build_alloc_start);
    if atom_plans
        .iter()
        .any(|atom| atom.variables.is_empty() && atom.row_count == 0)
    {
        return Ok(());
    }
    let runtime = LftjRuntime {
        participants_by_variable: lftj_participants_by_variable(query.vars.len(), &atom_plans),
        iters: atom_plans
            .iter()
            .map(|atom| atom.trie.as_ref().iter())
            .collect(),
    };
    let execute_start = Instant::now();
    let execute_alloc_start = allocation::snapshot();
    let result = {
        let _span =
            tracing::debug_span!("bumbledb.query.lftj.execute", variables = query.vars.len())
                .entered();
        let _sink_emit_span = tracing::debug_span!("bumbledb.query.sink.emit").entered();
        let mut executor = LftjExecutor {
            txn,
            query,
            inputs,
            plan,
            runtime,
            binding: EncodedBinding::new(query.vars.len()),
            sink,
        };
        executor.execute(0)
    };
    plan.summary.timings.lftj_execute_micros = plan
        .summary
        .timings
        .lftj_execute_micros
        .saturating_add(elapsed_micros(execute_start));
    plan.summary.allocations.lftj_execute = allocation_delta_since(execute_alloc_start);
    result
}

fn lftj_participants_by_variable(
    variable_count: usize,
    atom_plans: &[LftjAtomPlan],
) -> Vec<SmallParticipants> {
    let mut participants = vec![SmallParticipants::new(); variable_count];
    for (atom_id, atom) in atom_plans.iter().enumerate() {
        for variable in &atom.variables {
            participants[*variable].push(atom_id);
        }
    }
    participants
}

struct LftjExecutor<'txn, 'input, 'query, 'plan, 'image, S: TupleSink> {
    txn: &'input ReadTxn<'txn>,
    query: &'query NormalizedQuery,
    inputs: &'input EncodedInputs,
    plan: &'plan mut ExecutionPlan,
    runtime: LftjRuntime<'image>,
    binding: EncodedBinding,
    sink: &'plan mut S,
}

impl<S: TupleSink> LftjExecutor<'_, '_, '_, '_, '_, S> {
    fn execute(&mut self, depth: usize) -> Result<()> {
        if depth == self.plan.variable_order_ids.len() {
            if comparisons_ready_pass(
                self.txn,
                &self.plan.comparisons,
                self.query,
                self.inputs,
                &self.binding,
                &mut self.plan.summary.counters,
            )? {
                self.plan.summary.counters.bindings_yielded += 1;
                self.sink.emit(
                    self.txn,
                    self.query,
                    &self.binding,
                    &mut self.plan.summary.counters,
                )?;
            }
            return Ok(());
        }

        let variable = self.plan.variable_order_ids[depth];
        let participants = self.participants(variable);
        if participants.is_empty() {
            return Err(Error::internal(format!(
                "variable {} is not constrained by any trie atom",
                self.query.vars[variable].name
            )));
        }

        for atom_id in &participants {
            self.runtime.iters[*atom_id].open();
            self.plan.summary.counters.trie_open += 1;
        }

        let mut leapfrog = LeapfrogState::new(participants.clone());
        leapfrog.init(&mut self.runtime.iters, &mut self.plan.summary.counters)?;
        while !leapfrog.at_end {
            let value = leapfrog.key(&self.runtime.iters, &mut self.plan.summary.counters)?;
            self.plan.summary.counters.variable_candidates += 1;
            if self.binding.bind(
                variable,
                EncodedValue::new(self.query.vars[variable].value_type.clone(), value),
            ) {
                let keep = comparisons_ready_pass(
                    self.txn,
                    &self.plan.comparisons,
                    self.query,
                    self.inputs,
                    &self.binding,
                    &mut self.plan.summary.counters,
                )?;
                if keep {
                    if let Some(rows) = self.plan.summary.node_rows.get_mut(depth) {
                        rows.actual_rows = rows.actual_rows.saturating_add(1);
                    }
                    self.execute(depth + 1)?;
                }
                self.binding.unbind(variable);
            }
            leapfrog.next(&mut self.runtime.iters, &mut self.plan.summary.counters)?;
        }

        for atom_id in participants.iter().rev() {
            self.runtime.iters[*atom_id].up();
            self.plan.summary.counters.trie_up += 1;
        }
        Ok(())
    }

    fn participants(&self, variable: usize) -> SmallParticipants {
        self.runtime
            .participants_by_variable
            .get(variable)
            .cloned()
            .unwrap_or_default()
    }
}

struct LeapfrogState {
    iter_ids: SmallParticipants,
    p: usize,
    at_end: bool,
}

impl LeapfrogState {
    fn new(iter_ids: SmallParticipants) -> Self {
        Self {
            iter_ids,
            p: 0,
            at_end: false,
        }
    }

    fn init(
        &mut self,
        iters: &mut [crate::SortedTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<()> {
        if self.iter_ids.iter().any(|id| iters[*id].at_end()) {
            self.at_end = true;
            return Ok(());
        }
        self.sort_iter_ids(iters, counters)?;
        self.p = 0;
        self.search(iters, counters)
    }

    fn sort_iter_ids(
        &mut self,
        iters: &[crate::SortedTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<()> {
        let mut error = None;
        self.iter_ids.sort_by(|left, right| {
            if error.is_some() {
                return std::cmp::Ordering::Equal;
            }
            let Some(left) = key_owned_opt(&iters[*left], counters) else {
                error = Some(missing_trie_key_error());
                return std::cmp::Ordering::Equal;
            };
            let Some(right) = key_owned_opt(&iters[*right], counters) else {
                error = Some(missing_trie_key_error());
                return std::cmp::Ordering::Equal;
            };
            left.cmp(&right)
        });
        if let Some(error) = error {
            return Err(error);
        }
        Ok(())
    }

    fn key(
        &self,
        iters: &[crate::SortedTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<EncodedOwned> {
        self.iter_ids
            .first()
            .map(|id| key_owned(&iters[*id], counters))
            .transpose()?
            .ok_or_else(|| Error::internal("leapfrog join has no iterators"))
    }

    fn next(
        &mut self,
        iters: &mut [crate::SortedTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<()> {
        if self.at_end {
            return Ok(());
        }
        let id = self.iter_ids[self.p];
        iters[id].next();
        counters.trie_next += 1;
        if iters[id].at_end() {
            self.at_end = true;
            return Ok(());
        }
        self.p = (self.p + 1) % self.iter_ids.len();
        self.search(iters, counters)
    }

    fn search(
        &mut self,
        iters: &mut [crate::SortedTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<()> {
        if self.iter_ids.is_empty() || self.at_end {
            return Ok(());
        }
        if self.iter_ids.len() == 1 {
            return Ok(());
        }
        let Some(mut max) = key_owned_opt(
            &iters[self.iter_ids[(self.p + self.iter_ids.len() - 1) % self.iter_ids.len()]],
            counters,
        ) else {
            return Err(missing_trie_key_error());
        };
        loop {
            let id = self.iter_ids[self.p];
            let Some(current) = key_owned_opt(&iters[id], counters) else {
                return Err(missing_trie_key_error());
            };
            if current == max {
                return Ok(());
            }
            iters[id].seek(max.as_ref());
            counters.trie_seek += 1;
            if iters[id].at_end() {
                self.at_end = true;
                return Ok(());
            }
            let Some(next_max) = key_owned_opt(&iters[id], counters) else {
                return Err(missing_trie_key_error());
            };
            max = next_max;
            self.p = (self.p + 1) % self.iter_ids.len();
        }
    }
}

fn key_owned(
    iter: &crate::SortedTrieIter<'_>,
    counters: &mut PlanCounters,
) -> Result<EncodedOwned> {
    key_owned_opt(iter, counters).ok_or_else(missing_trie_key_error)
}

fn key_owned_opt(
    iter: &crate::SortedTrieIter<'_>,
    counters: &mut PlanCounters,
) -> Option<EncodedOwned> {
    let key = iter.key()?;
    counters.trie_key_reads += 1;
    Some(EncodedOwned::from_ref(key))
}

fn missing_trie_key_error() -> Error {
    Error::internal("trie key requested for exhausted iterator")
}

fn build_lftj_atom_plans(
    image: &crate::QueryImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atoms: &[NormAtom],
    variable_order_ids: &[usize],
    counters: &mut PlanCounters,
) -> Result<Vec<LftjAtomPlan>> {
    atoms
        .iter()
        .map(|atom| build_lftj_atom_plan(image, query, inputs, atom, variable_order_ids, counters))
        .collect()
}

fn build_lftj_atom_plan(
    image: &crate::QueryImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variable_order_ids: &[usize],
    counters: &mut PlanCounters,
) -> Result<LftjAtomPlan> {
    let source = image
        .relations()
        .get(atom.relation.0 as usize)
        .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
    let variables = atom_variables_in_plan_order(atom, variable_order_ids);
    let cache_key = lftj_atom_cache_key(atom, &variables, variable_order_ids, inputs);
    let source_row_count = source.row_count;
    let cached = image.cached_sorted_trie(cache_key, || {
        build_lftj_sorted_trie(source, query, inputs, atom, &variables)
    })?;
    if cached.hit {
        counters.sorted_trie_cache_hits += 1;
    } else {
        counters.sorted_trie_cache_misses += 1;
        counters.sorted_trie_builds += 1;
        counters.sorted_trie_build_micros = counters
            .sorted_trie_build_micros
            .saturating_add(cached.build_micros as u64);
        counters.atom_temp_relation_builds += 1;
        counters.atom_temp_relation_source_rows = counters
            .atom_temp_relation_source_rows
            .saturating_add(source_row_count as u64);
        counters.atom_temp_relation_rows = counters
            .atom_temp_relation_rows
            .saturating_add(cached.index.stats.row_count as u64);
    }
    Ok(LftjAtomPlan {
        variables,
        trie: cached.index.clone(),
        row_count: cached.index.stats.row_count,
    })
}

fn build_lftj_sorted_trie(
    source: &RelationImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
) -> Result<SortedTrieIndex> {
    let fields = variables
        .iter()
        .enumerate()
        .map(|(id, variable)| crate::FieldImage {
            id: FieldId(id as u16),
            name: query.vars[*variable].name.clone(),
            value_type: query.vars[*variable].value_type.clone(),
            width: query.vars[*variable].value_type.encoded_width(),
        })
        .collect::<Vec<_>>();
    let mut raw_columns = vec![Vec::<Vec<u8>>::new(); variables.len()];
    let mut included_rows = 0usize;

    for row in 0..source.row_count {
        let row = RowId(row as u32);
        let Some(values) = atom_row_values(source, query, inputs, atom, row, variables)? else {
            continue;
        };
        included_rows += 1;
        for (column, bytes) in values.into_iter().enumerate() {
            raw_columns[column].push(bytes);
        }
    }

    let row_count = if variables.is_empty() {
        included_rows
    } else {
        raw_columns[0].len()
    };
    let encoded_column_bytes = raw_columns.iter().flatten().map(Vec::len).sum::<usize>();
    let columns = fields
        .iter()
        .zip(raw_columns)
        .map(|(field, raw_column)| {
            crate::ColumnImage::from_query_image_bytes(field.id, field.width, raw_column)
        })
        .collect::<Result<Vec<_>>>()?;
    let relation = RelationImage {
        id: source.id,
        name: atom.relation_name.clone(),
        row_count,
        fields,
        columns,
        sorted_index_count: 0,
        hash_index_count: 0,
        stats: RelationStats {
            row_count,
            field_count: variables.len(),
            encoded_column_bytes,
        },
    };
    let trie = crate::query_image::build_sorted_trie_index(
        &relation,
        IndexSpec::new(
            format!("{}_lftj", atom.relation_name),
            (0..variables.len()).map(|id| FieldId(id as u16)),
        ),
    )?;
    Ok(trie)
}

fn lftj_atom_cache_key(
    atom: &NormAtom,
    variables: &[usize],
    variable_order_ids: &[usize],
    inputs: &EncodedInputs,
) -> String {
    let mut key = String::new();
    let _ = write!(
        key,
        "relation={};atom={};vars={:?};order={:?};fields=",
        atom.relation.0, atom.id.0, variables, variable_order_ids
    );
    for field in &atom.fields {
        let _ = write!(key, "{}:", field.field.0);
        match &field.term {
            NormTerm::Var(variable) => {
                let _ = write!(key, "v{}", variable.0);
            }
            NormTerm::Input(input) => {
                let _ = write!(key, "i{}=", input.0);
                if let Some(value) = inputs.get(*input) {
                    append_hex(&mut key, value.as_bytes());
                } else {
                    key.push_str("missing");
                }
            }
            NormTerm::Literal(value) => {
                key.push_str("l=");
                append_hex(&mut key, value.as_bytes());
            }
            NormTerm::Wildcard => key.push('_'),
        }
        key.push(';');
    }
    key
}

fn append_hex(out: &mut String, bytes: &[u8]) {
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
}

fn atom_variables_in_plan_order(atom: &NormAtom, variable_order_ids: &[usize]) -> Vec<usize> {
    variable_order_ids
        .iter()
        .copied()
        .filter(|variable| atom_contains_variable(atom, *variable))
        .collect()
}

fn atom_row_values(
    relation: &RelationImage,
    _query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    row: RowId,
    variables: &[usize],
) -> Result<Option<Vec<Vec<u8>>>> {
    let mut values_by_variable = BTreeMap::<usize, Vec<u8>>::new();
    for field in &atom.fields {
        let bytes = relation
            .encoded_bytes(row, field.field)
            .ok_or_else(|| Error::internal("missing atom field in relation image"))?;
        match &field.term {
            NormTerm::Var(variable) => {
                let variable = variable.0 as usize;
                if let Some(existing) = values_by_variable.get(&variable) {
                    if existing.as_slice() != bytes {
                        return Ok(None);
                    }
                } else {
                    values_by_variable.insert(variable, bytes.to_vec());
                }
            }
            NormTerm::Input(input) => {
                let input = inputs
                    .get(*input)
                    .ok_or_else(|| Error::internal("missing normalized input"))?;
                if input.as_bytes() != bytes {
                    return Ok(None);
                }
            }
            NormTerm::Literal(literal) => {
                if literal.as_bytes() != bytes {
                    return Ok(None);
                }
            }
            NormTerm::Wildcard => {}
        }
    }
    variables
        .iter()
        .map(|variable| {
            values_by_variable
                .get(variable)
                .cloned()
                .ok_or_else(|| Error::internal("missing LFTJ variable value"))
        })
        .collect::<Result<Vec<_>>>()
        .map(Some)
}

fn plan_query(
    schema: &StorageSchema,
    query: &mut NormalizedQuery,
    image: &crate::QueryImage,
    query_image_cache: QueryImageCacheDiagnostics,
) -> Result<ExecutionPlan> {
    let _span = tracing::debug_span!("bumbledb.query.plan").entered();
    let (stats, variable_order_ids, variable_costs) = {
        let relation_atoms = query.atoms.iter().collect::<Vec<_>>();
        let comparisons = query.predicates.iter().collect::<Vec<_>>();
        let stats = {
            let _span =
                tracing::debug_span!("bumbledb.query.plan.stats", atoms = relation_atoms.len())
                    .entered();
            PlannerStats::collect(schema, image, &relation_atoms)?
        };
        let (variable_order_ids, variable_costs) = {
            let _span = tracing::debug_span!(
                "bumbledb.query.plan.variable_order",
                variables = query.vars.len()
            )
            .entered();
            choose_variable_order(schema, query, &relation_atoms, &comparisons, &stats)?
        };
        (stats, variable_order_ids, variable_costs)
    };
    attach_predicate_depths(query, &variable_order_ids);
    let relation_atoms = query.atoms.iter().collect::<Vec<_>>();
    let variable_order = variable_order_ids
        .iter()
        .map(|id| query.vars[*id].name.clone())
        .collect::<Vec<_>>();
    let variable_estimates = variable_costs
        .iter()
        .map(|cost| VariableEstimate {
            variable: query.vars[cost.variable].name.clone(),
            estimated_candidates: cost.estimated_candidates,
            static_constraints: cost.static_constraints,
            bound_constraints: cost.bound_constraints,
            relation_constraints: cost.relation_constraints,
            access: cost.access.clone(),
            reason: cost.reason.clone(),
        })
        .collect::<Vec<_>>();
    let node_rows = variable_order_ids
        .iter()
        .enumerate()
        .map(|(node_id, variable)| NodeRowEstimate {
            node: NodeId(node_id as u16),
            variable: query.vars[*variable].name.clone(),
            estimated_rows: variable_costs
                .get(node_id)
                .map_or(1, |cost| cost.estimated_candidates),
            actual_rows: 0,
        })
        .collect::<Vec<_>>();
    let missing_indexes = missing_index_recommendations(schema, query, &relation_atoms)?;
    let (free_join, optimizer) = {
        let _span = tracing::debug_span!(
            "bumbledb.query.plan.optimize_free_join",
            atoms = relation_atoms.len(),
            variables = variable_order_ids.len()
        )
        .entered();
        optimize_free_join_plan(
            schema,
            query,
            &relation_atoms,
            &variable_order_ids,
            &variable_costs,
            &stats,
        )?
    };
    free_join.validate()?;
    let node_timings = query_node_timings(&free_join, &node_rows);
    let planner_stats = image.planner_stats_diagnostics();

    let uses_indexed_multiway_join = relation_atoms.len() > 1;
    Ok(ExecutionPlan {
        variable_order_ids,
        relation_atoms: query.atoms.clone(),
        comparisons: query.predicates.clone(),
        summary: QueryPlan {
            variable_order,
            variable_estimates,
            missing_indexes,
            optimizer,
            query_image_cache,
            planner_stats,
            node_rows,
            node_timings,
            free_join,
            runtime_kind: QueryRuntimeKind::Unknown,
            timings: QueryTimings::default(),
            allocations: QueryAllocationStats::default(),
            counters: PlanCounters::default(),
            uses_indexed_multiway_join,
        },
    })
}

fn query_node_timings(
    free_join: &FreeJoinPlan,
    node_rows: &[NodeRowEstimate],
) -> Vec<QueryNodeTiming> {
    free_join
        .nodes
        .iter()
        .map(|node| {
            let rows = node_rows.get(node.id.0 as usize);
            QueryNodeTiming {
                node: node.id,
                implementation: node.implementation,
                bind_vars: node.bind_vars.clone(),
                estimated_rows: rows.map_or(0, |rows| rows.estimated_rows),
                actual_rows: rows.map_or(0, |rows| rows.actual_rows),
                execute_micros: 0,
            }
        })
        .collect()
}

fn choose_variable_order(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
    comparisons: &[&NormPredicate],
    stats: &PlannerStats,
) -> Result<(Vec<usize>, Vec<VariableCost>)> {
    let mut remaining = (0..query.vars.len()).collect::<BTreeSet<_>>();
    let mut bound = BTreeSet::new();
    let mut order = Vec::new();
    let mut costs = Vec::new();

    while !remaining.is_empty() {
        let mut candidates = remaining
            .iter()
            .map(|variable| {
                estimate_variable_cost(schema, atoms, comparisons, stats, &bound, *variable)
            })
            .collect::<Result<Vec<_>>>()?;
        candidates.sort_by_key(|cost| {
            (
                cost.estimated_candidates,
                std::cmp::Reverse(cost.static_constraints),
                std::cmp::Reverse(cost.bound_constraints),
                std::cmp::Reverse(cost.relation_constraints),
                std::cmp::Reverse(cost.degree),
                query.vars[cost.variable].name.clone(),
            )
        });
        let best = candidates
            .into_iter()
            .next()
            .ok_or_else(|| Error::internal("query has no remaining variables"))?;
        remaining.remove(&best.variable);
        bound.insert(best.variable);
        order.push(best.variable);
        costs.push(best);
    }

    Ok((order, costs))
}

fn estimate_variable_cost(
    schema: &StorageSchema,
    atoms: &[&NormAtom],
    comparisons: &[&NormPredicate],
    stats: &PlannerStats,
    bound: &BTreeSet<usize>,
    variable: usize,
) -> Result<VariableCost> {
    let atom_infos = atoms
        .iter()
        .copied()
        .filter(|atom| atom_contains_variable(atom, variable))
        .map(|atom| {
            let relation_constraints = atom_bound_constraint_count(atom, variable, bound);
            let static_constraints = atom_static_constraint_count(atom, variable)
                + comparison_static_constraint_count(comparisons, variable, bound);
            let has_unbound_other = atom_has_unbound_other_variable_id(atom, variable, bound);
            (
                atom,
                relation_constraints + static_constraints,
                has_unbound_other,
            )
        })
        .collect::<Vec<_>>();
    let has_constrained_stream = atom_infos.iter().any(|(_, strength, _)| *strength > 0);
    let has_unconstrained_payload_stream = atom_infos
        .iter()
        .any(|(_, strength, has_unbound_other)| *strength == 0 && *has_unbound_other);
    let mut estimates = Vec::new();
    let mut relation_constraints = 0usize;
    let mut static_constraints = comparison_static_constraint_count(comparisons, variable, bound);
    let mut bound_constraints = comparison_bound_constraint_count(comparisons, variable, bound);

    for (atom, strength, has_unbound_other) in atom_infos {
        relation_constraints += 1;
        static_constraints += atom_static_constraint_count(atom, variable);
        bound_constraints += atom_bound_constraint_count(atom, variable, bound);
        if has_constrained_stream && strength == 0 && has_unbound_other {
            continue;
        }
        estimates.push(estimate_atom_variable_access(
            schema, stats, bound, atom, variable,
        )?);
    }

    let degree = atoms
        .iter()
        .filter(|atom| atom_contains_variable(atom, variable))
        .count();
    let best_access = estimates.into_iter().min_by_key(|estimate| {
        (
            estimate.estimated_rows,
            std::cmp::Reverse(estimate.prefix_len),
            std::cmp::Reverse(estimate.current_is_next),
            estimate.access_label(),
        )
    });
    let mut estimated_candidates = best_access
        .as_ref()
        .map(|estimate| estimate.estimated_rows)
        .unwrap_or(u64::MAX / 4)
        .max(1);
    if static_constraints == 0
        && bound_constraints == 0
        && degree == 1
        && has_unconstrained_payload_stream
    {
        estimated_candidates = estimated_candidates.max(
            best_access
                .as_ref()
                .map(|estimate| stats.relation_rows(&estimate.relation))
                .unwrap_or(u64::MAX / 8),
        );
    }
    let access = best_access
        .as_ref()
        .map(AccessEstimate::access_label)
        .unwrap_or_else(|| "unindexed".to_owned());
    let reason = best_access
        .as_ref()
        .map(AccessEstimate::reason)
        .unwrap_or_else(|| "no relation stats for variable".to_owned());

    Ok(VariableCost {
        variable,
        estimated_candidates,
        static_constraints,
        bound_constraints,
        relation_constraints,
        degree,
        access,
        reason,
    })
}

fn estimate_atom_variable_access(
    schema: &StorageSchema,
    stats: &PlannerStats,
    bound: &BTreeSet<usize>,
    atom: &NormAtom,
    variable: usize,
) -> Result<AccessEstimate> {
    let paths = schema.access_paths(&atom.relation_name)?;
    let relation_rows = stats.relation_rows(&atom.relation_name);
    let mut best: Option<AccessEstimate> = None;

    for path in paths {
        if !path.components.iter().any(|component| {
            atom.fields.iter().any(|field| {
                field.field_name == component.field_name
                    && matches!(field.term, NormTerm::Var(id) if id.0 as usize == variable)
            })
        }) {
            continue;
        }

        let mut prefix_len = 0usize;
        let mut current_is_next = false;
        for field_name in &path.leading_fields {
            let Some(field) = atom
                .fields
                .iter()
                .find(|field| &field.field_name == field_name)
            else {
                break;
            };
            if matches!(field.term, NormTerm::Var(id) if id.0 as usize == variable) {
                current_is_next = true;
                break;
            }
            if field_is_bound_for_estimate(field, bound) {
                prefix_len += 1;
            } else {
                break;
            }
        }

        let Some(index_stats) = stats.index_stats(&atom.relation_name, &path.index_name) else {
            continue;
        };
        let mut estimate = if current_is_next {
            if prefix_len == 0 {
                index_stats
                    .distinct_by_depth
                    .first()
                    .copied()
                    .unwrap_or(index_stats.rows)
                    .max(1) as u64
            } else {
                index_stats.fanout_after_prefix(prefix_len)
            }
        } else {
            index_stats.estimated_rows_for_prefix(prefix_len)
        };
        if path.kind == IndexKind::Unique
            && current_is_next
            && prefix_len + 1 == path.leading_fields.len()
        {
            estimate = estimate.min(1);
        }
        let variable_field_stats = atom
            .fields
            .iter()
            .find(|field| matches!(field.term, NormTerm::Var(id) if id.0 as usize == variable))
            .and_then(|field| stats.field_stats(&atom.relation_name, &field.field_name));
        let distinct = index_stats
            .distinct_by_depth
            .get(prefix_len.saturating_sub(1))
            .copied()
            .unwrap_or(1);
        let candidate = AccessEstimate {
            relation: atom.relation_name.clone(),
            index: path.index_name,
            access: index_stats.index,
            estimated_rows: estimate.max(1),
            prefix_len,
            current_is_next,
            distinct,
            avg_fanout: index_stats.fanout_after_prefix(prefix_len),
            max_fanout: index_stats.max_fanout_after_prefix(prefix_len),
            variable_distinct: variable_field_stats.map_or(1, |stats| stats.distinct),
            has_min: variable_field_stats.is_some_and(|stats| stats.min.is_some()),
            has_max: variable_field_stats.is_some_and(|stats| stats.max.is_some()),
            heavy_hitters: variable_field_stats.map_or(0, |stats| stats.heavy_hitters.len()),
        };
        if best.as_ref().is_none_or(|best| {
            (
                candidate.estimated_rows,
                std::cmp::Reverse(candidate.prefix_len),
                std::cmp::Reverse(candidate.current_is_next),
                candidate.access_label(),
            ) < (
                best.estimated_rows,
                std::cmp::Reverse(best.prefix_len),
                std::cmp::Reverse(best.current_is_next),
                best.access_label(),
            )
        }) {
            best = Some(candidate);
        }
    }

    Ok(best.unwrap_or_else(|| AccessEstimate {
        relation: atom.relation_name.clone(),
        index: "full_scan".to_owned(),
        access: AccessId(0),
        estimated_rows: relation_rows.saturating_mul(4).max(1),
        prefix_len: 0,
        current_is_next: false,
        distinct: 1,
        avg_fanout: relation_rows.max(1),
        max_fanout: relation_rows as usize,
        variable_distinct: 1,
        has_min: false,
        has_max: false,
        heavy_hitters: 0,
    }))
}

fn field_is_bound_for_estimate(field: &NormAtomField, bound: &BTreeSet<usize>) -> bool {
    match field.term {
        NormTerm::Var(variable) => bound.contains(&(variable.0 as usize)),
        NormTerm::Input(_) | NormTerm::Literal(_) => true,
        NormTerm::Wildcard => false,
    }
}

fn atom_static_constraint_count(atom: &NormAtom, variable: usize) -> usize {
    atom.fields
        .iter()
        .filter(|field| {
            !matches!(field.term, NormTerm::Var(id) if id.0 as usize == variable)
                && matches!(field.term, NormTerm::Input(_) | NormTerm::Literal(_))
        })
        .count()
}

fn atom_bound_constraint_count(atom: &NormAtom, variable: usize, bound: &BTreeSet<usize>) -> usize {
    atom.fields
        .iter()
        .filter(|field| {
            matches!(field.term, NormTerm::Var(id) if id.0 as usize != variable && bound.contains(&(id.0 as usize)))
        })
        .count()
}

fn atom_has_unbound_other_variable_id(
    atom: &NormAtom,
    variable: usize,
    bound: &BTreeSet<usize>,
) -> bool {
    atom.fields.iter().any(|field| {
        matches!(field.term, NormTerm::Var(id) if id.0 as usize != variable && !bound.contains(&(id.0 as usize)))
    })
}

fn comparison_static_constraint_count(
    comparisons: &[&NormPredicate],
    variable: usize,
    bound: &BTreeSet<usize>,
) -> usize {
    comparisons
        .iter()
        .filter(|comparison| comparison_constrains_variable(comparison, variable, bound, true))
        .count()
}

fn comparison_bound_constraint_count(
    comparisons: &[&NormPredicate],
    variable: usize,
    bound: &BTreeSet<usize>,
) -> usize {
    comparisons
        .iter()
        .filter(|comparison| comparison_constrains_variable(comparison, variable, bound, false))
        .count()
}

fn comparison_constrains_variable(
    comparison: &NormPredicate,
    variable: usize,
    bound: &BTreeSet<usize>,
    static_only: bool,
) -> bool {
    let left_is_var =
        matches!(comparison.operands[0], NormOperand::Var(id) if id.0 as usize == variable);
    let right_is_var =
        matches!(comparison.operands[1], NormOperand::Var(id) if id.0 as usize == variable);
    if left_is_var {
        operand_constrains_for_estimate(&comparison.operands[1], bound, static_only)
    } else if right_is_var {
        operand_constrains_for_estimate(&comparison.operands[0], bound, static_only)
    } else {
        false
    }
}

fn operand_constrains_for_estimate(
    operand: &NormOperand,
    bound: &BTreeSet<usize>,
    static_only: bool,
) -> bool {
    match operand {
        NormOperand::Var(variable) => !static_only && bound.contains(&(variable.0 as usize)),
        NormOperand::Input(_) | NormOperand::Literal(_) => static_only,
    }
}

fn missing_index_recommendations(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
) -> Result<Vec<MissingIndexRecommendation>> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut variable_degree = vec![0usize; query.vars.len()];
    for atom in atoms {
        for variable in atom_variables(atom) {
            variable_degree[variable] += 1;
        }
    }
    for atom in atoms {
        let (_, relation) = schema.relation(&atom.relation_name)?;
        for field in &atom.fields {
            match field.term {
                NormTerm::Input(_) | NormTerm::Literal(_) => {
                    if has_leading_index(schema, &atom.relation_name, &field.field_name)? {
                        continue;
                    }
                    let fields = recommended_index_fields(relation, &field.field_name);
                    if seen.insert((atom.relation_name.clone(), fields.clone())) {
                        out.push(MissingIndexRecommendation {
                            relation: atom.relation_name.clone(),
                            fields,
                            reason: "StaticPredicate: chosen prefix has no leading index"
                                .to_owned(),
                        });
                    }
                }
                NormTerm::Var(variable) if variable_degree[variable.0 as usize] > 1 => {
                    if has_leading_index(schema, &atom.relation_name, &field.field_name)? {
                        continue;
                    }
                    let fields = recommended_index_fields(relation, &field.field_name);
                    if seen.insert((atom.relation_name.clone(), fields.clone())) {
                        out.push(MissingIndexRecommendation {
                            relation: atom.relation_name.clone(),
                            fields,
                            reason: "JoinPrefix: joined variable has no leading index".to_owned(),
                        });
                    }
                }
                NormTerm::Var(_) | NormTerm::Wildcard => {}
            }
        }
    }
    Ok(out)
}

fn has_leading_index(schema: &StorageSchema, relation: &str, field: &str) -> Result<bool> {
    Ok(schema.access_paths(relation)?.iter().any(|path| {
        path.leading_fields
            .first()
            .is_some_and(|leading| leading == field)
    }))
}

fn recommended_index_fields(
    relation: &bumbledb_core::schema::RelationDescriptor,
    field: &str,
) -> Vec<String> {
    let mut fields = vec![field.to_owned()];
    for primary in &relation.primary_key.fields {
        if !fields.iter().any(|field| field == primary) {
            fields.push(primary.clone());
        }
    }
    fields
}

fn optimize_free_join_plan(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
    variable_order_ids: &[usize],
    variable_costs: &[VariableCost],
    stats: &PlannerStats,
) -> Result<(FreeJoinPlan, OptimizerTrace)> {
    let cyclic = is_cyclic_multiway_query(query, atoms);
    let mut candidates = Vec::new();

    let lftj_impls = vec![NodeImpl::SortedLeapfrog; variable_order_ids.len()];
    candidates.push(build_plan_candidate(
        "pure_lftj",
        schema,
        query,
        atoms,
        variable_order_ids,
        variable_costs,
        stats,
        lftj_impls,
        cyclic,
    )?);

    let probe_impls = probe_node_impls(schema, atoms, variable_order_ids, stats, cyclic)?;
    candidates.push(build_plan_candidate(
        "hash_probe",
        schema,
        query,
        atoms,
        variable_order_ids,
        variable_costs,
        stats,
        probe_impls,
        cyclic,
    )?);

    let hybrid_impls = hybrid_node_impls(schema, atoms, variable_order_ids, stats, cyclic)?;
    candidates.push(build_plan_candidate(
        "hybrid",
        schema,
        query,
        atoms,
        variable_order_ids,
        variable_costs,
        stats,
        hybrid_impls,
        cyclic,
    )?);

    if has_aggregate(query) {
        candidates.push(build_plan_candidate(
            "aggregate_pushdown",
            schema,
            query,
            atoms,
            variable_order_ids,
            variable_costs,
            stats,
            vec![NodeImpl::SortedLeapfrog; variable_order_ids.len()],
            cyclic,
        )?);
    }

    candidates.sort_by_key(|candidate| candidate.cost.clone());
    let chosen = candidates
        .first()
        .ok_or_else(|| Error::internal("no optimizer plan candidates"))?
        .name
        .clone();
    let plan = candidates
        .iter()
        .find(|candidate| candidate.name == chosen)
        .ok_or_else(|| Error::internal("chosen optimizer candidate missing"))?
        .plan
        .clone();
    let trace_candidates = candidates
        .into_iter()
        .map(|candidate| PlanCandidate {
            selected: candidate.name == chosen,
            rejected_reason: if candidate.name == chosen {
                "selected minimum stable cost".to_owned()
            } else {
                "higher stable cost".to_owned()
            },
            name: candidate.name,
            implementations: candidate.implementations,
            cost: candidate.cost,
        })
        .collect::<Vec<_>>();

    Ok((
        plan,
        OptimizerTrace {
            chosen,
            candidates: trace_candidates,
        },
    ))
}

#[derive(Clone, Debug)]
struct OptimizerCandidate {
    name: String,
    implementations: Vec<NodeImpl>,
    cost: CostKey,
    plan: FreeJoinPlan,
}

#[expect(
    clippy::too_many_arguments,
    reason = "optimizer candidate builder mirrors the full planning context"
)]
fn build_plan_candidate(
    name: &str,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
    variable_order_ids: &[usize],
    variable_costs: &[VariableCost],
    stats: &PlannerStats,
    implementations: Vec<NodeImpl>,
    cyclic: bool,
) -> Result<OptimizerCandidate> {
    let estimates = estimate_free_join_plan(name, query, variable_costs, &implementations, cyclic);
    let cost = CostKey {
        estimated_micros: estimates
            .iterator_ops
            .saturating_add(estimates.hash_probe_rows)
            .saturating_add(estimates.hash_build_rows / 64)
            .saturating_add(estimates.materialized_values),
        memory_bytes: estimates.memory_bytes,
        materialization_penalty: estimates.materialized_values,
        tie_breaker: format!(
            "{}:{}",
            name,
            implementations
                .iter()
                .map(|implementation| format!("{implementation:?}"))
                .collect::<Vec<_>>()
                .join(",")
        ),
    };
    let plan = build_free_join_plan(
        schema,
        query,
        atoms,
        variable_order_ids,
        &implementations,
        stats,
        estimates,
    )?;
    Ok(OptimizerCandidate {
        name: name.to_owned(),
        implementations,
        cost,
        plan,
    })
}

fn build_free_join_plan(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
    variable_order_ids: &[usize],
    implementations: &[NodeImpl],
    stats: &PlannerStats,
    estimates: PlanEstimates,
) -> Result<FreeJoinPlan> {
    let mut nodes = Vec::new();
    let mut bound = BTreeSet::new();
    for (node_id, variable) in variable_order_ids.iter().enumerate() {
        let var_id = VarId(*variable as u16);
        let subatoms = atoms
            .iter()
            .enumerate()
            .map(|(atom_id, atom)| {
                let fields = atom
                    .fields
                    .iter()
                    .filter(
                        |field| matches!(field.term, NormTerm::Var(id) if id.0 as usize == *variable),
                    )
                    .map(|field| field.field)
                    .collect::<Vec<_>>();
                if fields.is_empty() {
                    return Ok(None);
                }
                let access =
                    estimate_atom_variable_access(schema, stats, &bound, atom, *variable)?.access;
                Ok(Some(SubAtom {
                    atom_id: AtomId(atom_id as u16),
                    relation: atom.relation,
                    vars: vec![var_id; fields.len()],
                    fields,
                    access,
                }))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        nodes.push(PlanNode {
            id: NodeId(node_id as u16),
            bind_vars: vec![var_id],
            subatoms,
            implementation: implementations
                .get(node_id)
                .copied()
                .unwrap_or(NodeImpl::SortedLeapfrog),
            payload: payload_demand(query),
        });
        bound.insert(*variable);
    }

    Ok(FreeJoinPlan {
        nodes,
        output: output_plan(query),
        estimates,
    })
}

fn estimate_free_join_plan(
    name: &str,
    query: &NormalizedQuery,
    variable_costs: &[VariableCost],
    implementations: &[NodeImpl],
    cyclic: bool,
) -> PlanEstimates {
    let mut iterator_ops = 0u64;
    let mut hash_build_rows = 0u64;
    let mut hash_probe_rows = 0u64;
    for (cost, implementation) in variable_costs.iter().zip(implementations) {
        let mut variable_ops = cost.estimated_candidates.max(1);
        match implementation {
            NodeImpl::SortedLeapfrog => {
                variable_ops = variable_ops.saturating_mul(if cyclic { 1 } else { 3 });
            }
            NodeImpl::HashProbe => {
                hash_probe_rows = hash_probe_rows.saturating_add(cost.estimated_candidates.max(1));
                hash_build_rows = hash_build_rows.saturating_add(cost.estimated_candidates.max(1));
            }
            NodeImpl::Hybrid => {
                variable_ops = variable_ops.saturating_mul(2);
                hash_probe_rows =
                    hash_probe_rows.saturating_add(cost.estimated_candidates.max(1) / 2);
            }
            NodeImpl::VectorLoop
            | NodeImpl::ExistenceCheck
            | NodeImpl::Product
            | NodeImpl::AggregateSink => {
                variable_ops = variable_ops.saturating_mul(4);
            }
        }
        iterator_ops = iterator_ops.saturating_add(variable_ops);
    }

    if cyclic && name != "pure_lftj" && name != "aggregate_pushdown" {
        iterator_ops = iterator_ops.saturating_mul(8);
    }

    if name == "hybrid" {
        iterator_ops = iterator_ops.saturating_add(25);
    }

    let output_rows = estimate_output_rows(query, variable_costs);
    let materialized_values = estimate_materialized_values(query, output_rows);
    let memory_bytes = (hash_build_rows as usize)
        .saturating_mul(32)
        .saturating_add(materialized_values as usize * 16);

    PlanEstimates {
        output_rows,
        iterator_ops,
        hash_build_rows,
        hash_probe_rows,
        materialized_values,
        memory_bytes,
    }
}

fn estimate_output_rows(query: &NormalizedQuery, variable_costs: &[VariableCost]) -> u64 {
    let has_aggregate = has_aggregate(query);
    let group_vars = query
        .find
        .iter()
        .filter(|term| matches!(term, NormFindTerm::Variable { .. }))
        .count() as u64;
    if has_aggregate && group_vars == 0 {
        return 1;
    }
    variable_costs
        .iter()
        .map(|cost| cost.estimated_candidates)
        .min()
        .unwrap_or(1)
        .max(1)
}

fn estimate_materialized_values(query: &NormalizedQuery, output_rows: u64) -> u64 {
    let projected_values = query.find.len() as u64;
    output_rows
        .saturating_mul(projected_values)
        .max(projected_values)
}

fn probe_node_impls(
    schema: &StorageSchema,
    atoms: &[&NormAtom],
    variable_order_ids: &[usize],
    stats: &PlannerStats,
    cyclic: bool,
) -> Result<Vec<NodeImpl>> {
    let mut bound = BTreeSet::new();
    let mut out = Vec::new();
    for variable in variable_order_ids {
        let implementation =
            if !cyclic && variable_probe_eligible(schema, atoms, stats, &bound, *variable)? {
                NodeImpl::HashProbe
            } else {
                NodeImpl::SortedLeapfrog
            };
        out.push(implementation);
        bound.insert(*variable);
    }
    Ok(out)
}

fn hybrid_node_impls(
    schema: &StorageSchema,
    atoms: &[&NormAtom],
    variable_order_ids: &[usize],
    stats: &PlannerStats,
    cyclic: bool,
) -> Result<Vec<NodeImpl>> {
    let mut bound = BTreeSet::new();
    let mut out = Vec::new();
    for variable in variable_order_ids {
        let implementation =
            if !cyclic && variable_probe_eligible(schema, atoms, stats, &bound, *variable)? {
                NodeImpl::Hybrid
            } else {
                NodeImpl::SortedLeapfrog
            };
        out.push(implementation);
        bound.insert(*variable);
    }
    Ok(out)
}

fn variable_probe_eligible(
    schema: &StorageSchema,
    atoms: &[&NormAtom],
    stats: &PlannerStats,
    bound: &BTreeSet<usize>,
    variable: usize,
) -> Result<bool> {
    for atom in atoms
        .iter()
        .copied()
        .filter(|atom| atom_contains_variable(atom, variable))
    {
        let estimate = estimate_atom_variable_access(schema, stats, bound, atom, variable)?;
        let relation_rows = stats.relation_rows(&atom.relation_name);
        if estimate.prefix_len > 0 && estimate.estimated_rows <= relation_rows.max(1).div_ceil(2) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn is_cyclic_multiway_query(query: &NormalizedQuery, atoms: &[&NormAtom]) -> bool {
    if atoms.len() < 3 {
        return false;
    }
    let mut degree = vec![0usize; query.vars.len()];
    for atom in atoms {
        for variable in atom_variables(atom) {
            degree[variable] += 1;
        }
    }
    degree
        .into_iter()
        .filter(|count| *count > 0)
        .all(|count| count >= 2)
}

fn atom_variables(atom: &NormAtom) -> BTreeSet<usize> {
    atom.fields
        .iter()
        .filter_map(|field| match field.term {
            NormTerm::Var(variable) => Some(variable.0 as usize),
            NormTerm::Input(_) | NormTerm::Literal(_) | NormTerm::Wildcard => None,
        })
        .collect()
}

fn payload_demand(query: &NormalizedQuery) -> PayloadDemand {
    let mut projected_vars = Vec::new();
    let mut aggregate_vars = Vec::new();
    for term in &query.find {
        match term {
            NormFindTerm::Variable { variable } => projected_vars.push(*variable),
            NormFindTerm::Aggregate { variable, .. } => {
                aggregate_vars.push(*variable);
            }
        }
    }
    PayloadDemand {
        projected_vars,
        aggregate_vars,
        existence_only_relations: Vec::new(),
        row_id_demands: Vec::new(),
    }
}

fn output_plan(query: &NormalizedQuery) -> OutputPlan {
    output_plan_from_find(&query.find)
}

fn output_plan_from_find(find: &[NormFindTerm]) -> OutputPlan {
    if find
        .iter()
        .any(|term| matches!(term, NormFindTerm::Aggregate { .. }))
    {
        let mut group_vars = Vec::new();
        let mut aggregates = Vec::new();
        for term in find {
            match term {
                NormFindTerm::Variable { variable } => group_vars.push(*variable),
                NormFindTerm::Aggregate {
                    function,
                    variable,
                    value_type,
                } => aggregates.push(AggregateTerm {
                    function: *function,
                    var: *variable,
                    value_type: value_type.clone(),
                }),
            }
        }
        OutputPlan::Aggregate(AggregatePlan {
            group_vars,
            aggregates,
        })
    } else {
        OutputPlan::Project(ProjectPlan {
            vars: find
                .iter()
                .filter_map(|term| match term {
                    NormFindTerm::Variable { variable } => Some(*variable),
                    NormFindTerm::Aggregate { .. } => None,
                })
                .collect(),
            set_semantics: true,
        })
    }
}

fn atom_contains_variable(atom: &NormAtom, variable: usize) -> bool {
    atom.fields
        .iter()
        .any(|field| matches!(field.term, NormTerm::Var(id) if id.0 as usize == variable))
}

fn comparisons_ready_pass(
    txn: &ReadTxn<'_>,
    comparisons: &[NormPredicate],
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    binding: &EncodedBinding,
    counters: &mut PlanCounters,
) -> Result<bool> {
    for comparison in comparisons {
        let Some(left_encoded) = operand_encoded_value(
            &comparison.operands[0],
            &comparison.value_type,
            inputs,
            binding,
        ) else {
            continue;
        };
        let Some(right_encoded) = operand_encoded_value(
            &comparison.operands[1],
            &comparison.value_type,
            inputs,
            binding,
        ) else {
            continue;
        };
        if encoded_comparison_supported(comparison.op, &comparison.value_type) {
            counters.comparisons_evaluated += 1;
            counters.encoded_comparisons_evaluated += 1;
            if !compare_encoded_values(
                left_encoded.as_bytes(),
                comparison.op,
                right_encoded.as_bytes(),
            ) {
                counters.comparisons_failed += 1;
                return Ok(false);
            }
            continue;
        }

        let Some(left) = operand_logical_value(
            txn,
            &comparison.operands[0],
            &comparison.value_type,
            query,
            inputs,
            binding,
            counters,
        )?
        else {
            continue;
        };
        let Some(right) = operand_logical_value(
            txn,
            &comparison.operands[1],
            &comparison.value_type,
            query,
            inputs,
            binding,
            counters,
        )?
        else {
            continue;
        };
        counters.comparisons_evaluated += 1;
        counters.decoded_comparisons_evaluated += 1;
        let left = normalize_value_for_type(&left, &comparison.value_type);
        let right = normalize_value_for_type(&right, &comparison.value_type);
        if !compare_values(&left, comparison.op, &right) {
            counters.comparisons_failed += 1;
            return Ok(false);
        }
    }
    Ok(true)
}

fn operand_encoded_value(
    operand: &NormOperand,
    value_type: &ValueType,
    inputs: &EncodedInputs,
    binding: &EncodedBinding,
) -> Option<EncodedValue> {
    match operand {
        NormOperand::Var(variable) => binding.get(variable.0 as usize).map(|value| EncodedValue {
            value_type: value_type.clone(),
            encoded: value.encoded.clone(),
        }),
        NormOperand::Input(input) => inputs
            .get(*input)
            .map(|value| EncodedValue::from_owned(value_type.clone(), value)),
        NormOperand::Literal(literal) => {
            Some(EncodedValue::from_owned(value_type.clone(), literal))
        }
    }
}

fn encoded_comparison_supported(operator: ComparisonOperator, value_type: &ValueType) -> bool {
    match operator {
        ComparisonOperator::Eq | ComparisonOperator::NotEq => true,
        ComparisonOperator::Lt
        | ComparisonOperator::Lte
        | ComparisonOperator::Gt
        | ComparisonOperator::Gte => !matches!(value_type, ValueType::String | ValueType::Bytes),
    }
}

fn compare_encoded_values(left: &[u8], operator: ComparisonOperator, right: &[u8]) -> bool {
    match operator {
        ComparisonOperator::Eq => left == right,
        ComparisonOperator::NotEq => left != right,
        ComparisonOperator::Lt => left < right,
        ComparisonOperator::Lte => left <= right,
        ComparisonOperator::Gt => left > right,
        ComparisonOperator::Gte => left >= right,
    }
}

fn compare_values(left: &Value, operator: ComparisonOperator, right: &Value) -> bool {
    match operator {
        ComparisonOperator::Eq => left == right,
        ComparisonOperator::NotEq => left != right,
        ComparisonOperator::Lt => left < right,
        ComparisonOperator::Lte => left <= right,
        ComparisonOperator::Gt => left > right,
        ComparisonOperator::Gte => left >= right,
    }
}

fn operand_logical_value(
    txn: &ReadTxn<'_>,
    operand: &NormOperand,
    value_type: &ValueType,
    _query: &NormalizedQuery,
    inputs: &EncodedInputs,
    binding: &EncodedBinding,
    counters: &mut PlanCounters,
) -> Result<Option<Value>> {
    Ok(match operand {
        NormOperand::Var(variable) => binding
            .get(variable.0 as usize)
            .map(|value| {
                record_decode(value_type, counters);
                txn.decode_query_value(value_type, value.as_bytes())
            })
            .transpose()?,
        NormOperand::Input(input) => inputs
            .get(*input)
            .map(|value| {
                record_decode(value_type, counters);
                txn.decode_query_value(value_type, value.as_bytes())
            })
            .transpose()?,
        NormOperand::Literal(literal) => {
            record_decode(value_type, counters);
            Some(txn.decode_query_value(value_type, literal.as_bytes())?)
        }
    })
}

fn record_decode(value_type: &ValueType, counters: &mut PlanCounters) {
    counters.decoded_values += 1;
    if matches!(value_type, ValueType::String | ValueType::Bytes) {
        counters.dictionary_reverse_lookups += 1;
    }
}

fn input_value<'a>(
    query: &'a TypedQuery,
    inputs: &'a InputBindings,
    input: usize,
) -> Result<&'a Value> {
    let input = &query.inputs[input];
    let value = inputs
        .get(&input.name)
        .ok_or_else(|| Error::missing_input(&input.name))?;
    if !value_matches_type(value, &input.value_type) {
        return Err(Error::query_input_type_mismatch(
            &input.name,
            value_type_name(&input.value_type),
            value.kind_name(),
        ));
    }
    Ok(value)
}

fn validate_inputs(query: &TypedQuery, inputs: &InputBindings) -> Result<()> {
    for input in &query.inputs {
        input_value(query, inputs, input.id)?;
    }
    Ok(())
}

fn value_matches_type(value: &Value, value_type: &ValueType) -> bool {
    matches!(
        (value, value_type),
        (Value::Bool(_), ValueType::Bool)
            | (Value::U64(_), ValueType::U64)
            | (Value::I64(_), ValueType::I64)
            | (Value::Id(_), ValueType::Id { .. })
            | (Value::Ref(_), ValueType::Ref { .. })
            | (Value::Timestamp(_), ValueType::TimestampMicros)
            | (Value::Decimal(_), ValueType::Decimal { .. })
            | (Value::Uuid(_), ValueType::Uuid)
            | (Value::Symbol(_), ValueType::Symbol { .. })
            | (Value::String(_), ValueType::String)
            | (Value::Bytes(_), ValueType::Bytes)
    )
}

fn normalize_value_for_type(value: &Value, value_type: &ValueType) -> Value {
    match (value, value_type) {
        (Value::Ref(raw), ValueType::Id { .. }) => Value::Id(*raw),
        (Value::Id(raw), ValueType::Ref { .. }) => Value::Ref(*raw),
        _ => value.clone(),
    }
}

fn literal_to_value(literal: &TypedLiteral) -> Result<Value> {
    let value = match (&literal.literal, &literal.value_type) {
        (Literal::Bool(value), ValueType::Bool) => Value::Bool(*value),
        (Literal::String(value), ValueType::String) => Value::String(value.clone()),
        (Literal::Integer(value), ValueType::U64) => Value::U64(*value as u64),
        (Literal::Integer(value), ValueType::I64) => Value::I64(*value as i64),
        (Literal::Integer(value), ValueType::Id { .. }) => Value::Id(*value as u64),
        (Literal::Integer(value), ValueType::Ref { .. }) => Value::Ref(*value as u64),
        (Literal::Integer(value), ValueType::Symbol { .. }) => Value::Symbol(*value as u64),
        (Literal::Integer(value), ValueType::TimestampMicros) => {
            Value::Timestamp(TimestampMicros(*value as i64))
        }
        (Literal::Integer(value), ValueType::Decimal { .. }) => Value::Decimal(DecimalRaw(*value)),
        _ => {
            return Err(Error::internal(
                "typed literal does not match literal value",
            ));
        }
    };
    Ok(value)
}

fn normalize_query(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &TypedQuery,
) -> Result<NormalizedQuery> {
    let vars = query
        .variables
        .iter()
        .map(|variable| NormVar {
            id: VarId(variable.id as u16),
            name: variable.name.clone(),
            value_type: variable.value_type.clone(),
        })
        .collect::<Vec<_>>();
    let inputs = query
        .inputs
        .iter()
        .map(|input| NormInput {
            id: InputId(input.id as u16),
            name: input.name.clone(),
            value_type: input.value_type.clone(),
        })
        .collect::<Vec<_>>();
    let mut atoms = Vec::new();
    let mut predicates = Vec::new();
    for clause in &query.clauses {
        match clause {
            TypedClause::Relation(atom) => atoms.push(normalize_atom(txn, atom, atoms.len())?),
            TypedClause::Comparison(comparison) => {
                predicates.push(normalize_predicate(txn, comparison, predicates.len())?)
            }
        }
    }
    let find = query
        .find
        .iter()
        .map(|term| match term {
            TypedFindTerm::Variable { variable } => NormFindTerm::Variable {
                variable: VarId(*variable as u16),
            },
            TypedFindTerm::Aggregate {
                function,
                variable,
                value_type,
            } => NormFindTerm::Aggregate {
                function: *function,
                variable: VarId(*variable as u16),
                value_type: value_type.clone(),
            },
        })
        .collect::<Vec<_>>();
    let output = output_plan_from_find(&find);
    let normalized = NormalizedQuery {
        vars,
        inputs,
        atoms,
        predicates,
        output,
        find,
    };
    validate_normalized_query(schema, &normalized)?;
    Ok(normalized)
}

fn normalize_atom(txn: &ReadTxn<'_>, atom: &TypedRelationAtom, atom_id: usize) -> Result<NormAtom> {
    let fields = atom
        .fields
        .iter()
        .map(|field| {
            Ok(NormAtomField {
                field: FieldId(field.field_id as u16),
                field_name: field.field.clone(),
                term: normalize_term(txn, &field.term)?,
                value_type: field.value_type.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(NormAtom {
        id: AtomId(atom_id as u16),
        relation: crate::RelationId(atom.relation_id as u16),
        relation_name: atom.relation.clone(),
        fields,
    })
}

fn normalize_term(txn: &ReadTxn<'_>, term: &TypedTerm) -> Result<NormTerm> {
    Ok(match term {
        TypedTerm::Variable(variable) => NormTerm::Var(VarId(*variable as u16)),
        TypedTerm::Input(input) => NormTerm::Input(InputId(*input as u16)),
        TypedTerm::Literal(literal) => NormTerm::Literal(encode_literal(txn, literal)?),
        TypedTerm::Wildcard => NormTerm::Wildcard,
    })
}

fn normalize_predicate(
    txn: &ReadTxn<'_>,
    comparison: &TypedComparison,
    predicate_id: usize,
) -> Result<NormPredicate> {
    Ok(NormPredicate {
        id: PredicateId(predicate_id as u16),
        operands: [
            normalize_operand(txn, &comparison.left, &comparison.value_type)?,
            normalize_operand(txn, &comparison.right, &comparison.value_type)?,
        ],
        op: comparison.operator,
        value_type: comparison.value_type.clone(),
        earliest_depth: None,
    })
}

fn normalize_operand(
    txn: &ReadTxn<'_>,
    operand: &TypedOperand,
    value_type: &ValueType,
) -> Result<NormOperand> {
    Ok(match operand {
        TypedOperand::Variable(variable) => NormOperand::Var(VarId(*variable as u16)),
        TypedOperand::Input(input) => NormOperand::Input(InputId(*input as u16)),
        TypedOperand::Literal(literal) => {
            let value = literal_to_value(literal)?;
            let normalized = normalize_value_for_type(&value, value_type);
            NormOperand::Literal(encode_owned_value(txn, value_type, &normalized)?)
        }
    })
}

fn encode_literal(txn: &ReadTxn<'_>, literal: &TypedLiteral) -> Result<EncodedOwned> {
    let value = literal_to_value(literal)?;
    let normalized = normalize_value_for_type(&value, &literal.value_type);
    encode_owned_value(txn, &literal.value_type, &normalized)
}

fn encode_owned_value(
    txn: &ReadTxn<'_>,
    value_type: &ValueType,
    value: &Value,
) -> Result<EncodedOwned> {
    let bytes = txn.encode_query_value(value_type, value)?;
    encoded_owned_from_bytes(value_type, bytes)
}

fn encoded_owned_from_bytes(value_type: &ValueType, bytes: Vec<u8>) -> Result<EncodedOwned> {
    match value_type.encoded_width() {
        1 => Ok(EncodedOwned::One(exact_encoded_array::<1>(&bytes)?)),
        8 => Ok(EncodedOwned::Eight(exact_encoded_array::<8>(&bytes)?)),
        16 => Ok(EncodedOwned::Sixteen(exact_encoded_array::<16>(&bytes)?)),
        width => Err(Error::internal(format!(
            "unsupported normalized encoded width {width}"
        ))),
    }
}

fn exact_encoded_array<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| Error::internal("normalized encoded value width mismatch"))
}

fn encode_inputs(
    txn: &ReadTxn<'_>,
    query: &NormalizedQuery,
    inputs: &InputBindings,
) -> Result<EncodedInputs> {
    let values = query
        .inputs
        .iter()
        .map(|input| {
            let value = inputs
                .get(&input.name)
                .ok_or_else(|| Error::missing_input(&input.name))?;
            if !value_matches_type(value, &input.value_type) {
                return Err(Error::query_input_type_mismatch(
                    &input.name,
                    value_type_name(&input.value_type),
                    value.kind_name(),
                ));
            }
            let normalized = normalize_value_for_type(value, &input.value_type);
            encode_owned_value(txn, &input.value_type, &normalized)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(EncodedInputs { values })
}

fn validate_normalized_query(schema: &StorageSchema, query: &NormalizedQuery) -> Result<()> {
    for atom in &query.atoms {
        let (_, relation) = schema.relation(&atom.relation_name)?;
        if atom.relation.0 as usize >= schema.descriptor().relations.len() {
            return Err(Error::unknown_relation(&atom.relation_name));
        }
        for field in &atom.fields {
            let descriptor = relation
                .fields
                .get(field.field.0 as usize)
                .ok_or_else(|| Error::unknown_field(&atom.relation_name, &field.field_name))?;
            if descriptor.name != field.field_name {
                return Err(Error::unknown_field(&atom.relation_name, &field.field_name));
            }
        }
    }
    Ok(())
}

fn attach_predicate_depths(query: &mut NormalizedQuery, variable_order_ids: &[usize]) {
    let mut depth_by_var = BTreeMap::new();
    for (depth, variable) in variable_order_ids.iter().enumerate() {
        depth_by_var.insert(VarId(*variable as u16), depth);
    }
    for predicate in &mut query.predicates {
        predicate.earliest_depth = predicate
            .operands
            .iter()
            .filter_map(|operand| match operand {
                NormOperand::Var(variable) => depth_by_var.get(variable).copied(),
                NormOperand::Input(_) | NormOperand::Literal(_) => Some(0),
            })
            .max();
    }
}

fn has_aggregate(query: &NormalizedQuery) -> bool {
    query
        .find
        .iter()
        .any(|term| matches!(term, NormFindTerm::Aggregate { .. }))
}

fn result_columns(query: &NormalizedQuery) -> Vec<ResultColumn> {
    query
        .find
        .iter()
        .map(|term| match term {
            NormFindTerm::Variable { variable } => {
                ResultColumn::Variable(query.vars[variable.0 as usize].name.clone())
            }
            NormFindTerm::Aggregate {
                function, variable, ..
            } => ResultColumn::Aggregate {
                function: *function,
                variable: query.vars[variable.0 as usize].name.clone(),
            },
        })
        .collect()
}

trait TupleSink {
    fn emit(
        &mut self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<()>;

    fn finish(
        self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>>
    where
        Self: Sized;
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "compiled plan trait is reserved for specialization work"
    )
)]
trait ExecutablePlan {
    fn execute(
        &mut self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        image: &crate::QueryImage,
        inputs: &EncodedInputs,
        sink: &mut dyn TupleSink,
    ) -> Result<PlanCounters>;
}

#[expect(
    dead_code,
    reason = "compiled plan scaffold is reserved for specialization work"
)]
#[derive(Clone, Debug)]
struct InterpretedFreeJoinPlan {
    query: NormalizedQuery,
    plan: FreeJoinPlan,
}

#[expect(
    dead_code,
    reason = "compiled plan enum is reserved for specialization work"
)]
enum CompiledPlan {
    Interpreted(Box<InterpretedFreeJoinPlan>),
    Specialized(Box<dyn ExecutablePlan + Send + Sync>),
}

#[derive(Clone, Debug)]
enum OutputSink {
    Project(EncodedProjectSink),
    Aggregate(AggregateSink),
}

impl OutputSink {
    fn new(output: &OutputPlan) -> Self {
        match output {
            OutputPlan::Project(plan) => OutputSink::Project(EncodedProjectSink::new(plan)),
            OutputPlan::Aggregate(plan) => OutputSink::Aggregate(AggregateSink::new(plan)),
        }
    }
}

impl TupleSink for OutputSink {
    fn emit(
        &mut self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<()> {
        match self {
            OutputSink::Project(sink) => sink.emit(txn, query, binding, counters),
            OutputSink::Aggregate(sink) => sink.emit(txn, query, binding, counters),
        }
    }

    fn finish(
        self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>> {
        match self {
            OutputSink::Project(sink) => sink.finish(txn, query, counters),
            OutputSink::Aggregate(sink) => sink.finish(txn, query, counters),
        }
    }
}

#[derive(Clone, Debug)]
struct EncodedProjectSink {
    vars: Vec<VarId>,
    rows: BTreeSet<SmallEncodedRow>,
}

impl EncodedProjectSink {
    fn new(plan: &ProjectPlan) -> Self {
        Self {
            vars: plan.vars.clone(),
            rows: BTreeSet::new(),
        }
    }
}

impl TupleSink for EncodedProjectSink {
    fn emit(
        &mut self,
        _txn: &ReadTxn<'_>,
        _query: &NormalizedQuery,
        binding: &EncodedBinding,
        _counters: &mut PlanCounters,
    ) -> Result<()> {
        let row = self
            .vars
            .iter()
            .map(|variable| bound_encoded_variable(binding, variable.0 as usize).cloned())
            .collect::<Result<SmallEncodedRow>>()?;
        self.rows.insert(row);
        Ok(())
    }

    fn finish(
        self,
        txn: &ReadTxn<'_>,
        _query: &NormalizedQuery,
        counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>> {
        let _span =
            tracing::debug_span!("bumbledb.query.project", rows = self.rows.len()).entered();
        self.rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|value| decode_output_value(txn, value, counters))
                    .collect::<Result<Vec<_>>>()
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct AggregateSink {
    group_vars: Vec<VarId>,
    terms: Vec<AggregateTerm>,
    groups: BTreeMap<SmallEncodedRow, Vec<AggregateState>>,
}

impl AggregateSink {
    fn new(plan: &AggregatePlan) -> Self {
        Self {
            group_vars: plan.group_vars.clone(),
            terms: plan.aggregates.clone(),
            groups: BTreeMap::new(),
        }
    }

    fn group_key(&self, binding: &EncodedBinding) -> Result<SmallEncodedRow> {
        self.group_vars
            .iter()
            .map(|variable| bound_encoded_variable(binding, variable.0 as usize).cloned())
            .collect()
    }

    fn count_only(&self) -> bool {
        self.terms
            .iter()
            .all(|term| term.function == AggregateFunction::Count)
    }

    fn emit_count_range(&mut self, binding: &EncodedBinding, count: u64) -> Result<()> {
        let key = self.group_key(binding)?;
        let states = ensure_aggregate_group(&mut self.groups, &self.terms, key);
        for state in states {
            state.apply_count_by(count)?;
        }
        Ok(())
    }
}

impl TupleSink for AggregateSink {
    fn emit(
        &mut self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<()> {
        if self.count_only() {
            return self.emit_count_range(binding, 1);
        }

        let key = self.group_key(binding)?;
        let states = ensure_aggregate_group(&mut self.groups, &self.terms, key);
        for (state, term) in states.iter_mut().zip(&self.terms) {
            state.apply_encoded(txn, query, binding, term, counters)?;
        }
        Ok(())
    }

    fn finish(
        self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>> {
        let _span =
            tracing::debug_span!("bumbledb.query.aggregate", groups = self.groups.len()).entered();
        let mut rows = Vec::new();
        for (key, states) in self.groups {
            let mut row = Vec::new();
            let mut key_iter = key.into_iter();
            let mut state_iter = states.into_iter();
            for term in &query.find {
                match term {
                    NormFindTerm::Variable { .. } => {
                        let value = key_iter
                            .next()
                            .ok_or_else(|| Error::internal("aggregate group key is missing"))?;
                        row.push(decode_output_value(txn, value, counters)?);
                    }
                    NormFindTerm::Aggregate { .. } => {
                        counters.materialized_output_values += 1;
                        let state = state_iter
                            .next()
                            .ok_or_else(|| Error::internal("aggregate state is missing"))?;
                        row.push(state.finish_encoded(txn, counters)?);
                    }
                }
            }
            rows.push(row);
        }
        rows.sort();
        Ok(rows)
    }
}

fn initial_aggregate_states(terms: &[AggregateTerm]) -> Vec<AggregateState> {
    terms
        .iter()
        .map(|term| AggregateState::new_encoded(term.function, term.value_type.clone()))
        .collect()
}

fn ensure_aggregate_group<'a>(
    groups: &'a mut BTreeMap<SmallEncodedRow, Vec<AggregateState>>,
    terms: &[AggregateTerm],
    key: SmallEncodedRow,
) -> &'a mut Vec<AggregateState> {
    match groups.entry(key) {
        std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(initial_aggregate_states(terms))
        }
    }
}

fn bound_encoded_variable(binding: &EncodedBinding, variable: usize) -> Result<&EncodedValue> {
    binding
        .get(variable)
        .ok_or_else(|| Error::internal(format!("variable {variable} is unbound at projection")))
}

fn decode_bound_variable(
    txn: &ReadTxn<'_>,
    query: &NormalizedQuery,
    binding: &EncodedBinding,
    variable: usize,
    counters: &mut PlanCounters,
) -> Result<Value> {
    let value = bound_encoded_variable(binding, variable)?;
    record_decode(&query.vars[variable].value_type, counters);
    txn.decode_query_value(&query.vars[variable].value_type, value.as_bytes())
}

fn decode_output_value(
    txn: &ReadTxn<'_>,
    value: EncodedValue,
    counters: &mut PlanCounters,
) -> Result<Value> {
    counters.materialized_output_values += 1;
    record_decode(&value.value_type, counters);
    txn.decode_query_value(&value.value_type, value.as_bytes())
}

#[derive(Clone, Debug)]
enum AggregateState {
    Count(u64),
    SumU64(u64),
    SumI64(i64),
    SumDecimal(i128),
    EncodedMin(Option<EncodedValue>),
    EncodedMax(Option<EncodedValue>),
    Min(Option<Value>),
    Max(Option<Value>),
}

impl AggregateState {
    fn new(function: AggregateFunction, value_type: ValueType) -> Self {
        match (function, value_type) {
            (AggregateFunction::Count, _) => AggregateState::Count(0),
            (AggregateFunction::Sum, ValueType::U64) => AggregateState::SumU64(0),
            (AggregateFunction::Sum, ValueType::I64) => AggregateState::SumI64(0),
            (AggregateFunction::Sum, ValueType::Decimal { .. }) => AggregateState::SumDecimal(0),
            (AggregateFunction::Min, _) => AggregateState::Min(None),
            (AggregateFunction::Max, _) => AggregateState::Max(None),
            _ => AggregateState::Count(0),
        }
    }

    fn new_encoded(function: AggregateFunction, value_type: ValueType) -> Self {
        match function {
            AggregateFunction::Min if encoded_minmax_supported(&value_type) => {
                AggregateState::EncodedMin(None)
            }
            AggregateFunction::Max if encoded_minmax_supported(&value_type) => {
                AggregateState::EncodedMax(None)
            }
            _ => AggregateState::new(function, value_type),
        }
    }

    fn apply_count(&mut self) -> Result<()> {
        self.apply_count_by(1)
    }

    fn apply_count_by(&mut self, value: u64) -> Result<()> {
        let AggregateState::Count(count) = self else {
            return Err(Error::internal("count aggregate state mismatch"));
        };
        *count = count
            .checked_add(value)
            .ok_or_else(|| Error::integer_overflow("count"))?;
        Ok(())
    }

    fn apply_encoded(
        &mut self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        term: &AggregateTerm,
        counters: &mut PlanCounters,
    ) -> Result<()> {
        match self {
            AggregateState::Count(_) => self.apply_count(),
            AggregateState::EncodedMin(current) => {
                let value = bound_encoded_variable(binding, term.var.0 as usize)?.clone();
                if current.as_ref().is_none_or(|existing| &value < existing) {
                    *current = Some(value);
                }
                Ok(())
            }
            AggregateState::EncodedMax(current) => {
                let value = bound_encoded_variable(binding, term.var.0 as usize)?.clone();
                if current.as_ref().is_none_or(|existing| &value > existing) {
                    *current = Some(value);
                }
                Ok(())
            }
            _ => {
                let value =
                    decode_bound_variable(txn, query, binding, term.var.0 as usize, counters)?;
                self.apply(&value)
            }
        }
    }

    fn apply(&mut self, value: &Value) -> Result<()> {
        match self {
            AggregateState::Count(_) => self.apply_count()?,
            AggregateState::SumU64(sum) => {
                let Value::U64(value) = value else {
                    return Err(Error::aggregate_type_mismatch("sum", value.kind_name()));
                };
                *sum = sum
                    .checked_add(*value)
                    .ok_or_else(|| Error::integer_overflow("sum"))?;
            }
            AggregateState::SumI64(sum) => {
                let Value::I64(value) = value else {
                    return Err(Error::aggregate_type_mismatch("sum", value.kind_name()));
                };
                *sum = sum
                    .checked_add(*value)
                    .ok_or_else(|| Error::integer_overflow("sum"))?;
            }
            AggregateState::SumDecimal(sum) => {
                let Value::Decimal(DecimalRaw(value)) = value else {
                    return Err(Error::aggregate_type_mismatch("sum", value.kind_name()));
                };
                *sum = sum
                    .checked_add(*value)
                    .ok_or_else(|| Error::decimal_overflow("sum"))?;
            }
            AggregateState::EncodedMin(_) | AggregateState::EncodedMax(_) => {
                return Err(Error::internal(
                    "encoded aggregate state cannot apply logical value",
                ));
            }
            AggregateState::Min(current) => match current {
                Some(existing) if &*existing <= value => {}
                _ => *current = Some(value.clone()),
            },
            AggregateState::Max(current) => match current {
                Some(existing) if &*existing >= value => {}
                _ => *current = Some(value.clone()),
            },
        }
        Ok(())
    }

    fn finish(self) -> Result<Value> {
        Ok(match self {
            AggregateState::Count(count) => Value::U64(count),
            AggregateState::SumU64(sum) => Value::U64(sum),
            AggregateState::SumI64(sum) => Value::I64(sum),
            AggregateState::SumDecimal(sum) => Value::Decimal(DecimalRaw(sum)),
            AggregateState::EncodedMin(_) | AggregateState::EncodedMax(_) => {
                return Err(Error::internal(
                    "encoded aggregate state requires output decoder",
                ));
            }
            AggregateState::Min(Some(value)) | AggregateState::Max(Some(value)) => value,
            AggregateState::Min(None) | AggregateState::Max(None) => Value::U64(0),
        })
    }

    fn finish_encoded(self, txn: &ReadTxn<'_>, counters: &mut PlanCounters) -> Result<Value> {
        Ok(match self {
            AggregateState::EncodedMin(Some(value)) | AggregateState::EncodedMax(Some(value)) => {
                record_decode(&value.value_type, counters);
                txn.decode_query_value(&value.value_type, value.as_bytes())?
            }
            AggregateState::EncodedMin(None) | AggregateState::EncodedMax(None) => Value::U64(0),
            state => state.finish()?,
        })
    }
}

fn encoded_minmax_supported(value_type: &ValueType) -> bool {
    !matches!(value_type, ValueType::String | ValueType::Bytes)
}

fn value_type_name(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Bool => "bool".to_owned(),
        ValueType::U64 => "u64".to_owned(),
        ValueType::I64 => "i64".to_owned(),
        ValueType::Id { name, .. } => name.clone(),
        ValueType::Ref { name, .. } => name.clone(),
        ValueType::TimestampMicros => "timestamp".to_owned(),
        ValueType::Decimal { scale } => format!("decimal(scale={scale})"),
        ValueType::Uuid => "uuid".to_owned(),
        ValueType::Symbol { name } => name.clone(),
        ValueType::String => "string".to_owned(),
        ValueType::Bytes => "bytes".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_image::QueryImageBuilder;
    use crate::{AggregateError, Environment, ExecuteError, QueryError, Row};
    use bumbledb_core::datalog::parse_and_typecheck;
    use bumbledb_core::schema::{
        FieldDescriptor, IndexDescriptor, PrimaryKeyDescriptor, RelationDescriptor, RelationKind,
    };

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn query_observability_defaults_are_zero() {
        let timings = QueryTimings::default();
        assert_eq!(timings.total_micros, 0);
        assert_eq!(timings.execute_micros, 0);
        assert_eq!(QueryRuntimeKind::default(), QueryRuntimeKind::Unknown);

        let allocations = QueryAllocationStats::default();
        assert!(!allocations.enabled);
        assert_eq!(allocations.alloc_calls, 0);
        assert_eq!(allocations.net_bytes, 0);
    }

    #[test]
    fn executes_single_relation_query() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?account where Account(id: ?account, holder: $holder)",
        )?;

        let output = env.read(|txn| {
            txn.execute_query(
                &schema,
                &query,
                &InputBindings::from_values([("holder", Value::Ref(1))]),
            )
        })?;

        assert_eq!(output.rows, vec![vec![Value::Id(1)], vec![Value::Id(2)]]);
        assert!(
            output
                .plan
                .variable_estimates
                .iter()
                .any(|estimate| estimate.access == "Account.by_holder")
        );
        assert_ne!(output.plan.runtime_kind, QueryRuntimeKind::Unknown);
        assert!(output.plan.timings.total_micros > 0);
        assert!(output.plan.timings.execute_micros <= output.plan.timings.total_micros);
        assert!(!output.plan.allocations.enabled);
        assert!(!output.plan.node_timings.is_empty());
        Ok(())
    }

    #[test]
    fn planner_recommends_missing_static_predicate_index() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?account where Account(id: ?account, currency: $currency)",
        )?;

        let output = env.read(|txn| {
            txn.execute_query(
                &schema,
                &query,
                &InputBindings::from_values([("currency", Value::Symbol(840))]),
            )
        })?;

        assert_same_rows(&output.rows, &[vec![Value::Id(1)], vec![Value::Id(3)]]);
        let expected_fields = vec!["currency".to_owned(), "id".to_owned()];
        assert!(output.plan.missing_indexes.iter().any(|missing| {
            missing.relation == "Account"
                && missing.fields == expected_fields
                && missing.reason.contains("StaticPredicate")
        }));
        Ok(())
    }

    #[test]
    fn optimizer_selects_equality_index_and_hash_probe_for_static_lookup() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = StorageSchema::new(optimizer_schema(), env.max_key_size())?;
        env.write(|txn| {
            txn.insert(&schema, item_row(1, 1))?;
            txn.insert(&schema, item_row(2, 1))?;
            txn.insert(&schema, item_row(3, 2))?;
            Ok::<(), Error>(())
        })?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?item where Item(id: ?item, kind: $kind)",
        )?;

        let output = env.read(|txn| {
            txn.execute_query(
                &schema,
                &query,
                &InputBindings::from_values([("kind", Value::Symbol(1))]),
            )
        })?;

        assert!(
            output
                .plan
                .variable_estimates
                .iter()
                .any(|estimate| estimate.access == "Item.by_kind")
        );
        assert!(
            output
                .plan
                .free_join
                .nodes
                .iter()
                .any(|node| node.implementation == NodeImpl::HashProbe)
        );
        assert_eq!(output.plan.optimizer.chosen, "hash_probe");
        assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::HashProbe);
        assert!(output.plan.counters.hash_probe_calls > 0);
        assert_eq!(output.plan.counters.trie_open, 0);
        assert_eq!(output.plan.counters.trie_next, 0);
        assert_eq!(output.plan.counters.trie_seek, 0);
        assert_eq!(output.plan.counters.trie_key_reads, 0);
        assert_same_rows(&output.rows, &[vec![Value::Id(1)], vec![Value::Id(2)]]);
        Ok(())
    }

    #[test]
    fn hash_probe_runtime_checks_static_existence_atoms() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = StorageSchema::new(chain_schema(), env.max_key_size())?;
        env.write(|txn| {
            txn.insert(&schema, b_row(1, 99))?;
            Ok::<(), Error>(())
        })?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?b
            where
              A(id: $a)
              B(id: ?b, a: $a)
            "#,
        )?;

        let output = env.read(|txn| {
            txn.execute_query(
                &schema,
                &query,
                &InputBindings::from_values([("a", Value::U64(99))]),
            )
        })?;

        assert!(output.rows.is_empty());
        assert_eq!(output.plan.counters.trie_open, 0);
        assert!(output.plan.counters.hash_probe_calls > 0);
        Ok(())
    }

    #[test]
    fn optimizer_keeps_cyclic_triangle_on_lftj() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = StorageSchema::new(triangle_schema(), env.max_key_size())?;
        env.write(|txn| {
            txn.insert(&schema, edge_ab_row(1, 10))?;
            txn.insert(&schema, edge_ac_row(1, 20))?;
            txn.insert(&schema, edge_bc_row(10, 20))?;
            txn.insert(&schema, edge_ab_row(2, 10))?;
            txn.insert(&schema, edge_ac_row(2, 30))?;
            txn.insert(&schema, edge_bc_row(10, 40))?;
            Ok::<(), Error>(())
        })?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find count(?a)
            where
              EdgeAB(a: ?a, b: ?b)
              EdgeAC(a: ?a, c: ?c)
              EdgeBC(b: ?b, c: ?c)
            "#,
        )?;

        let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

        assert_eq!(output.rows, vec![vec![Value::U64(1)]]);
        assert_eq!(output.plan.runtime_kind, QueryRuntimeKind::Lftj);
        assert!(
            output
                .plan
                .free_join
                .nodes
                .iter()
                .all(|node| node.implementation == NodeImpl::SortedLeapfrog)
        );
        assert!(
            output
                .plan
                .optimizer
                .candidates
                .iter()
                .any(|candidate| candidate.name == "pure_lftj")
        );
        Ok(())
    }

    #[test]
    fn optimizer_trace_and_cost_tiebreak_are_stable() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?account ?holder_name
            where
              Account(id: ?account, holder: ?holder)
              Holder(id: ?holder, name: ?holder_name)
            "#,
        )?;

        let first = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
        let second = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

        assert_eq!(first.plan.optimizer, second.plan.optimizer);
        assert!(first.explain().contains("candidate_plan"));
        assert!(first.explain().contains("free_join_estimates"));
        assert!(first.explain().contains("reason=stats"));
        Ok(())
    }

    #[test]
    fn planner_stats_are_cached_per_query_image() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?account where Account(id: ?account, holder: $holder)",
        )?;
        let inputs = InputBindings::from_values([("holder", Value::Ref(1))]);

        let first = env.read(|txn| txn.execute_query(&schema, &query, &inputs))?;
        let second = env.read(|txn| txn.execute_query(&schema, &query, &inputs))?;

        assert_eq!(first.rows, second.rows);
        assert_eq!(first.plan.planner_stats.builds, 1);
        assert_eq!(first.plan.planner_stats.misses, 1);
        assert_eq!(second.plan.planner_stats.builds, 1);
        assert_eq!(second.plan.planner_stats.misses, 1);
        assert!(second.plan.planner_stats.hits >= 1);
        if second
            .plan
            .free_join
            .nodes
            .iter()
            .all(|node| node.implementation == NodeImpl::HashProbe)
        {
            assert!(second.plan.counters.hash_probe_calls > 0);
            assert_eq!(second.plan.counters.trie_open, 0);
        } else {
            assert_eq!(second.plan.counters.sorted_trie_builds, 0);
            assert_eq!(second.plan.counters.atom_temp_relation_builds, 0);
            assert!(second.plan.counters.sorted_trie_cache_hits >= 1);
        }
        Ok(())
    }

    #[test]
    fn execute_query_uses_warmed_query_image_cache() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?account where Account(id: ?account, holder: $holder)",
        )?;
        let inputs = InputBindings::from_values([("holder", Value::Ref(1))]);

        let _warm = env.query_image(&schema)?;
        let before = env.query_image_cache_diagnostics();
        let output = env.read(|txn| txn.execute_query(&schema, &query, &inputs))?;
        let after = env.query_image_cache_diagnostics();

        assert_eq!(before.builds, 1);
        assert_eq!(after.builds, 1);
        assert_eq!(output.plan.query_image_cache.builds, 1);
        assert!(output.plan.query_image_cache.hits > before.hits);
        assert_eq!(output.rows, vec![vec![Value::Id(1)], vec![Value::Id(2)]]);
        Ok(())
    }

    #[test]
    fn execute_query_cache_misses_after_write_commit() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?account where Account(id: ?account, holder: ?holder)",
        )?;

        let before = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
        env.write(|txn| {
            txn.insert(&schema, account_row(4, 2, 978))?;
            Ok::<_, Error>(())
        })?;
        let after = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

        assert_eq!(before.plan.query_image_cache.builds, 1);
        assert_eq!(after.plan.query_image_cache.builds, 2);
        assert_eq!(after.rows.len(), before.rows.len() + 1);
        Ok(())
    }

    #[test]
    fn execute_query_cache_is_schema_fingerprint_scoped() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema_a = StorageSchema::new(optimizer_schema(), env.max_key_size())?;
        let schema_b = StorageSchema::new(triangle_schema(), env.max_key_size())?;
        let item_query = parse_and_typecheck(
            schema_a.descriptor(),
            "find ?item where Item(id: ?item, kind: $kind)",
        )?;
        let edge_query =
            parse_and_typecheck(schema_b.descriptor(), "find ?a where EdgeAB(a: ?a, b: ?b)")?;

        let item = env.read(|txn| {
            txn.execute_query(
                &schema_a,
                &item_query,
                &InputBindings::from_values([("kind", Value::Symbol(1))]),
            )
        })?;
        let edge =
            env.read(|txn| txn.execute_query(&schema_b, &edge_query, &InputBindings::new()))?;

        assert_eq!(item.plan.query_image_cache.builds, 1);
        assert_eq!(edge.plan.query_image_cache.builds, 2);
        assert_eq!(edge.plan.query_image_cache.cached_images, 2);
        Ok(())
    }

    #[test]
    fn planner_stats_reuse_shared_relations_across_queries() -> TestResult {
        let (env, schema) = seeded_db()?;
        let first_query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?posting
            where
              Posting(id: ?posting, account: ?account)
              Account(id: ?account, holder: $holder)
            "#,
        )?;
        let second_query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?posting
            where
              Posting(id: ?posting, account: ?account, at: ?t)
              ?t >= $start
            "#,
        )?;

        let inputs = InputBindings::from_values([
            ("holder", Value::Ref(1)),
            ("start", Value::Timestamp(TimestampMicros(0))),
        ]);

        let first = env.read(|txn| txn.execute_query(&schema, &first_query, &inputs))?;
        let second = env.read(|txn| txn.execute_query(&schema, &second_query, &inputs))?;

        assert_eq!(first.plan.planner_stats.builds, 2);
        assert_eq!(second.plan.planner_stats.builds, 2);
        assert!(second.plan.planner_stats.hits >= 1);
        Ok(())
    }

    #[test]
    fn planner_stats_cache_is_snapshot_scoped() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?account where Account(id: ?account, holder: ?holder)",
        )?;

        let before = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
        env.write(|txn| {
            txn.insert(&schema, account_row(4, 2, 978))?;
            Ok::<_, Error>(())
        })?;
        let after = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

        assert_eq!(before.plan.planner_stats.builds, 1);
        assert_eq!(after.plan.planner_stats.builds, 1);
        assert_eq!(after.rows.len(), before.rows.len() + 1);
        Ok(())
    }

    #[test]
    fn normalized_query_preserves_typed_query_shape() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?posting ?amount
            where
              Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
              Account(id: ?account, holder: $holder)
              ?t >= $start
              ?t < $end
            "#,
        )?;

        let normalized = env.read(|txn| normalize_query(txn, &schema, &query))?;

        assert_eq!(normalized.vars.len(), query.variables.len());
        assert_eq!(normalized.inputs.len(), query.inputs.len());
        assert_eq!(normalized.atoms.len(), 2);
        assert_eq!(normalized.predicates.len(), 2);
        assert!(matches!(normalized.output, OutputPlan::Project(_)));
        assert!(matches!(
            normalized.atoms[0].fields[0].term,
            NormTerm::Var(_)
        ));
        Ok(())
    }

    #[test]
    fn repeated_variable_atom_matches_equal_encoded_fields() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = StorageSchema::new(triangle_schema(), env.max_key_size())?;
        env.write(|txn| {
            txn.insert(&schema, edge_ab_row(1, 1))?;
            txn.insert(&schema, edge_ab_row(1, 2))?;
            Ok::<(), Error>(())
        })?;
        let query = parse_and_typecheck(schema.descriptor(), "find ?a where EdgeAB(a: ?a, b: ?a)")?;

        let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

        assert_eq!(output.rows, vec![vec![Value::U64(1)]]);
        Ok(())
    }

    #[test]
    fn predicate_earliest_depth_assignment_is_deterministic() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?posting
            where
              Posting(id: ?posting, account: ?account, at: ?t)
              Account(id: ?account, holder: $holder)
              ?t >= $start
            "#,
        )?;

        let depths = env.read(|txn| {
            let mut normalized = normalize_query(txn, &schema, &query)?;
            let image = QueryImageBuilder::new(txn, &schema).build()?;
            let plan = plan_query(
                &schema,
                &mut normalized,
                &image,
                QueryImageCacheDiagnostics::default(),
            )?;
            let t_depth = plan
                .summary
                .variable_order
                .iter()
                .position(|name| name == "t")
                .ok_or_else(|| Error::internal("missing t variable in plan"))?;
            Ok::<_, Error>((normalized.predicates[0].earliest_depth, t_depth))
        })?;

        assert_eq!(depths.0, Some(depths.1));
        Ok(())
    }

    #[test]
    fn specialized_mock_plan_matches_interpreted_sink_output() -> TestResult {
        struct MockSpecializedPlan {
            bindings: Vec<EncodedBinding>,
        }

        impl ExecutablePlan for MockSpecializedPlan {
            fn execute(
                &mut self,
                txn: &ReadTxn<'_>,
                query: &NormalizedQuery,
                _image: &crate::QueryImage,
                _inputs: &EncodedInputs,
                sink: &mut dyn TupleSink,
            ) -> Result<PlanCounters> {
                let mut counters = PlanCounters::default();
                for binding in &self.bindings {
                    sink.emit(txn, query, binding, &mut counters)?;
                    counters.bindings_yielded += 1;
                }
                Ok(counters)
            }
        }

        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = StorageSchema::new(optimizer_schema(), env.max_key_size())?;
        env.write(|txn| {
            txn.insert(&schema, item_row(1, 1))?;
            Ok::<(), Error>(())
        })?;
        let typed = parse_and_typecheck(
            schema.descriptor(),
            "find ?item where Item(id: ?item, kind: $kind)",
        )?;
        let inputs = InputBindings::from_values([("kind", Value::Symbol(1))]);
        let interpreted = env
            .read(|txn| txn.execute_query(&schema, &typed, &inputs))?
            .rows;

        let specialized = env.read(|txn| {
            let normalized = normalize_query(txn, &schema, &typed)?;
            let encoded_inputs = encode_inputs(txn, &normalized, &inputs)?;
            let image = QueryImageBuilder::new(txn, &schema).build()?;
            let mut binding = EncodedBinding::new(normalized.vars.len());
            let encoded = txn.encode_query_value(&normalized.vars[0].value_type, &Value::Id(1))?;
            assert!(binding.bind(
                0,
                EncodedValue::from_bytes(normalized.vars[0].value_type.clone(), &encoded)?,
            ));
            let mut plan = MockSpecializedPlan {
                bindings: vec![binding],
            };
            let mut sink = OutputSink::new(&normalized.output);
            let _ = plan.execute(txn, &normalized, &image, &encoded_inputs, &mut sink)?;
            sink.finish(txn, &normalized, &mut PlanCounters::default())
        })?;

        assert_same_rows(&specialized, &interpreted);
        Ok(())
    }

    #[test]
    fn executes_two_relation_join() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?account ?holder_name
            where
              Account(id: ?account, holder: ?holder)
              Holder(id: ?holder, name: ?holder_name)
            "#,
        )?;

        let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
        assert!(output.plan.uses_indexed_multiway_join);
        assert_same_rows(
            &output.rows,
            &[
                vec![Value::Id(1), Value::String("Alice".to_owned())],
                vec![Value::Id(2), Value::String("Alice".to_owned())],
                vec![Value::Id(3), Value::String("Bob".to_owned())],
            ],
        );
        Ok(())
    }

    #[test]
    fn executes_many_relation_join_and_range_filter() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?posting ?account ?holder_name
            where
              Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
              Account(id: ?account, holder: ?holder)
              Holder(id: ?holder, name: ?holder_name)
              ?t >= $start
              ?t < $end
            "#,
        )?;

        let output = env.read(|txn| {
            txn.execute_query(
                &schema,
                &query,
                &InputBindings::from_values([
                    ("start", Value::Timestamp(TimestampMicros(15))),
                    ("end", Value::Timestamp(TimestampMicros(35))),
                ]),
            )
        })?;

        assert!(
            output
                .plan
                .variable_estimates
                .iter()
                .any(|estimate| estimate.access == "Posting.by_at")
        );
        assert_same_rows(
            &output.rows,
            &[
                vec![
                    Value::Id(2),
                    Value::Id(1),
                    Value::String("Alice".to_owned()),
                ],
                vec![
                    Value::Id(3),
                    Value::Id(2),
                    Value::String("Alice".to_owned()),
                ],
            ],
        );
        Ok(())
    }

    #[test]
    fn projection_uses_set_semantics() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?holder where Account(id: ?account, holder: ?holder)",
        )?;

        let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;
        assert_eq!(output.rows, vec![vec![Value::Ref(1)], vec![Value::Ref(2)]]);
        assert_eq!(output.plan.counters.bindings_yielded, 3);
        assert_eq!(output.plan.counters.materialized_output_values, 2);
        Ok(())
    }

    #[test]
    fn count_sink_avoids_decoding_counted_variable() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find count(?posting) where Posting(id: ?posting)",
        )?;

        let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

        assert_eq!(output.rows, vec![vec![Value::U64(3)]]);
        assert_eq!(output.plan.counters.bindings_yielded, 3);
        assert_eq!(output.plan.counters.aggregate_groups, 1);
        assert_eq!(output.plan.counters.decoded_values, 0);
        assert_eq!(output.plan.counters.materialized_output_values, 1);
        assert!(
            output.plan.counters.materialized_output_values < output.plan.counters.bindings_yielded
        );
        Ok(())
    }

    #[test]
    fn sum_sink_decodes_only_aggregate_operand_values() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find sum(?amount) count(?posting)
            where
              Posting(id: ?posting, amount: ?amount)
            "#,
        )?;

        let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

        assert_eq!(
            output.rows,
            vec![vec![Value::Decimal(DecimalRaw(600)), Value::U64(3)]]
        );
        assert_eq!(output.plan.counters.bindings_yielded, 3);
        assert_eq!(output.plan.counters.decoded_values, 3);
        assert_eq!(output.plan.counters.materialized_output_values, 2);
        Ok(())
    }

    #[test]
    fn grouped_count_decodes_dictionary_keys_only_at_final_output() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?holder_name count(?account)
            where
              Account(id: ?account, holder: ?holder)
              Holder(id: ?holder, name: ?holder_name)
            "#,
        )?;

        let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

        assert_same_rows(
            &output.rows,
            &[
                vec![Value::String("Alice".to_owned()), Value::U64(2)],
                vec![Value::String("Bob".to_owned()), Value::U64(1)],
            ],
        );
        assert_eq!(output.plan.counters.bindings_yielded, 3);
        assert_eq!(output.plan.counters.decoded_values, 2);
        assert_eq!(output.plan.counters.dictionary_reverse_lookups, 2);
        assert_eq!(output.plan.counters.materialized_output_values, 4);
        Ok(())
    }

    #[test]
    fn aggregate_count_range_uses_multiplicity() -> TestResult {
        let mut sink = AggregateSink::new(&AggregatePlan {
            group_vars: Vec::new(),
            aggregates: vec![AggregateTerm {
                function: AggregateFunction::Count,
                var: VarId(0),
                value_type: ValueType::U64,
            }],
        });
        let binding = EncodedBinding::new(0);

        sink.emit_count_range(&binding, 7)?;

        let states = sink
            .groups
            .get(&SmallEncodedRow::new())
            .ok_or_else(|| Error::internal("missing aggregate state group"))?;
        assert!(matches!(states.as_slice(), [AggregateState::Count(7)]));
        Ok(())
    }

    #[test]
    fn aggregation_groups_and_sums_decimal_values() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?account sum(?amount) count(?posting) min(?t) max(?t)
            where
              Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
            "#,
        )?;

        let output = env.read(|txn| txn.execute_query(&schema, &query, &InputBindings::new()))?;

        assert_same_rows(
            &output.rows,
            &[
                vec![
                    Value::Ref(1),
                    Value::Decimal(DecimalRaw(300)),
                    Value::U64(2),
                    Value::Timestamp(TimestampMicros(10)),
                    Value::Timestamp(TimestampMicros(20)),
                ],
                vec![
                    Value::Ref(2),
                    Value::Decimal(DecimalRaw(300)),
                    Value::U64(1),
                    Value::Timestamp(TimestampMicros(30)),
                    Value::Timestamp(TimestampMicros(30)),
                ],
            ],
        );
        Ok(())
    }

    #[test]
    fn detects_integer_and_decimal_aggregation_overflow() -> TestResult {
        let dir = tempfile::tempdir()?;
        let env = Environment::open(dir.path())?;
        let schema = StorageSchema::new(overflow_schema(), env.max_key_size())?;
        env.write(|txn| {
            txn.insert(&schema, number_row(1, i64::MAX, i128::MAX))?;
            txn.insert(&schema, number_row(2, 1, 1))?;
            Ok::<(), Error>(())
        })?;

        let int_query =
            parse_and_typecheck(schema.descriptor(), "find sum(?n) where Number(n: ?n)")?;
        assert!(matches!(
            env.read(|txn| txn.execute_query(&schema, &int_query, &InputBindings::new())),
            Err(Error::Query(QueryError::Aggregate(
                AggregateError::IntegerOverflow { .. }
            )))
        ));

        let decimal_query =
            parse_and_typecheck(schema.descriptor(), "find sum(?d) where Number(d: ?d)")?;
        assert!(matches!(
            env.read(|txn| txn.execute_query(&schema, &decimal_query, &InputBindings::new())),
            Err(Error::Query(QueryError::Aggregate(
                AggregateError::DecimalOverflow { .. }
            )))
        ));
        Ok(())
    }

    #[test]
    fn input_type_mismatch_is_rejected_at_execution() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            "find ?account where Account(id: ?account, holder: $holder)",
        )?;
        let result = env.read(|txn| {
            txn.execute_query(
                &schema,
                &query,
                &InputBindings::from_values([("holder", Value::String("bad".to_owned()))]),
            )
        });
        assert!(matches!(
            result,
            Err(Error::Query(QueryError::Execute(
                ExecuteError::InputTypeMismatch { .. }
            )))
        ));
        Ok(())
    }

    #[test]
    fn explain_and_storage_diagnostics_are_available() -> TestResult {
        let (env, schema) = seeded_db()?;
        let query = parse_and_typecheck(
            schema.descriptor(),
            r#"
            find ?posting ?amount
            where
              Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
              Account(id: ?account, holder: $holder)
              ?t >= $start
              ?t < $end
            "#,
        )?;

        let output = env.read(|txn| {
            txn.execute_query(
                &schema,
                &query,
                &InputBindings::from_values([
                    ("holder", Value::Ref(1)),
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    ("end", Value::Timestamp(TimestampMicros(100))),
                ]),
            )
        })?;
        let explain = output.explain();
        assert!(explain.contains("variable_order"));
        assert!(explain.contains("runtime_kind"));
        assert!(explain.contains("timings:"));
        assert!(explain.contains("query_timing"));
        assert!(explain.contains("allocations:"));
        assert!(explain.contains("allocation_summary"));
        assert!(explain.contains("node_timing"));
        assert!(explain.contains("variable_estimate"));
        assert!(explain.contains("free_join_node"));
        assert!(explain.contains("candidate_plan"));
        assert!(explain.contains("free_join_estimates"));
        assert!(explain.contains("node_rows"));
        assert!(explain.contains("free_join_subatom"));
        assert!(!explain.contains("atoms:\n"));
        assert!(!explain.contains("index="));
        assert!(explain.contains("cursor_seeks"));
        assert!(explain.contains("rows_scanned"));
        assert!(explain.contains("bindings_yielded"));
        assert!(explain.contains("decoded_values"));
        assert!(explain.contains("encoded_comparisons_evaluated"));
        assert!(explain.contains("materialized_output_values"));
        assert!(explain.contains("trie_open"));
        assert!(explain.contains("trie_seek"));
        assert!(explain.contains("output_rows"));

        let diagnostics = env.storage_diagnostics(&schema)?;
        assert_eq!(diagnostics.storage_tx_id, 1);
        assert!(diagnostics.lmdb_map_size > 0);
        assert!(diagnostics.dictionary_entries > 0);
        assert!(
            diagnostics
                .relations
                .iter()
                .any(|relation| relation.relation == "Account" && relation.row_count == 3)
        );
        assert_eq!(
            diagnostics.schema_fingerprint,
            schema.descriptor().fingerprint().to_string()
        );
        Ok(())
    }

    #[test]
    fn differential_reference_evaluator_matches_lmdb() -> TestResult {
        let (env, schema) = seeded_db()?;
        let reference = ReferenceDb::from_rows(seeded_rows());
        let cases = [
            (
                "find ?account where Account(id: ?account, holder: $holder)",
                InputBindings::from_values([("holder", Value::Ref(1))]),
            ),
            (
                r#"
                find ?account ?holder_name
                where
                  Account(id: ?account, holder: ?holder)
                  Holder(id: ?holder, name: ?holder_name)
                "#,
                InputBindings::new(),
            ),
            (
                r#"
                find ?account sum(?amount) count(?posting)
                where
                  Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
                  ?t >= $start
                  ?t < $end
                "#,
                InputBindings::from_values([
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    ("end", Value::Timestamp(TimestampMicros(100))),
                ]),
            ),
        ];

        for (source, inputs) in cases {
            let query = parse_and_typecheck(schema.descriptor(), source)?;
            let lmdb_rows = env
                .read(|txn| txn.execute_query(&schema, &query, &inputs))?
                .rows;
            let reference_rows = reference.execute(&query, &inputs)?;
            assert_same_rows(&lmdb_rows, &reference_rows);
        }
        Ok(())
    }

    fn seeded_db() -> Result<(Environment, StorageSchema)> {
        let dir = tempfile::tempdir().map_err(|error| Error::io("tempdir", error))?;
        let path = dir.keep();
        let env = Environment::open(&path)?;
        let schema = StorageSchema::new(ledger_schema(), env.max_key_size())?;
        let rows = seeded_rows();
        env.write(|txn| {
            for row in &rows {
                txn.insert(&schema, row.clone())?;
            }
            Ok::<(), Error>(())
        })?;
        Ok((env, schema))
    }

    fn seeded_rows() -> Vec<Row> {
        vec![
            holder_row(1, "Alice"),
            holder_row(2, "Bob"),
            account_row(1, 1, 840),
            account_row(2, 1, 978),
            account_row(3, 2, 840),
            posting_row(1, 1, 100, 10),
            posting_row(2, 1, 200, 20),
            posting_row(3, 2, 300, 30),
        ]
    }

    fn ledger_schema() -> bumbledb_core::schema::SchemaDescriptor {
        bumbledb_core::schema::SchemaDescriptor::new(
            "LedgerDb",
            vec![
                RelationDescriptor::new(
                    "Holder",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "HolderId".to_owned(),
                                relation: "Holder".to_owned(),
                            },
                        ),
                        FieldDescriptor::new("name", ValueType::String),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(bumbledb_core::schema::GeneratedIdDescriptor::new("id")),
                RelationDescriptor::new(
                    "Account",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "AccountId".to_owned(),
                                relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "holder",
                            ValueType::Ref {
                                name: "HolderId".to_owned(),
                                target_relation: "Holder".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "currency",
                            ValueType::Symbol {
                                name: "Currency".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(bumbledb_core::schema::GeneratedIdDescriptor::new("id")),
                RelationDescriptor::new(
                    "Posting",
                    RelationKind::Event,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "PostingId".to_owned(),
                                relation: "Posting".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "account",
                            ValueType::Ref {
                                name: "AccountId".to_owned(),
                                target_relation: "Account".to_owned(),
                            },
                        ),
                        FieldDescriptor::new("amount", ValueType::Decimal { scale: 4 }),
                        FieldDescriptor::new("at", ValueType::TimestampMicros).range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_generated_id(bumbledb_core::schema::GeneratedIdDescriptor::new("id")),
            ],
        )
    }

    fn overflow_schema() -> bumbledb_core::schema::SchemaDescriptor {
        bumbledb_core::schema::SchemaDescriptor::new(
            "OverflowDb",
            vec![RelationDescriptor::new(
                "Number",
                RelationKind::Entity,
                vec![
                    FieldDescriptor::new(
                        "id",
                        ValueType::Id {
                            name: "NumberId".to_owned(),
                            relation: "Number".to_owned(),
                        },
                    ),
                    FieldDescriptor::new("n", ValueType::I64),
                    FieldDescriptor::new("d", ValueType::Decimal { scale: 0 }),
                ],
                PrimaryKeyDescriptor::new(["id"]),
            )],
        )
    }

    fn optimizer_schema() -> bumbledb_core::schema::SchemaDescriptor {
        bumbledb_core::schema::SchemaDescriptor::new(
            "OptimizerDb",
            vec![
                RelationDescriptor::new(
                    "Item",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new(
                            "id",
                            ValueType::Id {
                                name: "ItemId".to_owned(),
                                relation: "Item".to_owned(),
                            },
                        ),
                        FieldDescriptor::new(
                            "kind",
                            ValueType::Symbol {
                                name: "Kind".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_index(IndexDescriptor::equality("by_kind", ["kind", "id"])),
            ],
        )
    }

    fn triangle_schema() -> bumbledb_core::schema::SchemaDescriptor {
        bumbledb_core::schema::SchemaDescriptor::new(
            "TriangleDb",
            vec![
                RelationDescriptor::new(
                    "EdgeAB",
                    RelationKind::Edge,
                    vec![
                        FieldDescriptor::new("a", ValueType::U64),
                        FieldDescriptor::new("b", ValueType::U64),
                    ],
                    PrimaryKeyDescriptor::new(["a", "b"]),
                ),
                RelationDescriptor::new(
                    "EdgeAC",
                    RelationKind::Edge,
                    vec![
                        FieldDescriptor::new("a", ValueType::U64),
                        FieldDescriptor::new("c", ValueType::U64),
                    ],
                    PrimaryKeyDescriptor::new(["a", "c"]),
                ),
                RelationDescriptor::new(
                    "EdgeBC",
                    RelationKind::Edge,
                    vec![
                        FieldDescriptor::new("b", ValueType::U64),
                        FieldDescriptor::new("c", ValueType::U64),
                    ],
                    PrimaryKeyDescriptor::new(["b", "c"]),
                ),
            ],
        )
    }

    fn chain_schema() -> bumbledb_core::schema::SchemaDescriptor {
        bumbledb_core::schema::SchemaDescriptor::new(
            "ChainDb",
            vec![
                RelationDescriptor::new(
                    "A",
                    RelationKind::Entity,
                    vec![FieldDescriptor::new("id", ValueType::U64)],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "B",
                    RelationKind::Entity,
                    vec![
                        FieldDescriptor::new("id", ValueType::U64),
                        FieldDescriptor::new("a", ValueType::U64),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_index(IndexDescriptor::equality("by_a", ["a", "id"])),
            ],
        )
    }

    fn holder_row(id: u64, name: &str) -> Row {
        Row::new(
            "Holder",
            [
                ("id", Value::Id(id)),
                ("name", Value::String(name.to_owned())),
            ],
        )
    }

    fn account_row(id: u64, holder: u64, currency: u64) -> Row {
        Row::new(
            "Account",
            [
                ("id", Value::Id(id)),
                ("holder", Value::Ref(holder)),
                ("currency", Value::Symbol(currency)),
            ],
        )
    }

    fn posting_row(id: u64, account: u64, amount: i128, at: i64) -> Row {
        Row::new(
            "Posting",
            [
                ("id", Value::Id(id)),
                ("account", Value::Ref(account)),
                ("amount", Value::Decimal(DecimalRaw(amount))),
                ("at", Value::Timestamp(TimestampMicros(at))),
            ],
        )
    }

    fn number_row(id: u64, n: i64, d: i128) -> Row {
        Row::new(
            "Number",
            [
                ("id", Value::Id(id)),
                ("n", Value::I64(n)),
                ("d", Value::Decimal(DecimalRaw(d))),
            ],
        )
    }

    fn item_row(id: u64, kind: u64) -> Row {
        Row::new(
            "Item",
            [("id", Value::Id(id)), ("kind", Value::Symbol(kind))],
        )
    }

    fn edge_ab_row(a: u64, b: u64) -> Row {
        Row::new("EdgeAB", [("a", Value::U64(a)), ("b", Value::U64(b))])
    }

    fn edge_ac_row(a: u64, c: u64) -> Row {
        Row::new("EdgeAC", [("a", Value::U64(a)), ("c", Value::U64(c))])
    }

    fn edge_bc_row(b: u64, c: u64) -> Row {
        Row::new("EdgeBC", [("b", Value::U64(b)), ("c", Value::U64(c))])
    }

    fn b_row(id: u64, a: u64) -> Row {
        Row::new("B", [("id", Value::U64(id)), ("a", Value::U64(a))])
    }

    fn assert_same_rows(actual: &[Vec<Value>], expected: &[Vec<Value>]) {
        let mut actual = actual.to_vec();
        let mut expected = expected.to_vec();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected);
    }

    struct ReferenceDb {
        rows: BTreeMap<String, Vec<Row>>,
    }

    #[derive(Clone, Debug)]
    struct ReferenceBinding {
        values: Vec<Option<Value>>,
    }

    impl ReferenceBinding {
        fn new(variable_count: usize) -> Self {
            Self {
                values: vec![None; variable_count],
            }
        }

        fn get(&self, variable: usize) -> Option<&Value> {
            self.values[variable].as_ref()
        }

        fn bind(&mut self, variable: usize, value: Value) -> bool {
            match &self.values[variable] {
                Some(existing) => existing == &value,
                None => {
                    self.values[variable] = Some(value);
                    true
                }
            }
        }
    }

    impl ReferenceDb {
        fn from_rows(rows: Vec<Row>) -> Self {
            let mut by_relation: BTreeMap<String, Vec<Row>> = BTreeMap::new();
            for row in rows {
                by_relation
                    .entry(row.relation().to_owned())
                    .or_default()
                    .push(row);
            }
            Self { rows: by_relation }
        }

        fn execute(&self, query: &TypedQuery, inputs: &InputBindings) -> Result<Vec<Vec<Value>>> {
            validate_inputs(query, inputs)?;
            let atoms = query
                .clauses
                .iter()
                .filter_map(|clause| match clause {
                    TypedClause::Relation(atom) => Some(atom),
                    TypedClause::Comparison(_) => None,
                })
                .collect::<Vec<_>>();
            let comparisons = query
                .clauses
                .iter()
                .filter_map(|clause| match clause {
                    TypedClause::Comparison(comparison) => Some(comparison),
                    TypedClause::Relation(_) => None,
                })
                .collect::<Vec<_>>();
            let mut output = Vec::new();
            let mut counters = PlanCounters::default();
            self.recurse(
                query,
                inputs,
                &atoms,
                &comparisons,
                0,
                ReferenceBinding::new(query.variables.len()),
                &mut output,
                &mut counters,
            )?;
            reference_project_results(query, &output)
        }

        #[expect(
            clippy::too_many_arguments,
            reason = "test reference recursion carries explicit evaluator state"
        )]
        fn recurse(
            &self,
            query: &TypedQuery,
            inputs: &InputBindings,
            atoms: &[&TypedRelationAtom],
            comparisons: &[&TypedComparison],
            depth: usize,
            binding: ReferenceBinding,
            output: &mut Vec<ReferenceBinding>,
            counters: &mut PlanCounters,
        ) -> Result<()> {
            if depth == atoms.len() {
                if reference_comparisons_pass(comparisons, query, inputs, &binding, counters)? {
                    output.push(binding);
                }
                return Ok(());
            }

            let atom = atoms[depth];
            for row in self.rows.get(&atom.relation).into_iter().flatten() {
                let Some(next) = reference_match_atom(atom, query, inputs, &binding, row)? else {
                    continue;
                };
                if reference_comparisons_pass(comparisons, query, inputs, &next, counters)? {
                    self.recurse(
                        query,
                        inputs,
                        atoms,
                        comparisons,
                        depth + 1,
                        next,
                        output,
                        counters,
                    )?;
                }
            }
            Ok(())
        }
    }

    fn reference_match_atom(
        atom: &TypedRelationAtom,
        query: &TypedQuery,
        inputs: &InputBindings,
        binding: &ReferenceBinding,
        row: &Row,
    ) -> Result<Option<ReferenceBinding>> {
        let mut next = binding.clone();
        for field in &atom.fields {
            let Some(row_value) = row.value(&field.field) else {
                return Ok(None);
            };
            match &field.term {
                TypedTerm::Variable(variable) => {
                    let normalized =
                        normalize_value_for_type(row_value, &query.variables[*variable].value_type);
                    if !next.bind(*variable, normalized) {
                        return Ok(None);
                    }
                }
                TypedTerm::Input(input) => {
                    let input_value = input_value(query, inputs, *input)?;
                    let normalized =
                        normalize_value_for_type(row_value, &query.inputs[*input].value_type);
                    if input_value != &normalized {
                        return Ok(None);
                    }
                }
                TypedTerm::Literal(literal) => {
                    let normalized = normalize_value_for_type(row_value, &literal.value_type);
                    if literal_to_value(literal)? != normalized {
                        return Ok(None);
                    }
                }
                TypedTerm::Wildcard => {}
            }
        }
        Ok(Some(next))
    }

    fn reference_comparisons_pass(
        comparisons: &[&TypedComparison],
        query: &TypedQuery,
        inputs: &InputBindings,
        binding: &ReferenceBinding,
        counters: &mut PlanCounters,
    ) -> Result<bool> {
        for comparison in comparisons {
            let Some(left) = reference_operand_value(&comparison.left, query, inputs, binding)?
            else {
                continue;
            };
            let Some(right) = reference_operand_value(&comparison.right, query, inputs, binding)?
            else {
                continue;
            };
            counters.comparisons_evaluated += 1;
            let left = normalize_value_for_type(&left, &comparison.value_type);
            let right = normalize_value_for_type(&right, &comparison.value_type);
            if !compare_values(&left, comparison.operator, &right) {
                counters.comparisons_failed += 1;
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn reference_operand_value(
        operand: &TypedOperand,
        query: &TypedQuery,
        inputs: &InputBindings,
        binding: &ReferenceBinding,
    ) -> Result<Option<Value>> {
        Ok(match operand {
            TypedOperand::Variable(variable) => binding.get(*variable).cloned(),
            TypedOperand::Input(input) => Some(input_value(query, inputs, *input)?.clone()),
            TypedOperand::Literal(literal) => Some(literal_to_value(literal)?),
        })
    }

    fn reference_project_results(
        query: &TypedQuery,
        bindings: &[ReferenceBinding],
    ) -> Result<Vec<Vec<Value>>> {
        let has_aggregate = query
            .find
            .iter()
            .any(|term| matches!(term, TypedFindTerm::Aggregate { .. }));
        if has_aggregate {
            reference_project_aggregates(query, bindings)
        } else {
            let mut set = BTreeSet::new();
            for binding in bindings {
                let mut row = Vec::new();
                for term in &query.find {
                    let TypedFindTerm::Variable { variable } = term else {
                        continue;
                    };
                    row.push(reference_bound_variable(binding, *variable)?.clone());
                }
                set.insert(row);
            }
            Ok(set.into_iter().collect())
        }
    }

    fn reference_project_aggregates(
        query: &TypedQuery,
        bindings: &[ReferenceBinding],
    ) -> Result<Vec<Vec<Value>>> {
        let group_terms = query
            .find
            .iter()
            .filter_map(|term| match term {
                TypedFindTerm::Variable { variable } => Some(*variable),
                TypedFindTerm::Aggregate { .. } => None,
            })
            .collect::<Vec<_>>();
        let aggregate_terms = query
            .find
            .iter()
            .filter_map(|term| match term {
                TypedFindTerm::Aggregate {
                    function,
                    variable,
                    value_type,
                } => Some((*function, *variable, value_type.clone())),
                TypedFindTerm::Variable { .. } => None,
            })
            .collect::<Vec<_>>();

        let mut groups: BTreeMap<Vec<Value>, Vec<AggregateState>> = BTreeMap::new();
        for binding in bindings {
            let key = group_terms
                .iter()
                .map(|variable| reference_bound_variable(binding, *variable).cloned())
                .collect::<Result<Vec<_>>>()?;
            let states = groups.entry(key).or_insert_with(|| {
                aggregate_terms
                    .iter()
                    .map(|(function, _, value_type)| {
                        AggregateState::new(*function, value_type.clone())
                    })
                    .collect()
            });
            for (state, (_, variable, _)) in states.iter_mut().zip(&aggregate_terms) {
                state.apply(reference_bound_variable(binding, *variable)?)?;
            }
        }

        let mut rows = Vec::new();
        for (key, states) in groups {
            let mut row = Vec::new();
            let mut key_iter = key.into_iter();
            let mut state_iter = states.into_iter();
            for term in &query.find {
                match term {
                    TypedFindTerm::Variable { .. } => {
                        row.push(key_iter.next().ok_or_else(|| {
                            Error::internal("missing reference aggregate group key")
                        })?)
                    }
                    TypedFindTerm::Aggregate { .. } => {
                        let state = state_iter
                            .next()
                            .ok_or_else(|| Error::internal("missing reference aggregate state"))?;
                        row.push(state.finish()?)
                    }
                }
            }
            rows.push(row);
        }
        rows.sort();
        Ok(rows)
    }

    fn reference_bound_variable(binding: &ReferenceBinding, variable: usize) -> Result<&Value> {
        binding
            .get(variable)
            .ok_or_else(|| Error::internal(format!("variable {variable} is unbound at projection")))
    }
}
