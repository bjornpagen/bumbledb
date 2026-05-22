use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use smallvec::SmallVec;

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_ir::{
    AggregateFunction, ComparisonOperator, Literal, TypedClause, TypedComparison, TypedFindTerm,
    TypedLiteral, TypedOperand, TypedQuery, TypedRelationAtom, TypedTerm,
};
use bumbledb_core::schema::{IndexKind, SchemaFingerprint, ValueType};

use crate::hash_trie::PrefixRows;
use crate::query_image::{RowId, RowRange};
use crate::{
    AccessId, AggregatePlan, AggregateTerm, AtomId, EncodedOwned, Error, FieldId, FieldValues,
    FreeJoinPlan, HashTrieIndex, IndexSpec, LeafMode, LinearIter, NodeId, NodeImpl, OutputPlan,
    PayloadDemand, PlanEstimates, PlanNode, PrefixProbe, ProjectPlan, ReadTxn, RelationImage,
    RelationIndexImage, RelationStats, Result, Row, SortedTrieIndex, StorageSchema, SubAtom,
    TrieIter, Value, VarId,
};

use crate::allocation::{self, ALLOCATION_SIZE_CLASS_COUNT, AllocationDelta};
use crate::planner_stats::{
    OptimizerFieldStats, OptimizerIndexStats, OptimizerRelationStats, PlannerStatsCacheDiagnostics,
};
use crate::query_access::{AccessProbe, AccessSource, encoded_refs};
use crate::query_image::{
    EncodedColumnBuilder, HashTrieKey, LftjAtomKey, QueryImageKey, QueryImageScope, QueryShapeKey,
    SortedTrieBuild, StaticProofCacheKey, StaticProofCacheValue, StaticProofKind,
    encoded_column_builders, finish_column_builders,
};
use crate::{PreparedPlanCacheDiagnostics, QueryImageCacheDiagnostics};

const HASH_BUILD_ROWS_PER_MICRO: u64 = 5;
const STATIC_SEMIJOIN_MAX_PROBES: u64 = 2_048;
const STATIC_SEMIJOIN_MAX_SCANNED_ROWS: u64 = 2_048;
const STATIC_SEMIJOIN_MAX_SEED_CANDIDATES: usize = 1_024;
const STATIC_SEMIJOIN_MAX_CANDIDATES: usize = 1_024;
const STATIC_SEMIJOIN_MAX_OUTPUT_VARS: usize = 1;
const STATIC_SEMIJOIN_MAX_ROUNDS: u64 = 4;
const STATIC_SEMIJOIN_SCAN_THRESHOLD: usize = 256;

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

/// Executor-friendly normalized typed query IR.
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
    /// Aggregate over an explicit set domain.
    Aggregate {
        /// Aggregate function.
        function: AggregateFunction,
        /// Measured variable or first domain variable for domain count.
        variable: VarId,
        /// Distinct set domain for this aggregate.
        domain: Vec<VarId>,
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

/// One tuple in a query result set.
pub type ResultTuple = Vec<Value>;

/// Duplicate-free query result set in canonical tuple order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryResultSet {
    /// Result columns in projection order.
    pub columns: Vec<ResultColumn>,
    /// Result tuples in canonical order.
    pub tuples: Vec<ResultTuple>,
}

impl QueryResultSet {
    /// Builds a canonical result set from possibly unordered tuples.
    pub fn new(columns: Vec<ResultColumn>, mut tuples: Vec<ResultTuple>) -> Self {
        tuples.sort();
        tuples.dedup();
        Self { columns, tuples }
    }

    /// Number of tuples in the set.
    pub fn cardinality(&self) -> usize {
        self.tuples.len()
    }
}

/// Query execution output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryOutput {
    /// Duplicate-free result set.
    pub result: QueryResultSet,
    /// Physical plan and counters.
    pub plan: QueryPlan,
}

/// Explicit per-query execution cache controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryExecutionOptions {
    /// Allows prepared count/result cache hits and inserts.
    pub allow_prepared_result_cache: bool,
    /// Allows static-empty result cache hits and inserts.
    pub allow_static_empty_fast_cache: bool,
}

impl QueryExecutionOptions {
    /// Default cached execution behavior.
    pub const fn cached() -> Self {
        Self {
            allow_prepared_result_cache: true,
            allow_static_empty_fast_cache: true,
        }
    }

    /// Recompute result-producing optimizations while keeping normal plan/image caches available.
    pub const fn without_result_caches() -> Self {
        Self {
            allow_prepared_result_cache: false,
            allow_static_empty_fast_cache: false,
        }
    }
}

impl Default for QueryExecutionOptions {
    fn default() -> Self {
        Self::cached()
    }
}

/// Result-set cardinality output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryResultCardinality {
    /// Number of logical output tuples.
    pub cardinality: usize,
    /// Physical plan and counters.
    pub plan: QueryPlan,
}

/// Reusable typed query shape with snapshot-local normalized query cache.
#[derive(Debug)]
pub struct PreparedQuery {
    schema: SchemaFingerprint,
    query: TypedQuery,
    normalized: RwLock<BTreeMap<u64, Arc<NormalizedQuery>>>,
}

impl PreparedQuery {
    pub(crate) fn new(schema: &StorageSchema, query: TypedQuery) -> Self {
        Self {
            schema: schema.descriptor().fingerprint(),
            query,
            normalized: RwLock::default(),
        }
    }

    fn query(&self) -> &TypedQuery {
        &self.query
    }

    fn normalized_for(
        &self,
        txn: &ReadTxn<'_>,
        schema: &StorageSchema,
    ) -> Result<(Arc<NormalizedQuery>, bool)> {
        let schema_fingerprint = schema.descriptor().fingerprint();
        if self.schema != schema_fingerprint {
            return Err(Error::schema_mismatch(
                self.schema.to_string(),
                schema_fingerprint.to_string(),
            ));
        }
        let tx_id = txn.last_committed_tx_id()?;
        if let Some(normalized) = self
            .normalized
            .read()
            .map_err(|_| Error::internal("prepared query cache read lock poisoned"))?
            .get(&tx_id)
            .cloned()
        {
            return Ok((normalized, false));
        }
        let normalized = Arc::new(normalize_query(txn, schema, &self.query)?);
        let mut cache = self
            .normalized
            .write()
            .map_err(|_| Error::internal("prepared query cache write lock poisoned"))?;
        if let Some(existing) = cache.get(&tx_id).cloned() {
            return Ok((existing, false));
        }
        cache.insert(tx_id, normalized.clone());
        Ok((normalized, true))
    }
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
    /// Top-level physical runtime family selected by planning/classification.
    pub plan_family: PlanFamily,
    /// Query image cache diagnostics after acquiring this query image.
    pub query_image_cache: QueryImageCacheDiagnostics,
    /// Planner statistics cache diagnostics after planning.
    pub planner_stats: PlannerStatsCacheDiagnostics,
    /// Prepared physical plan cache diagnostics after planning.
    pub prepared_plan_cache: PreparedPlanCacheDiagnostics,
    /// Node-level estimated and observed row/candidate counts.
    pub node_rows: Vec<NodeRowEstimate>,
    /// Node-level execution summaries.
    pub node_timings: Vec<QueryNodeTiming>,
    /// Free Join physical plan IR.
    pub free_join: FreeJoinPlan,
    /// Optional direct kernel selected for a simple hot query shape.
    pub direct_kernel: Option<DirectKernelSummary>,
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
        out.push_str(&format!("plan_family: {:?}\n", self.plan_family));
        out.push_str(&format!(
            "uses_indexed_multiway_join: {}\n",
            self.uses_indexed_multiway_join
        ));
        out.push_str("timings:\n");
        out.push_str(&format!(
            "  query_timing total_micros={} validate_inputs_micros={} normalize_micros={} encode_inputs_micros={} query_image_micros={} static_empty_lookup_micros={} static_literal_proof_micros={} static_semijoin_proof_micros={} direct_storage_micros={} plan_micros={} lftj_build_micros={} hash_index_micros={} execute_micros={} lftj_execute_micros={} hash_execute_micros={} sink_emit_micros={} sink_finish_micros={} decode_micros={} unaccounted_micros={}\n",
            self.timings.total_micros,
            self.timings.validate_inputs_micros,
            self.timings.normalize_micros,
            self.timings.encode_inputs_micros,
            self.timings.query_image_micros,
            self.timings.static_empty_lookup_micros,
            self.timings.static_literal_proof_micros,
            self.timings.static_semijoin_proof_micros,
            self.timings.direct_storage_micros,
            self.timings.plan_micros,
            self.timings.lftj_build_micros,
            self.timings.hash_index_micros,
            self.timings.execute_micros,
            self.timings.lftj_execute_micros,
            self.timings.hash_execute_micros,
            self.timings.sink_emit_micros,
            self.timings.sink_finish_micros,
            self.timings.decode_micros,
            self.timings.unaccounted_micros
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
            "  planner_stats cached_relations={} hits={} misses={} builds={} build_micros={} field_stats_built={} index_stats_built={} stats_from_segments={} stats_exact_scans={}\n",
            self.planner_stats.cached_relations,
            self.planner_stats.hits,
            self.planner_stats.misses,
            self.planner_stats.builds,
            self.planner_stats.build_micros,
            self.planner_stats.field_stats_built,
            self.planner_stats.index_stats_built,
            self.planner_stats.stats_from_segments,
            self.planner_stats.stats_exact_scans
        ));
        out.push_str(&format!(
            "  prepared_plan_cache cached_plans={} hits={} misses={} builds={} build_micros={}\n",
            self.prepared_plan_cache.cached_plans,
            self.prepared_plan_cache.hits,
            self.prepared_plan_cache.misses,
            self.prepared_plan_cache.builds,
            self.prepared_plan_cache.build_micros
        ));
        out.push_str(&format!("  chosen_plan: {}\n", self.optimizer.chosen));
        for candidate in &self.optimizer.candidates {
            out.push_str(&format!(
                "  candidate_plan name={} family={:?} selected={} estimated_micros={} setup_micros={} memory_bytes={} materialization_penalty={} candidate_rank={} implementation_mask={} rejected_reason={} impls={:?}\n",
                candidate.name,
                candidate.family,
                candidate.selected,
                candidate.cost.estimated_micros,
                candidate.cost.setup_micros,
                candidate.cost.memory_bytes,
                candidate.cost.materialization_penalty,
                candidate.cost.candidate_rank,
                candidate.cost.implementation_mask,
                candidate.rejected_reason,
                candidate.implementations
            ));
        }
        out.push_str(&format!(
            "free_join_estimates: output_rows={} iterator_ops={} hash_build_rows={} materialized_values={} memory_bytes={} actual_output_rows={}\n",
            self.free_join.estimates.output_rows,
            self.free_join.estimates.iterator_ops,
            self.free_join.estimates.hash_build_rows,
            self.free_join.estimates.materialized_values,
            self.free_join.estimates.memory_bytes,
            self.counters.output_rows
        ));
        if let Some(direct) = &self.direct_kernel {
            out.push_str(&format!(
                "direct_kernel kind={:?} target={} steps={}\n",
                direct.kind, direct.target, direct.steps
            ));
        }
        if self.runtime_kind == QueryRuntimeKind::StaticEmpty {
            out.push_str(&format!(
                "static_empty cache_hits={} cache_misses={} atoms_checked={} rows_scanned={} semijoin_prefixes_probed={} semijoin_candidate_values={} semijoin_rounds={}\n",
                self.counters.static_empty_cache_hits,
                self.counters.static_empty_cache_misses,
                self.counters.static_empty_atoms_checked,
                self.counters.static_empty_rows_scanned,
                self.counters.static_semijoin_prefixes_probed,
                self.counters.static_semijoin_candidate_values,
                self.counters.static_semijoin_rounds,
            ));
        }
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
            "  lftj_atom_source_rows_scanned: {}\n",
            self.counters.lftj_atom_source_rows_scanned
        ));
        out.push_str(&format!(
            "  lftj_atom_rows_retained: {}\n",
            self.counters.lftj_atom_rows_retained
        ));
        out.push_str(&format!(
            "  lftj_atom_bytes_copied: {}\n",
            self.counters.lftj_atom_bytes_copied
        ));
        out.push_str(&format!(
            "  lftj_atom_scan_micros: {}\n",
            self.counters.lftj_atom_scan_micros
        ));
        out.push_str(&format!(
            "  lftj_atom_column_micros: {}\n",
            self.counters.lftj_atom_column_micros
        ));
        out.push_str(&format!(
            "  lftj_atom_sort_micros: {}\n",
            self.counters.lftj_atom_sort_micros
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
            "  hash_rows_returned: {}\n",
            self.counters.hash_rows_returned
        ));
        out.push_str(&format!(
            "  hash_distinct_emits: {}\n",
            self.counters.hash_distinct_emits
        ));
        out.push_str(&format!(
            "  direct_kernel_probes: {}\n",
            self.counters.direct_kernel_probes
        ));
        out.push_str(&format!(
            "  direct_kernel_rows: {}\n",
            self.counters.direct_kernel_rows
        ));
        out.push_str(&format!(
            "  direct_kernel_predicates: {}\n",
            self.counters.direct_kernel_predicates
        ));
        out.push_str(&format!(
            "  static_empty_atoms_checked: {}\n",
            self.counters.static_empty_atoms_checked
        ));
        out.push_str(&format!(
            "  static_empty_rows_scanned: {}\n",
            self.counters.static_empty_rows_scanned
        ));
        out.push_str(&format!(
            "  static_empty_cache_hits: {}\n",
            self.counters.static_empty_cache_hits
        ));
        out.push_str(&format!(
            "  static_empty_cache_misses: {}\n",
            self.counters.static_empty_cache_misses
        ));
        out.push_str(&format!(
            "  static_semijoin_prefixes_probed: {}\n",
            self.counters.static_semijoin_prefixes_probed
        ));
        out.push_str(&format!(
            "  static_semijoin_candidate_values: {}\n",
            self.counters.static_semijoin_candidate_values
        ));
        out.push_str(&format!(
            "  static_semijoin_rounds: {}\n",
            self.counters.static_semijoin_rounds
        ));
        out.push_str(&format!(
            "  static_semijoin_skipped: {}\n",
            self.counters.static_semijoin_skipped
        ));
        out.push_str(&format!(
            "  static_semijoin_skipped_reason: {}\n",
            self.counters.static_semijoin_skipped_reason.as_str()
        ));
        out.push_str(&format!(
            "  prepared_result_cache_hits: {}\n",
            self.counters.prepared_result_cache_hits
        ));
        out.push_str(&format!(
            "  prepared_result_cache_misses: {}\n",
            self.counters.prepared_result_cache_misses
        ));
        out.push_str(&format!(
            "  prepared_result_cache_inserts: {}\n",
            self.counters.prepared_result_cache_inserts
        ));
        out.push_str(&format!(
            "  prepared_result_cache_bypasses: {}\n",
            self.counters.prepared_result_cache_bypasses
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
    /// Acyclic index nested-loop executor.
    IndexNestedLoop,
    /// Query was proven empty by static literal atom analysis before planning.
    StaticEmpty,
    /// Reserved for direct selective kernels.
    DirectKernel,
}

/// Top-level physical plan family.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlanFamily {
    /// Runtime not selected yet.
    #[default]
    Unknown,
    /// Direct current-index/storage execution.
    Direct,
    /// Acyclic index nested-loop family.
    IndexNestedLoop,
    /// Free Join/LFTJ family.
    FreeJoinLftj,
    /// Static empty proof family.
    StaticEmpty,
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
    /// Static-empty cache lookup time.
    pub static_empty_lookup_micros: u128,
    /// Static literal atom proof time.
    pub static_literal_proof_micros: u128,
    /// Static semijoin proof time.
    pub static_semijoin_proof_micros: u128,
    /// Direct storage projection execution time.
    pub direct_storage_micros: u128,
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
    /// Inclusive total minus non-overlapping known top-level phases.
    pub unaccounted_micros: u128,
}

impl QueryTimings {
    /// Returns the non-overlapping phase total used for unaccounted timing.
    pub fn accounted_micros(&self) -> u128 {
        let direct_runtime = self.direct_storage_micros;
        let runtime_micros = if direct_runtime > 0 {
            direct_runtime
        } else {
            self.execute_micros
        };
        self.validate_inputs_micros
            .saturating_add(self.normalize_micros)
            .saturating_add(self.encode_inputs_micros)
            .saturating_add(self.query_image_micros)
            .saturating_add(self.static_empty_lookup_micros)
            .saturating_add(self.static_literal_proof_micros)
            .saturating_add(self.static_semijoin_proof_micros)
            .saturating_add(self.plan_micros)
            .saturating_add(self.lftj_build_micros)
            .saturating_add(self.hash_index_micros)
            .saturating_add(runtime_micros)
            .saturating_add(self.sink_finish_micros)
            .saturating_add(self.decode_micros)
    }

    /// Refreshes unaccounted timing from the current total and known phases.
    pub fn refresh_unaccounted(&mut self) {
        self.unaccounted_micros = self.total_micros.saturating_sub(self.accounted_micros());
    }
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

/// Direct kernel summary for explain/benchmark output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectKernelSummary {
    /// Direct kernel kind.
    pub kind: DirectKernelKind,
    /// Human-readable target shape.
    pub target: String,
    /// Number of direct probe/scan steps.
    pub steps: usize,
}

/// Direct kernel kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectKernelKind {
    /// Single point lookup.
    PointLookup,
    /// Equality prefix plus range predicates.
    PrefixRange,
    /// Acyclic low-fanout chain probe.
    ChainProbe,
    /// Count-only direct aggregate.
    CountOnly,
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
    /// Runtime family for this candidate.
    pub family: PlanFamily,
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
    /// Estimated setup/build time in microseconds.
    pub setup_micros: u64,
    /// Estimated extra memory footprint in bytes.
    pub memory_bytes: usize,
    /// Penalty for materializing output values or intermediate payload.
    pub materialization_penalty: u64,
    /// Stable candidate rank tie-breaker.
    pub candidate_rank: u8,
    /// Stable implementation-shape tie-breaker.
    pub implementation_mask: u64,
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

/// Reason static semijoin proof did not run after cheap preflight.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StaticSemijoinSkipReason {
    /// Proof was not skipped.
    #[default]
    NotSkipped,
    /// Query has fewer than two relation atoms.
    TooFewAtoms,
    /// Query has no static literal/input/range seed.
    NoStaticConstraint,
    /// Query output shape is too broad for speculative empty proof.
    OutputTooBroad,
    /// No exact low-cardinality seed could be produced cheaply.
    NoCheapExactSeed,
    /// A previous proof on the same snapshot/query/input was inconclusive.
    NegativeCache,
}

impl StaticSemijoinSkipReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::NotSkipped => "not_skipped",
            Self::TooFewAtoms => "too_few_atoms",
            Self::NoStaticConstraint => "no_static_constraint",
            Self::OutputTooBroad => "output_too_broad",
            Self::NoCheapExactSeed => "no_cheap_exact_seed",
            Self::NegativeCache => "negative_cache",
        }
    }
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
    /// Number of complete bindings that reached an output boundary.
    pub bindings_completed: u64,
    /// Number of sink emit calls.
    pub sink_emit_calls: u64,
    /// Number of aggregate sink emit calls.
    pub aggregate_emit_calls: u64,
    /// Number of aggregate sink count-range emit calls.
    pub aggregate_count_range_calls: u64,
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
    /// Number of LFTJ iterator open operations.
    pub lftj_open_calls: u64,
    /// Number of trie iterator up operations.
    pub trie_up: u64,
    /// Number of LFTJ iterator up operations.
    pub lftj_up_calls: u64,
    /// Number of trie iterator next operations.
    pub trie_next: u64,
    /// Number of LFTJ iterator next operations.
    pub lftj_next_calls: u64,
    /// Number of trie iterator seek operations.
    pub trie_seek: u64,
    /// Number of LFTJ iterator seek operations.
    pub lftj_seek_calls: u64,
    /// Number of trie iterator key reads.
    pub trie_key_reads: u64,
    /// Number of LFTJ iterator key reads.
    pub lftj_key_reads: u64,
    /// Number of LFTJ candidate values considered.
    pub lftj_candidate_values: u64,
    /// Number of successful LFTJ variable binds.
    pub lftj_bind_successes: u64,
    /// Number of rejected LFTJ variable binds.
    pub lftj_bind_rejects: u64,
    /// Number of LFTJ completed bindings.
    pub lftj_completed_bindings: u64,
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
    /// Number of source rows inspected by LFTJ atom build subphase tracing.
    pub lftj_atom_source_rows_scanned: u64,
    /// Number of rows retained by LFTJ atom build subphase tracing.
    pub lftj_atom_rows_retained: u64,
    /// Number of encoded bytes copied by LFTJ atom build subphase tracing.
    pub lftj_atom_bytes_copied: u64,
    /// LFTJ atom scan/filter/copy microseconds.
    pub lftj_atom_scan_micros: u64,
    /// LFTJ atom temporary column construction microseconds.
    pub lftj_atom_column_micros: u64,
    /// LFTJ atom sorted trie construction microseconds.
    pub lftj_atom_sort_micros: u64,
    /// Number of hash trie indexes built for direct execution.
    pub hash_index_builds: u64,
    /// Number of source rows used to build hash indexes.
    pub hash_index_build_rows: u64,
    /// Number of row IDs returned from hash prefix probes.
    pub hash_rows_returned: u64,
    /// Number of bindings emitted from hash-backed direct nodes.
    pub hash_distinct_emits: u64,
    /// Number of direct kernel prefix/point probes.
    pub direct_kernel_probes: u64,
    /// Number of relation rows visited by direct kernels.
    pub direct_kernel_rows: u64,
    /// Number of predicates evaluated by direct kernels.
    pub direct_kernel_predicates: u64,
    /// Number of direct variable bind attempts.
    pub direct_bind_attempts: u64,
    /// Number of successful direct variable binds.
    pub direct_bind_successes: u64,
    /// Number of direct chain steps entered.
    pub direct_chain_steps: u64,
    /// Number of direct chain step rows observed.
    pub direct_chain_step_rows: u64,
    /// Number of direct chain output rows emitted.
    pub direct_chain_output_rows: u64,
    /// Number of direct chain output values emitted.
    pub direct_chain_output_values: u64,
    /// Number of direct storage output rows emitted.
    pub direct_storage_output_rows: u64,
    /// Number of direct rows appended through the direct batch projection path.
    pub direct_batch_rows: u64,
    /// Number of encoded bytes appended through the direct batch projection path.
    pub direct_batch_row_bytes: u64,
    /// Number of direct rows that fell back to generic sink emission.
    pub direct_batch_fallback_rows: u64,
    /// Number of times direct materialization reused an existing encoded binding.
    pub direct_binding_reuses: u64,
    /// Number of encoded projection rows observed before dedup.
    pub encoded_project_rows_seen: u64,
    /// Number of encoded projection rows inserted after dedup.
    pub encoded_project_rows_inserted: u64,
    /// Number of duplicate encoded projection rows observed.
    pub encoded_project_duplicate_rows: u64,
    /// Number of encoded row bytes observed by projection sink.
    pub encoded_project_row_bytes: u64,
    /// Number of projection values decoded at output boundary.
    pub project_decode_values: u64,
    /// Number of static-empty proof atoms checked.
    pub static_empty_atoms_checked: u64,
    /// Number of relation/index rows inspected by static-empty proof.
    pub static_empty_rows_scanned: u64,
    /// Number of static-empty proof cache hits.
    pub static_empty_cache_hits: u64,
    /// Number of static-empty proof cache misses.
    pub static_empty_cache_misses: u64,
    /// Number of static semijoin index prefixes probed.
    pub static_semijoin_prefixes_probed: u64,
    /// Number of static semijoin candidate values retained.
    pub static_semijoin_candidate_values: u64,
    /// Number of static semijoin propagation rounds completed.
    pub static_semijoin_rounds: u64,
    /// Number of static semijoin proof attempts skipped by preflight.
    pub static_semijoin_skipped: u64,
    /// Last reason static semijoin proof was skipped.
    pub static_semijoin_skipped_reason: StaticSemijoinSkipReason,
    /// Number of prepared result cache hits.
    pub prepared_result_cache_hits: u64,
    /// Number of prepared result cache misses.
    pub prepared_result_cache_misses: u64,
    /// Number of prepared result cache inserts.
    pub prepared_result_cache_inserts: u64,
    /// Number of prepared result cache bypasses.
    pub prepared_result_cache_bypasses: u64,
}

#[derive(Clone, Debug)]
struct EncodedBinding {
    values: SmallVec<[Option<EncodedOwned>; 8]>,
}

impl EncodedBinding {
    fn new(variable_count: usize) -> Self {
        Self {
            values: std::iter::repeat_with(|| None)
                .take(variable_count)
                .collect(),
        }
    }

    fn get(&self, variable: usize) -> Option<&EncodedOwned> {
        self.values[variable].as_ref()
    }

    fn bind(&mut self, variable: usize, value: EncodedOwned) -> bool {
        match &self.values[variable] {
            Some(existing) => existing == &value,
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
pub(crate) struct ExecutionPlan {
    variable_order_ids: Vec<usize>,
    relation_atoms: Vec<NormAtom>,
    comparisons: Vec<NormPredicate>,
    direct_kernel: Option<DirectKernelPlan>,
    summary: QueryPlan,
}

impl ExecutionPlan {
    fn instantiate(
        &self,
        query_image_cache: QueryImageCacheDiagnostics,
        planner_stats: PlannerStatsCacheDiagnostics,
        prepared_plan_cache: PreparedPlanCacheDiagnostics,
    ) -> Self {
        let mut plan = self.clone();
        plan.summary.query_image_cache = query_image_cache;
        plan.summary.planner_stats = planner_stats;
        plan.summary.prepared_plan_cache = prepared_plan_cache;
        plan.summary.runtime_kind = QueryRuntimeKind::Unknown;
        plan.summary.timings = QueryTimings::default();
        plan.summary.allocations = QueryAllocationStats::default();
        plan.summary.counters = PlanCounters::default();
        for rows in &mut plan.summary.node_rows {
            rows.actual_rows = 0;
        }
        plan.summary.node_timings =
            query_node_timings(&plan.summary.free_join, &plan.summary.node_rows);
        plan
    }
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
                .relation_by_id(atom.relation)
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
    source: LftjAtomSource,
    row_count: usize,
}

enum LftjAtomSource {
    SortedTrie(Arc<SortedTrieIndex>),
}

impl LftjAtomSource {
    fn iter(&self) -> LftjTrieIter<'_> {
        match self {
            LftjAtomSource::SortedTrie(index) => LftjTrieIter::Sorted(index.iter()),
        }
    }
}

enum LftjTrieIter<'a> {
    Sorted(crate::SortedTrieIter<'a>),
}

impl LinearIter for LftjTrieIter<'_> {
    fn key(&self) -> Option<crate::EncodedRef<'_>> {
        match self {
            LftjTrieIter::Sorted(iter) => iter.key(),
        }
    }

    fn next(&mut self) {
        match self {
            LftjTrieIter::Sorted(iter) => iter.next(),
        }
    }

    fn seek(&mut self, target: crate::EncodedRef<'_>) {
        match self {
            LftjTrieIter::Sorted(iter) => iter.seek(target),
        }
    }

    fn at_end(&self) -> bool {
        match self {
            LftjTrieIter::Sorted(iter) => iter.at_end(),
        }
    }
}

impl TrieIter for LftjTrieIter<'_> {
    fn open(&mut self) {
        match self {
            LftjTrieIter::Sorted(iter) => iter.open(),
        }
    }

    fn up(&mut self) {
        match self {
            LftjTrieIter::Sorted(iter) => iter.up(),
        }
    }

    fn depth(&self) -> usize {
        match self {
            LftjTrieIter::Sorted(iter) => iter.depth(),
        }
    }

    fn current_range(&self) -> RowRange {
        match self {
            LftjTrieIter::Sorted(iter) => iter.current_range(),
        }
    }

    fn count(&self) -> usize {
        match self {
            LftjTrieIter::Sorted(iter) => iter.count(),
        }
    }
}

struct LftjRuntime<'a> {
    participants_by_variable: Vec<SmallParticipants>,
    iters: Vec<LftjTrieIter<'a>>,
}

#[derive(Clone, Debug)]
struct DirectKernelPlan {
    kind: DirectKernel,
    summary: DirectKernelSummary,
}

#[derive(Clone, Debug)]
enum DirectKernel {
    PrefixRange(DirectPrefixRangePlan),
    ChainProbe(DirectChainProbePlan),
}

#[derive(Clone, Debug)]
struct DirectPrefixRangePlan {
    atom_id: usize,
    relation: crate::RelationId,
    prefix_fields: SmallVec<[FieldId; 4]>,
    prefix_terms: SmallVec<[NormTerm; 4]>,
    index_name: String,
}

#[derive(Clone, Debug)]
struct DirectChainProbePlan {
    existence_checks: Vec<DirectExistenceCheck>,
    steps: Vec<DirectChainStep>,
}

#[derive(Clone, Debug)]
struct DirectExistenceCheck {
    atom_id: usize,
    relation: crate::RelationId,
    fields: SmallVec<[FieldId; 4]>,
    terms: SmallVec<[NormTerm; 4]>,
    index_name: String,
}

#[derive(Clone, Debug)]
struct DirectChainStep {
    atom_id: usize,
    relation: crate::RelationId,
    prefix_fields: SmallVec<[FieldId; 4]>,
    prefix_terms: SmallVec<[NormTerm; 4]>,
    bind_var: usize,
    bind_field: FieldId,
    index_name: String,
}

struct DirectImageRow {
    fields: SmallVec<[(FieldId, EncodedOwned); 8]>,
}

impl DirectImageRow {
    fn get(&self, field: FieldId) -> Option<&EncodedOwned> {
        self.fields
            .iter()
            .find(|(candidate, _)| *candidate == field)
            .map(|(_, value)| value)
    }
}

type SmallParticipants = SmallVec<[usize; 4]>;
type SmallEncodedPrefix = SmallVec<[EncodedOwned; 8]>;
type SmallEncodedRow = SmallVec<[EncodedOwned; 8]>;
type DirectRangeBounds = (usize, Option<Value>, Option<Value>);
enum DirectStorageAccess {
    Prefix {
        index_name: String,
        values: Vec<(String, Value)>,
    },
    Range {
        index_name: String,
        start: Option<Value>,
        end: Option<Value>,
    },
}

impl<'env> ReadTxn<'env> {
    /// Executes a typed positive query IR against current indexes.
    #[tracing::instrument(name = "bumbledb.query.execute", skip_all, fields(vars = query.variables.len(), clauses = query.clauses.len(), inputs = query.inputs.len()))]
    pub fn execute_query(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
    ) -> Result<QueryOutput> {
        self.execute_query_with_options(schema, query, inputs, QueryExecutionOptions::default())
    }

    /// Executes a typed positive query IR with explicit cache controls.
    #[tracing::instrument(name = "bumbledb.query.execute_with_options", skip_all, fields(vars = query.variables.len(), clauses = query.clauses.len(), inputs = query.inputs.len()))]
    pub fn execute_query_with_options(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
        options: QueryExecutionOptions,
    ) -> Result<QueryOutput> {
        let total_start = Instant::now();
        let total_alloc_start = allocation::snapshot();
        let mut timings = QueryTimings::default();
        let mut allocations = QueryAllocationStats::default();
        let static_empty_fast_key =
            if options.allow_static_empty_fast_cache && query.inputs.is_empty() {
                Some(typed_static_empty_fast_key(
                    schema,
                    self.last_committed_tx_id()?,
                    query,
                ))
            } else {
                None
            };
        if let Some(cache_key) = static_empty_fast_key {
            let lookup_start = Instant::now();
            let cache_hit = self.query_images.static_empty_fast_cached(cache_key)?;
            timings.static_empty_lookup_micros = timings
                .static_empty_lookup_micros
                .saturating_add(elapsed_recorded_micros(lookup_start));
            if cache_hit {
                return Ok(static_empty_output_from_typed(
                    query,
                    self.query_images.diagnostics(),
                    timings,
                    total_start,
                    total_alloc_start,
                    true,
                ));
            }
        }

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        {
            let _span = tracing::debug_span!("bumbledb.query.validate_inputs").entered();
            validate_inputs(schema, query, inputs)?;
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
            encode_inputs(self, schema, &normalized, inputs)?
        };
        timings.encode_inputs_micros = elapsed_micros(phase_start);
        allocations.encode_inputs = allocation_delta_since(phase_alloc_start);

        if let Some(output) = try_execute_direct_storage_project(
            self,
            schema,
            &normalized,
            &encoded_inputs,
            inputs,
            timings,
            allocations,
            total_start,
            total_alloc_start,
        )? {
            return Ok(output);
        }

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let image = {
            let _span = tracing::debug_span!("bumbledb.query.image").entered();
            self.query_images.get_or_build_scoped(
                self,
                schema,
                query_image_scope_for_query(schema, &normalized),
            )?
        };
        timings.query_image_micros = elapsed_micros(phase_start);
        allocations.query_image = allocation_delta_since(phase_alloc_start);

        let query_image_cache = self.query_images.diagnostics();
        let prepared_cache_key = query_shape_key(schema, &normalized);
        if let Some(output) = try_execute_direct_materialized_kernel(
            image.as_ref(),
            self,
            schema,
            &normalized,
            &encoded_inputs,
            query_image_cache,
            image.planner_stats_diagnostics(),
            image.prepared_plan_diagnostics(),
            timings,
            allocations,
            total_start,
            total_alloc_start,
        )? {
            return Ok(output);
        }
        let lookup_start = Instant::now();
        let static_empty_cached = image.static_empty_cached(prepared_cache_key)?;
        timings.static_empty_lookup_micros = timings
            .static_empty_lookup_micros
            .saturating_add(elapsed_recorded_micros(lookup_start));
        if options.allow_static_empty_fast_cache && static_empty_cached {
            let mut plan = static_empty_plan(
                &normalized,
                query_image_cache,
                image.planner_stats_diagnostics(),
                image.prepared_plan_diagnostics(),
            );
            plan.timings = timings;
            plan.allocations = allocations;
            plan.runtime_kind = QueryRuntimeKind::StaticEmpty;
            plan.counters.static_empty_cache_hits = 1;
            finish_timings(&mut plan.timings, total_start);
            let total_alloc = allocation_delta_since(total_alloc_start);
            plan.allocations = plan.allocations.with_total(total_alloc);
            return Ok(QueryOutput {
                result: QueryResultSet::new(
                    result_columns(&normalized),
                    empty_output_rows(&normalized.output),
                ),
                plan,
            });
        }
        let static_empty_proof = static_query_proves_empty_timed(
            image.as_ref(),
            &normalized,
            &encoded_inputs,
            prepared_cache_key,
            options.allow_static_empty_fast_cache,
            &mut timings,
        )?;
        if static_empty_proof.as_ref().is_some_and(|proof| proof.empty) {
            if options.allow_static_empty_fast_cache && normalized.inputs.is_empty() {
                image.insert_static_empty(prepared_cache_key)?;
                if let Some(cache_key) = static_empty_fast_key {
                    self.query_images.insert_static_empty_fast(cache_key)?;
                }
            }
            let mut plan = static_empty_plan(
                &normalized,
                query_image_cache,
                image.planner_stats_diagnostics(),
                image.prepared_plan_diagnostics(),
            );
            plan.timings = timings;
            plan.allocations = allocations;
            plan.runtime_kind = QueryRuntimeKind::StaticEmpty;
            if let Some(proof) = &static_empty_proof {
                record_static_proof_counters(&mut plan.counters, proof);
            }
            finish_timings(&mut plan.timings, total_start);
            let total_alloc = allocation_delta_since(total_alloc_start);
            plan.allocations = plan.allocations.with_total(total_alloc);
            return Ok(QueryOutput {
                result: QueryResultSet::new(
                    result_columns(&normalized),
                    empty_output_rows(&normalized.output),
                ),
                plan,
            });
        }

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let mut plan = if let Some(cached) = image.cached_prepared_plan(prepared_cache_key)? {
            cached.instantiate(
                query_image_cache,
                image.planner_stats_diagnostics(),
                image.prepared_plan_diagnostics(),
            )
        } else {
            let prepared_plan_cache = image.prepared_plan_diagnostics();
            let planned = plan_query(
                schema,
                &mut normalized,
                image.as_ref(),
                query_image_cache,
                prepared_plan_cache,
            )?;
            let build_micros = elapsed_micros(phase_start).min(u128::from(u64::MAX)) as u64;
            let cached = image.insert_prepared_plan(prepared_cache_key, planned, build_micros)?;
            cached.instantiate(
                query_image_cache,
                image.planner_stats_diagnostics(),
                image.prepared_plan_diagnostics(),
            )
        };
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
        if let Some(proof) = &static_empty_proof {
            record_static_proof_counters(&mut plan.summary.counters, proof);
        }
        finish_timings(&mut plan.summary.timings, total_start);
        let total_alloc = allocation_delta_since(total_alloc_start);
        plan.summary.allocations = plan.summary.allocations.with_total(total_alloc);
        plan.summary.refresh_node_timings();
        tracing::debug!(?plan.summary.counters, "free join query executed");
        Ok(QueryOutput {
            result: QueryResultSet::new(columns, rows),
            plan: plan.summary,
        })
    }

    /// Executes a prepared typed positive query IR against current indexes.
    #[tracing::instrument(name = "bumbledb.query.execute_prepared", skip_all, fields(vars = query.query().variables.len(), clauses = query.query().clauses.len(), inputs = query.query().inputs.len()))]
    pub fn execute_prepared_query(
        &self,
        schema: &StorageSchema,
        query: &PreparedQuery,
        inputs: &InputBindings,
    ) -> Result<QueryOutput> {
        self.execute_prepared_query_with_options(
            schema,
            query,
            inputs,
            QueryExecutionOptions::default(),
        )
    }

    /// Executes a prepared typed positive query IR with explicit cache controls.
    #[tracing::instrument(name = "bumbledb.query.execute_prepared_with_options", skip_all, fields(vars = query.query().variables.len(), clauses = query.query().clauses.len(), inputs = query.query().inputs.len()))]
    pub fn execute_prepared_query_with_options(
        &self,
        schema: &StorageSchema,
        query: &PreparedQuery,
        inputs: &InputBindings,
        options: QueryExecutionOptions,
    ) -> Result<QueryOutput> {
        let typed = query.query();
        let total_start = Instant::now();
        let total_alloc_start = allocation::snapshot();
        let mut timings = QueryTimings::default();
        let mut allocations = QueryAllocationStats::default();
        let static_empty_fast_key =
            if options.allow_static_empty_fast_cache && typed.inputs.is_empty() {
                Some(typed_static_empty_fast_key(
                    schema,
                    self.last_committed_tx_id()?,
                    typed,
                ))
            } else {
                None
            };
        if let Some(cache_key) = static_empty_fast_key {
            let lookup_start = Instant::now();
            let cache_hit = self.query_images.static_empty_fast_cached(cache_key)?;
            timings.static_empty_lookup_micros = timings
                .static_empty_lookup_micros
                .saturating_add(elapsed_recorded_micros(lookup_start));
            if cache_hit {
                return Ok(static_empty_output_from_typed(
                    typed,
                    self.query_images.diagnostics(),
                    timings,
                    total_start,
                    total_alloc_start,
                    true,
                ));
            }
        }

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        {
            let _span = tracing::debug_span!("bumbledb.query.validate_inputs").entered();
            validate_inputs(schema, typed, inputs)?;
        }
        timings.validate_inputs_micros = elapsed_micros(phase_start);
        allocations.validate_inputs = allocation_delta_since(phase_alloc_start);

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let (normalized, normalized_built) = {
            let _span = tracing::debug_span!(
                "bumbledb.query.normalize",
                vars = typed.variables.len(),
                clauses = typed.clauses.len()
            )
            .entered();
            query.normalized_for(self, schema)?
        };
        if normalized_built {
            timings.normalize_micros = elapsed_micros(phase_start);
            allocations.normalize = allocation_delta_since(phase_alloc_start);
        }
        let normalized = normalized.as_ref();

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let encoded_inputs = {
            let _span = tracing::debug_span!(
                "bumbledb.query.encode_inputs",
                inputs = normalized.inputs.len()
            )
            .entered();
            encode_inputs(self, schema, normalized, inputs)?
        };
        timings.encode_inputs_micros = elapsed_micros(phase_start);
        allocations.encode_inputs = allocation_delta_since(phase_alloc_start);

        if let Some(output) = try_execute_direct_storage_project(
            self,
            schema,
            normalized,
            &encoded_inputs,
            inputs,
            timings,
            allocations,
            total_start,
            total_alloc_start,
        )? {
            return Ok(output);
        }

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let image = {
            let _span = tracing::debug_span!("bumbledb.query.image").entered();
            self.query_images.get_or_build_scoped(
                self,
                schema,
                query_image_scope_for_query(schema, normalized),
            )?
        };
        timings.query_image_micros = elapsed_micros(phase_start);
        allocations.query_image = allocation_delta_since(phase_alloc_start);

        let query_image_cache = self.query_images.diagnostics();
        if let Some(output) = try_execute_direct_materialized_kernel(
            image.as_ref(),
            self,
            schema,
            normalized,
            &encoded_inputs,
            query_image_cache,
            image.planner_stats_diagnostics(),
            image.prepared_plan_diagnostics(),
            timings,
            allocations,
            total_start,
            total_alloc_start,
        )? {
            return Ok(output);
        }
        let prepared_result_cache_missed = false;
        let prepared_result_cache_bypassed = !options.allow_prepared_result_cache;
        let prepared_cache_key = query_shape_key(schema, normalized);
        let lookup_start = Instant::now();
        let static_empty_cached = image.static_empty_cached(prepared_cache_key)?;
        timings.static_empty_lookup_micros = timings
            .static_empty_lookup_micros
            .saturating_add(elapsed_recorded_micros(lookup_start));
        if options.allow_static_empty_fast_cache && static_empty_cached {
            let mut plan = static_empty_plan(
                normalized,
                query_image_cache,
                image.planner_stats_diagnostics(),
                image.prepared_plan_diagnostics(),
            );
            plan.timings = timings;
            plan.allocations = allocations;
            plan.runtime_kind = QueryRuntimeKind::StaticEmpty;
            plan.counters.static_empty_cache_hits = 1;
            record_prepared_result_cache_counters(
                &mut plan.counters,
                prepared_result_cache_missed,
                prepared_result_cache_bypassed,
                false,
            );
            finish_timings(&mut plan.timings, total_start);
            let total_alloc = allocation_delta_since(total_alloc_start);
            plan.allocations = plan.allocations.with_total(total_alloc);
            return Ok(QueryOutput {
                result: QueryResultSet::new(
                    result_columns(normalized),
                    empty_output_rows(&normalized.output),
                ),
                plan,
            });
        }
        let static_empty_proof = static_query_proves_empty_timed(
            image.as_ref(),
            normalized,
            &encoded_inputs,
            prepared_cache_key,
            options.allow_static_empty_fast_cache,
            &mut timings,
        )?;
        if static_empty_proof.as_ref().is_some_and(|proof| proof.empty) {
            if options.allow_static_empty_fast_cache && normalized.inputs.is_empty() {
                image.insert_static_empty(prepared_cache_key)?;
                if let Some(cache_key) = static_empty_fast_key {
                    self.query_images.insert_static_empty_fast(cache_key)?;
                }
            }
            let mut plan = static_empty_plan(
                normalized,
                query_image_cache,
                image.planner_stats_diagnostics(),
                image.prepared_plan_diagnostics(),
            );
            plan.timings = timings;
            plan.allocations = allocations;
            plan.runtime_kind = QueryRuntimeKind::StaticEmpty;
            if let Some(proof) = &static_empty_proof {
                record_static_proof_counters(&mut plan.counters, proof);
            }
            record_prepared_result_cache_counters(
                &mut plan.counters,
                prepared_result_cache_missed,
                prepared_result_cache_bypassed,
                false,
            );
            finish_timings(&mut plan.timings, total_start);
            let total_alloc = allocation_delta_since(total_alloc_start);
            plan.allocations = plan.allocations.with_total(total_alloc);
            return Ok(QueryOutput {
                result: QueryResultSet::new(
                    result_columns(normalized),
                    empty_output_rows(&normalized.output),
                ),
                plan,
            });
        }

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let mut plan = if let Some(cached) = image.cached_prepared_plan(prepared_cache_key)? {
            cached.instantiate(
                query_image_cache,
                image.planner_stats_diagnostics(),
                image.prepared_plan_diagnostics(),
            )
        } else {
            let prepared_plan_cache = image.prepared_plan_diagnostics();
            let mut planned_normalized = (*normalized).clone();
            let planned = plan_query(
                schema,
                &mut planned_normalized,
                image.as_ref(),
                query_image_cache,
                prepared_plan_cache,
            )?;
            let build_micros = elapsed_micros(phase_start).min(u128::from(u64::MAX)) as u64;
            let cached = image.insert_prepared_plan(prepared_cache_key, planned, build_micros)?;
            cached.instantiate(
                query_image_cache,
                image.planner_stats_diagnostics(),
                image.prepared_plan_diagnostics(),
            )
        };
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
            normalized,
            &encoded_inputs,
            &mut plan,
            &mut sink,
        )?;
        plan.summary.timings.execute_micros = elapsed_micros(execute_start);
        plan.summary.allocations.execute = allocation_delta_since(execute_alloc_start);

        let columns = result_columns(normalized);
        let sink_finish_start = Instant::now();
        let sink_finish_alloc_start = allocation::snapshot();
        let rows = {
            let _span = tracing::debug_span!("bumbledb.query.sink.finish").entered();
            sink.finish(self, normalized, &mut plan.summary.counters)?
        };
        plan.summary.timings.sink_finish_micros = elapsed_micros(sink_finish_start);
        plan.summary.allocations.sink_finish = allocation_delta_since(sink_finish_alloc_start);
        plan.summary.counters.output_rows = rows.len() as u64;
        if has_aggregate(normalized) {
            plan.summary.counters.aggregate_groups = rows.len() as u64;
        }
        if let Some(proof) = &static_empty_proof {
            record_static_proof_counters(&mut plan.summary.counters, proof);
        }
        record_prepared_result_cache_counters(
            &mut plan.summary.counters,
            prepared_result_cache_missed,
            prepared_result_cache_bypassed,
            false,
        );
        finish_timings(&mut plan.summary.timings, total_start);
        let total_alloc = allocation_delta_since(total_alloc_start);
        plan.summary.allocations = plan.summary.allocations.with_total(total_alloc);
        plan.summary.refresh_node_timings();
        tracing::debug!(?plan.summary.counters, "free join query executed");
        Ok(QueryOutput {
            result: QueryResultSet::new(columns, rows),
            plan: plan.summary,
        })
    }

    /// Executes a prepared typed query and returns only the output row count.
    #[tracing::instrument(name = "bumbledb.query.execute_prepared_cardinality", skip_all, fields(vars = query.query().variables.len(), clauses = query.query().clauses.len(), inputs = query.query().inputs.len()))]
    pub fn execute_prepared_result_cardinality(
        &self,
        schema: &StorageSchema,
        query: &PreparedQuery,
        inputs: &InputBindings,
    ) -> Result<QueryResultCardinality> {
        self.execute_prepared_result_cardinality_with_options(
            schema,
            query,
            inputs,
            QueryExecutionOptions::default(),
        )
    }

    /// Executes a prepared typed query count-only with explicit cache controls.
    #[tracing::instrument(name = "bumbledb.query.execute_prepared_cardinality_with_options", skip_all, fields(vars = query.query().variables.len(), clauses = query.query().clauses.len(), inputs = query.query().inputs.len()))]
    pub fn execute_prepared_result_cardinality_with_options(
        &self,
        schema: &StorageSchema,
        query: &PreparedQuery,
        inputs: &InputBindings,
        options: QueryExecutionOptions,
    ) -> Result<QueryResultCardinality> {
        self.execute_result_cardinality_with_options(schema, query.query(), inputs, options)
    }

    /// Executes a typed query and returns only the output row count.
    #[tracing::instrument(name = "bumbledb.query.execute_count", skip_all, fields(vars = query.variables.len(), clauses = query.clauses.len(), inputs = query.inputs.len()))]
    pub fn execute_result_cardinality(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
    ) -> Result<QueryResultCardinality> {
        self.execute_result_cardinality_with_options(
            schema,
            query,
            inputs,
            QueryExecutionOptions::default(),
        )
    }

    /// Executes a typed query count-only with explicit cache controls.
    #[tracing::instrument(name = "bumbledb.query.execute_count_with_options", skip_all, fields(vars = query.variables.len(), clauses = query.clauses.len(), inputs = query.inputs.len()))]
    pub fn execute_result_cardinality_with_options(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
        options: QueryExecutionOptions,
    ) -> Result<QueryResultCardinality> {
        let total_start = Instant::now();
        let total_alloc_start = allocation::snapshot();
        let mut timings = QueryTimings::default();
        let mut allocations = QueryAllocationStats::default();
        let static_empty_fast_key =
            if options.allow_static_empty_fast_cache && query.inputs.is_empty() {
                Some(typed_static_empty_fast_key(
                    schema,
                    self.last_committed_tx_id()?,
                    query,
                ))
            } else {
                None
            };
        if let Some(cache_key) = static_empty_fast_key {
            let lookup_start = Instant::now();
            let cache_hit = self.query_images.static_empty_fast_cached(cache_key)?;
            timings.static_empty_lookup_micros = timings
                .static_empty_lookup_micros
                .saturating_add(elapsed_recorded_micros(lookup_start));
            if cache_hit {
                return Ok(QueryResultCardinality {
                    cardinality: 0,
                    plan: static_empty_plan_from_typed(
                        query,
                        self.query_images.diagnostics(),
                        timings,
                        total_start,
                        total_alloc_start,
                        true,
                    ),
                });
            }
        }

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        validate_inputs(schema, query, inputs)?;
        timings.validate_inputs_micros = elapsed_micros(phase_start);
        allocations.validate_inputs = allocation_delta_since(phase_alloc_start);

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let mut normalized = normalize_query(self, schema, query)?;
        timings.normalize_micros = elapsed_micros(phase_start);
        allocations.normalize = allocation_delta_since(phase_alloc_start);

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let encoded_inputs = encode_inputs(self, schema, &normalized, inputs)?;
        timings.encode_inputs_micros = elapsed_micros(phase_start);
        allocations.encode_inputs = allocation_delta_since(phase_alloc_start);

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let image = self.query_images.get_or_build_scoped(
            self,
            schema,
            query_image_scope_for_query(schema, &normalized),
        )?;
        timings.query_image_micros = elapsed_micros(phase_start);
        allocations.query_image = allocation_delta_since(phase_alloc_start);

        let query_image_cache = self.query_images.diagnostics();
        let prepared_cache_key = query_shape_key(schema, &normalized);
        let lookup_start = Instant::now();
        let static_empty_cached = image.static_empty_cached(prepared_cache_key)?;
        timings.static_empty_lookup_micros = timings
            .static_empty_lookup_micros
            .saturating_add(elapsed_recorded_micros(lookup_start));
        if options.allow_static_empty_fast_cache && static_empty_cached {
            let mut plan = static_empty_plan(
                &normalized,
                query_image_cache,
                image.planner_stats_diagnostics(),
                image.prepared_plan_diagnostics(),
            );
            plan.timings = timings;
            plan.allocations = allocations;
            plan.runtime_kind = QueryRuntimeKind::StaticEmpty;
            plan.counters.static_empty_cache_hits = 1;
            finish_timings(&mut plan.timings, total_start);
            plan.allocations = plan
                .allocations
                .with_total(allocation_delta_since(total_alloc_start));
            return Ok(QueryResultCardinality {
                cardinality: 0,
                plan,
            });
        }

        let static_empty_proof = static_query_proves_empty_timed(
            image.as_ref(),
            &normalized,
            &encoded_inputs,
            prepared_cache_key,
            options.allow_static_empty_fast_cache,
            &mut timings,
        )?;
        if static_empty_proof.as_ref().is_some_and(|proof| proof.empty) {
            if options.allow_static_empty_fast_cache && normalized.inputs.is_empty() {
                image.insert_static_empty(prepared_cache_key)?;
                if let Some(cache_key) = static_empty_fast_key {
                    self.query_images.insert_static_empty_fast(cache_key)?;
                }
            }
            let mut plan = static_empty_plan(
                &normalized,
                query_image_cache,
                image.planner_stats_diagnostics(),
                image.prepared_plan_diagnostics(),
            );
            plan.timings = timings;
            plan.allocations = allocations;
            plan.runtime_kind = QueryRuntimeKind::StaticEmpty;
            if let Some(proof) = &static_empty_proof {
                record_static_proof_counters(&mut plan.counters, proof);
            }
            finish_timings(&mut plan.timings, total_start);
            plan.allocations = plan
                .allocations
                .with_total(allocation_delta_since(total_alloc_start));
            return Ok(QueryResultCardinality {
                cardinality: 0,
                plan,
            });
        }

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let mut plan = if let Some(cached) = image.cached_prepared_plan(prepared_cache_key)? {
            cached.instantiate(
                query_image_cache,
                image.planner_stats_diagnostics(),
                image.prepared_plan_diagnostics(),
            )
        } else {
            let prepared_plan_cache = image.prepared_plan_diagnostics();
            let planned = plan_query(
                schema,
                &mut normalized,
                image.as_ref(),
                query_image_cache,
                prepared_plan_cache,
            )?;
            let build_micros = elapsed_micros(phase_start).min(u128::from(u64::MAX)) as u64;
            let cached = image.insert_prepared_plan(prepared_cache_key, planned, build_micros)?;
            cached.instantiate(
                query_image_cache,
                image.planner_stats_diagnostics(),
                image.prepared_plan_diagnostics(),
            )
        };
        timings.plan_micros = elapsed_micros(phase_start);
        allocations.plan = allocation_delta_since(phase_alloc_start);
        plan.summary.timings = timings;
        plan.summary.allocations = allocations;

        let mut sink = OutputSink::new_count_rows(&plan.summary.free_join.output);
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

        let rows = sink.finish_count()?;
        plan.summary.counters.output_rows = rows as u64;
        if has_aggregate(&normalized) {
            plan.summary.counters.aggregate_groups = rows as u64;
        }
        if let Some(proof) = &static_empty_proof {
            record_static_proof_counters(&mut plan.summary.counters, proof);
        }
        finish_timings(&mut plan.summary.timings, total_start);
        plan.summary.allocations = plan
            .summary
            .allocations
            .with_total(allocation_delta_since(total_alloc_start));
        plan.summary.refresh_node_timings();
        Ok(QueryResultCardinality {
            cardinality: rows,
            plan: plan.summary,
        })
    }
}

fn elapsed_micros(start: Instant) -> u128 {
    start.elapsed().as_micros()
}

fn elapsed_recorded_micros(start: Instant) -> u128 {
    start.elapsed().as_micros().max(1)
}

fn finish_timings(timings: &mut QueryTimings, total_start: Instant) {
    timings.total_micros = elapsed_micros(total_start);
    timings.refresh_unaccounted();
}

fn record_static_proof_counters(counters: &mut PlanCounters, proof: &StaticEmptyProof) {
    counters.static_empty_cache_misses = 1;
    counters.static_empty_atoms_checked = proof.atoms_checked;
    counters.static_empty_rows_scanned = proof.rows_scanned;
    counters.static_semijoin_prefixes_probed = proof.prefixes_probed;
    counters.static_semijoin_candidate_values = proof.candidate_values;
    counters.static_semijoin_rounds = proof.rounds;
    if proof.semijoin_skipped {
        counters.static_semijoin_skipped = 1;
        counters.static_semijoin_skipped_reason = proof.semijoin_skipped_reason;
    }
}

fn record_prepared_result_cache_counters(
    counters: &mut PlanCounters,
    missed: bool,
    bypassed: bool,
    inserted: bool,
) {
    counters.prepared_result_cache_misses += u64::from(missed);
    counters.prepared_result_cache_bypasses += u64::from(bypassed);
    counters.prepared_result_cache_inserts += u64::from(inserted);
}

fn allocation_delta_since(start: allocation::AllocationSnapshot) -> AllocationPhaseStats {
    allocation::delta(start, allocation::snapshot()).into()
}

fn query_shape_key(schema: &StorageSchema, query: &NormalizedQuery) -> QueryShapeKey {
    let mut hasher = blake3::Hasher::new();
    hash_bytes_len_prefixed(&mut hasher, b"bumbledb.query_shape.v1");
    hasher.update(&schema.descriptor().fingerprint().0);
    hash_u64(&mut hasher, query.vars.len() as u64);
    for var in &query.vars {
        hash_u16(&mut hasher, var.id.0);
        hash_bytes_len_prefixed(&mut hasher, var.name.as_bytes());
        hash_value_type(&mut hasher, &var.value_type);
    }
    hash_u64(&mut hasher, query.inputs.len() as u64);
    for input in &query.inputs {
        hash_u16(&mut hasher, input.id.0);
        hash_bytes_len_prefixed(&mut hasher, input.name.as_bytes());
        hash_value_type(&mut hasher, &input.value_type);
    }
    hash_u64(&mut hasher, query.atoms.len() as u64);
    for atom in &query.atoms {
        hash_u16(&mut hasher, atom.id.0);
        hash_u16(&mut hasher, atom.relation.0);
        hash_bytes_len_prefixed(&mut hasher, atom.relation_name.as_bytes());
        hash_u64(&mut hasher, atom.fields.len() as u64);
        for field in &atom.fields {
            hash_u16(&mut hasher, field.field.0);
            hash_bytes_len_prefixed(&mut hasher, field.field_name.as_bytes());
            hash_value_type(&mut hasher, &field.value_type);
            hash_norm_term(&mut hasher, &field.term);
        }
    }
    hash_u64(&mut hasher, query.predicates.len() as u64);
    for predicate in &query.predicates {
        hash_u16(&mut hasher, predicate.id.0);
        hash_comparison_operator(&mut hasher, predicate.op);
        hash_value_type(&mut hasher, &predicate.value_type);
        for operand in &predicate.operands {
            hash_norm_operand(&mut hasher, operand);
        }
    }
    hash_u64(&mut hasher, query.find.len() as u64);
    for term in &query.find {
        hash_find_term(&mut hasher, term);
    }
    hash_output_plan(&mut hasher, &query.output);
    QueryShapeKey(*hasher.finalize().as_bytes())
}

fn static_proof_cache_key(
    image: &QueryImageKey,
    query_shape: QueryShapeKey,
    inputs: &EncodedInputs,
    kind: StaticProofKind,
) -> StaticProofCacheKey {
    let mut hasher = blake3::Hasher::new();
    hash_bytes_len_prefixed(&mut hasher, b"bumbledb.static_proof_cache.v1");
    hasher.update(&image.schema.0);
    hash_u64(&mut hasher, image.tx_id);
    hasher.update(&query_shape.0);
    hash_u8(
        &mut hasher,
        match kind {
            StaticProofKind::StaticLiteral => 1,
            StaticProofKind::StaticSemijoin => 2,
        },
    );
    hash_u64(&mut hasher, inputs.values.len() as u64);
    for value in &inputs.values {
        hash_encoded_owned(&mut hasher, value);
    }
    StaticProofCacheKey(*hasher.finalize().as_bytes())
}

fn query_image_scope_for_query(schema: &StorageSchema, query: &NormalizedQuery) -> QueryImageScope {
    QueryImageScope::relations_all(schema, query.atoms.iter().map(|atom| atom.relation))
}

fn typed_static_empty_fast_key(
    schema: &StorageSchema,
    tx_id: u64,
    query: &TypedQuery,
) -> QueryShapeKey {
    let mut hasher = blake3::Hasher::new();
    hash_bytes_len_prefixed(&mut hasher, b"bumbledb.static_empty_typed.v1");
    hasher.update(&schema.descriptor().fingerprint().0);
    hash_u64(&mut hasher, tx_id);
    hash_typed_query(&mut hasher, query);
    QueryShapeKey(*hasher.finalize().as_bytes())
}

fn hash_typed_query(hasher: &mut blake3::Hasher, query: &TypedQuery) {
    hash_u64(hasher, query.variables.len() as u64);
    for variable in &query.variables {
        hash_u64(hasher, variable.id as u64);
        hash_bytes_len_prefixed(hasher, variable.name.as_bytes());
        hash_value_type(hasher, &variable.value_type);
    }
    hash_u64(hasher, query.inputs.len() as u64);
    for input in &query.inputs {
        hash_u64(hasher, input.id as u64);
        hash_bytes_len_prefixed(hasher, input.name.as_bytes());
        hash_value_type(hasher, &input.value_type);
    }
    hash_u64(hasher, query.find.len() as u64);
    for term in &query.find {
        hash_typed_find_term(hasher, term);
    }
    hash_u64(hasher, query.clauses.len() as u64);
    for clause in &query.clauses {
        match clause {
            TypedClause::Relation(atom) => {
                hash_u8(hasher, 1);
                hash_u64(hasher, atom.relation_id as u64);
                hash_bytes_len_prefixed(hasher, atom.relation.as_bytes());
                hash_u64(hasher, atom.fields.len() as u64);
                for field in &atom.fields {
                    hash_u64(hasher, field.field_id as u64);
                    hash_bytes_len_prefixed(hasher, field.field.as_bytes());
                    hash_value_type(hasher, &field.value_type);
                    hash_typed_term(hasher, &field.term);
                }
            }
            TypedClause::Comparison(comparison) => {
                hash_u8(hasher, 2);
                hash_typed_operand(hasher, &comparison.left);
                hash_comparison_operator(hasher, comparison.operator);
                hash_typed_operand(hasher, &comparison.right);
                hash_value_type(hasher, &comparison.value_type);
            }
        }
    }
}

fn hash_typed_find_term(hasher: &mut blake3::Hasher, term: &TypedFindTerm) {
    match term {
        TypedFindTerm::Variable { variable } => {
            hash_u8(hasher, 1);
            hash_u64(hasher, *variable as u64);
        }
        TypedFindTerm::Aggregate {
            function,
            variable,
            domain,
            value_type,
        } => {
            hash_u8(hasher, 2);
            hash_aggregate_function(hasher, *function);
            hash_u64(hasher, *variable as u64);
            hash_u64(hasher, domain.len() as u64);
            for variable in domain {
                hash_u64(hasher, *variable as u64);
            }
            hash_value_type(hasher, value_type);
        }
    }
}

fn hash_typed_term(hasher: &mut blake3::Hasher, term: &TypedTerm) {
    match term {
        TypedTerm::Variable(variable) => {
            hash_u8(hasher, 1);
            hash_u64(hasher, *variable as u64);
        }
        TypedTerm::Input(input) => {
            hash_u8(hasher, 2);
            hash_u64(hasher, *input as u64);
        }
        TypedTerm::Wildcard => hash_u8(hasher, 3),
        TypedTerm::Literal(literal) => {
            hash_u8(hasher, 4);
            hash_typed_literal(hasher, literal);
        }
    }
}

fn hash_typed_operand(hasher: &mut blake3::Hasher, operand: &TypedOperand) {
    match operand {
        TypedOperand::Variable(variable) => {
            hash_u8(hasher, 1);
            hash_u64(hasher, *variable as u64);
        }
        TypedOperand::Input(input) => {
            hash_u8(hasher, 2);
            hash_u64(hasher, *input as u64);
        }
        TypedOperand::Literal(literal) => {
            hash_u8(hasher, 3);
            hash_typed_literal(hasher, literal);
        }
    }
}

fn hash_typed_literal(hasher: &mut blake3::Hasher, literal: &TypedLiteral) {
    hash_value_type(hasher, &literal.value_type);
    match &literal.literal {
        Literal::Bool(value) => {
            hash_u8(hasher, 1);
            hash_u8(hasher, u8::from(*value));
        }
        Literal::Integer(value) => {
            hash_u8(hasher, 2);
            hasher.update(&value.to_be_bytes());
        }
        Literal::String(value) => {
            hash_u8(hasher, 3);
            hash_bytes_len_prefixed(hasher, value.as_bytes());
        }
    }
}

fn hash_u8(hasher: &mut blake3::Hasher, value: u8) {
    hasher.update(&[value]);
}

fn hash_u16(hasher: &mut blake3::Hasher, value: u16) {
    hasher.update(&value.to_be_bytes());
}

fn hash_u32(hasher: &mut blake3::Hasher, value: u32) {
    hasher.update(&value.to_be_bytes());
}

fn hash_u64(hasher: &mut blake3::Hasher, value: u64) {
    hasher.update(&value.to_be_bytes());
}

fn hash_bytes_len_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hash_u64(hasher, bytes.len() as u64);
    hasher.update(bytes);
}

fn hash_value_type(hasher: &mut blake3::Hasher, value_type: &ValueType) {
    match value_type {
        ValueType::Bool => hash_u8(hasher, 1),
        ValueType::U64 => hash_u8(hasher, 2),
        ValueType::I64 => hash_u8(hasher, 3),
        ValueType::TimestampMicros => hash_u8(hasher, 4),
        ValueType::Decimal { scale } => {
            hash_u8(hasher, 5);
            hash_u32(hasher, *scale);
        }
        ValueType::Enum { name } => {
            hash_u8(hasher, 7);
            hash_bytes_len_prefixed(hasher, name.as_bytes());
        }
        ValueType::String => hash_u8(hasher, 8),
        ValueType::Bytes => hash_u8(hasher, 9),
        ValueType::Serial {
            type_name,
            owning_relation,
        } => {
            hash_u8(hasher, 10);
            hash_bytes_len_prefixed(hasher, type_name.as_bytes());
            hash_bytes_len_prefixed(hasher, owning_relation.as_bytes());
        }
    }
}

fn hash_encoded_owned(hasher: &mut blake3::Hasher, value: &EncodedOwned) {
    match value {
        EncodedOwned::One(bytes) => {
            hash_u8(hasher, 1);
            hash_bytes_len_prefixed(hasher, bytes);
        }
        EncodedOwned::Eight(bytes) => {
            hash_u8(hasher, 8);
            hash_bytes_len_prefixed(hasher, bytes);
        }
        EncodedOwned::Sixteen(bytes) => {
            hash_u8(hasher, 16);
            hash_bytes_len_prefixed(hasher, bytes);
        }
    }
}

fn hash_norm_term(hasher: &mut blake3::Hasher, term: &NormTerm) {
    match term {
        NormTerm::Var(variable) => {
            hash_u8(hasher, 1);
            hash_u16(hasher, variable.0);
        }
        NormTerm::Input(input) => {
            hash_u8(hasher, 2);
            hash_u16(hasher, input.0);
        }
        NormTerm::Literal(value) => {
            hash_u8(hasher, 3);
            hash_encoded_owned(hasher, value);
        }
        NormTerm::Wildcard => hash_u8(hasher, 4),
    }
}

fn hash_norm_operand(hasher: &mut blake3::Hasher, operand: &NormOperand) {
    match operand {
        NormOperand::Var(variable) => {
            hash_u8(hasher, 1);
            hash_u16(hasher, variable.0);
        }
        NormOperand::Input(input) => {
            hash_u8(hasher, 2);
            hash_u16(hasher, input.0);
        }
        NormOperand::Literal(value) => {
            hash_u8(hasher, 3);
            hash_encoded_owned(hasher, value);
        }
    }
}

fn hash_comparison_operator(hasher: &mut blake3::Hasher, op: ComparisonOperator) {
    hash_u8(
        hasher,
        match op {
            ComparisonOperator::Eq => 1,
            ComparisonOperator::NotEq => 2,
            ComparisonOperator::Lt => 3,
            ComparisonOperator::Lte => 4,
            ComparisonOperator::Gt => 5,
            ComparisonOperator::Gte => 6,
        },
    );
}

fn hash_aggregate_function(hasher: &mut blake3::Hasher, function: AggregateFunction) {
    hash_u8(
        hasher,
        match function {
            AggregateFunction::CountDomain => 1,
            AggregateFunction::CountDistinct => 2,
            AggregateFunction::Sum => 3,
            AggregateFunction::Min => 4,
            AggregateFunction::Max => 5,
        },
    );
}

fn hash_find_term(hasher: &mut blake3::Hasher, term: &NormFindTerm) {
    match term {
        NormFindTerm::Variable { variable } => {
            hash_u8(hasher, 1);
            hash_u16(hasher, variable.0);
        }
        NormFindTerm::Aggregate {
            function,
            variable,
            domain,
            value_type,
        } => {
            hash_u8(hasher, 2);
            hash_aggregate_function(hasher, *function);
            hash_u16(hasher, variable.0);
            hash_u64(hasher, domain.len() as u64);
            for variable in domain {
                hash_u16(hasher, variable.0);
            }
            hash_value_type(hasher, value_type);
        }
    }
}

fn hash_output_plan(hasher: &mut blake3::Hasher, output: &OutputPlan) {
    match output {
        OutputPlan::Project(project) => {
            hash_u8(hasher, 1);
            hash_u64(hasher, project.vars.len() as u64);
            for variable in &project.vars {
                hash_u16(hasher, variable.0);
            }
        }
        OutputPlan::Aggregate(aggregate) => {
            hash_u8(hasher, 2);
            hash_u64(hasher, aggregate.group_vars.len() as u64);
            for variable in &aggregate.group_vars {
                hash_u16(hasher, variable.0);
            }
            hash_u64(hasher, aggregate.aggregates.len() as u64);
            for term in &aggregate.aggregates {
                hash_aggregate_function(hasher, term.function);
                hash_u16(hasher, term.var.0);
                hash_u64(hasher, term.domain_vars.len() as u64);
                for variable in &term.domain_vars {
                    hash_u16(hasher, variable.0);
                }
                hash_value_type(hasher, &term.value_type);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct StaticEmptyProof {
    empty: bool,
    atoms_checked: u64,
    rows_scanned: u64,
    prefixes_probed: u64,
    candidate_values: u64,
    rounds: u64,
    semijoin_skipped: bool,
    semijoin_skipped_reason: StaticSemijoinSkipReason,
}

#[derive(Clone, Debug)]
struct CandidateSet {
    values: BTreeSet<EncodedOwned>,
}

impl CandidateSet {
    fn new(values: BTreeSet<EncodedOwned>) -> Self {
        Self { values }
    }
}

#[derive(Clone, Debug)]
enum FieldConstraint<'a> {
    Single(&'a EncodedOwned),
    Candidates(&'a BTreeSet<EncodedOwned>),
}

#[derive(Clone, Debug)]
enum StaticSemijoinProbe {
    Prefix(Vec<u8>),
    Range {
        lower: Vec<u8>,
        upper: Option<Vec<u8>>,
        upper_inclusive: bool,
    },
}

#[derive(Clone, Debug)]
struct StaticRangeConstraint {
    lower: Option<(EncodedOwned, bool)>,
    upper: Option<(EncodedOwned, bool)>,
}

type StaticSemijoinProbes<'a> = (&'a RelationIndexImage, Vec<StaticSemijoinProbe>);

// Static proof owns emptiness detection only. It must not count rows or select plans.
fn static_query_proves_empty_timed(
    image: &crate::QueryImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    query_shape: QueryShapeKey,
    allow_static_proof_cache: bool,
    timings: &mut QueryTimings,
) -> Result<Option<StaticEmptyProof>> {
    let image_key = image.key();
    let literal_cache_key = static_proof_cache_key(
        &image_key,
        query_shape,
        inputs,
        StaticProofKind::StaticLiteral,
    );
    let literal_start = Instant::now();
    let mut proof = if allow_static_proof_cache
        && let Some(cached) = image.cached_static_proof(literal_cache_key)?
    {
        match cached {
            StaticProofCacheValue::ProvenEmpty => StaticEmptyProof {
                empty: true,
                ..StaticEmptyProof::default()
            },
            StaticProofCacheValue::ProvenNotEmptyOrInconclusive => StaticEmptyProof::default(),
        }
    } else {
        let proof = static_literal_atoms_prove_empty(image, query, inputs)?;
        if allow_static_proof_cache {
            image.insert_static_proof(
                literal_cache_key,
                if proof.empty {
                    StaticProofCacheValue::ProvenEmpty
                } else {
                    StaticProofCacheValue::ProvenNotEmptyOrInconclusive
                },
            )?;
        }
        proof
    };
    timings.static_literal_proof_micros = timings
        .static_literal_proof_micros
        .saturating_add(elapsed_recorded_micros(literal_start));
    if proof.empty {
        return Ok(Some(proof));
    }
    let semijoin_start = Instant::now();
    let semijoin_cache_key = static_proof_cache_key(
        &image_key,
        query_shape,
        inputs,
        StaticProofKind::StaticSemijoin,
    );
    if allow_static_proof_cache
        && let Some(cached) = image.cached_static_proof(semijoin_cache_key)?
    {
        timings.static_semijoin_proof_micros = timings
            .static_semijoin_proof_micros
            .saturating_add(elapsed_recorded_micros(semijoin_start));
        match cached {
            StaticProofCacheValue::ProvenEmpty => {
                proof.empty = true;
            }
            StaticProofCacheValue::ProvenNotEmptyOrInconclusive => {
                proof.semijoin_skipped = true;
                proof.semijoin_skipped_reason = StaticSemijoinSkipReason::NegativeCache;
            }
        }
        return Ok(Some(proof));
    }
    if let Some(reason) = static_semijoin_skip_reason(image, query, inputs)? {
        timings.static_semijoin_proof_micros = timings
            .static_semijoin_proof_micros
            .saturating_add(elapsed_recorded_micros(semijoin_start));
        proof.semijoin_skipped = true;
        proof.semijoin_skipped_reason = reason;
        if allow_static_proof_cache {
            image.insert_static_proof(
                semijoin_cache_key,
                StaticProofCacheValue::ProvenNotEmptyOrInconclusive,
            )?;
        }
        return Ok(Some(proof));
    }
    let semijoin = static_semijoin_proves_empty(image, query, inputs)?;
    timings.static_semijoin_proof_micros = timings
        .static_semijoin_proof_micros
        .saturating_add(elapsed_recorded_micros(semijoin_start));
    proof.atoms_checked = proof.atoms_checked.saturating_add(semijoin.atoms_checked);
    proof.rows_scanned = proof.rows_scanned.saturating_add(semijoin.rows_scanned);
    proof.prefixes_probed = semijoin.prefixes_probed;
    proof.candidate_values = semijoin.candidate_values;
    proof.rounds = semijoin.rounds;
    proof.empty = semijoin.empty;
    proof.semijoin_skipped = semijoin.semijoin_skipped;
    proof.semijoin_skipped_reason = semijoin.semijoin_skipped_reason;
    if allow_static_proof_cache {
        image.insert_static_proof(
            semijoin_cache_key,
            if proof.empty {
                StaticProofCacheValue::ProvenEmpty
            } else {
                StaticProofCacheValue::ProvenNotEmptyOrInconclusive
            },
        )?;
    }
    Ok(Some(proof))
}

fn static_semijoin_skip_reason(
    image: &crate::QueryImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
) -> Result<Option<StaticSemijoinSkipReason>> {
    if query.atoms.len() < 2 {
        return Ok(Some(StaticSemijoinSkipReason::TooFewAtoms));
    }
    if !static_semijoin_output_allowed(&query.output) {
        return Ok(Some(StaticSemijoinSkipReason::OutputTooBroad));
    }
    if !query
        .atoms
        .iter()
        .any(|atom| atom_has_static_constraint(query, atom))
    {
        return Ok(Some(StaticSemijoinSkipReason::NoStaticConstraint));
    }

    let empty_candidates = BTreeMap::new();
    for atom in &query.atoms {
        if !atom_has_static_constraint(query, atom) {
            continue;
        }
        let relation = image
            .relation_by_id(atom.relation)
            .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
        let mut proof = StaticEmptyProof::default();
        let Some(seed_candidates) = enumerate_static_atom_candidates(
            relation,
            query,
            atom,
            inputs,
            &empty_candidates,
            &mut proof,
            false,
            STATIC_SEMIJOIN_MAX_SEED_CANDIDATES,
        )?
        else {
            continue;
        };
        if total_raw_candidate_values(&seed_candidates) <= STATIC_SEMIJOIN_MAX_SEED_CANDIDATES {
            return Ok(None);
        }
    }

    Ok(Some(StaticSemijoinSkipReason::NoCheapExactSeed))
}

fn static_semijoin_output_allowed(output: &OutputPlan) -> bool {
    match output {
        OutputPlan::Aggregate(plan) if is_global_count_plan(plan) => true,
        OutputPlan::Project(plan) => plan.vars.len() <= STATIC_SEMIJOIN_MAX_OUTPUT_VARS,
        OutputPlan::Aggregate(_) => false,
    }
}

fn static_literal_atoms_prove_empty(
    image: &crate::QueryImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
) -> Result<StaticEmptyProof> {
    let mut proof = StaticEmptyProof::default();
    let _span = tracing::debug_span!("bumbledb.query.static_empty.prove").entered();
    for atom in &query.atoms {
        if !atom
            .fields
            .iter()
            .any(|field| matches!(field.term, NormTerm::Input(_) | NormTerm::Literal(_)))
        {
            continue;
        }
        proof.atoms_checked += 1;
        let relation = image
            .relation_by_id(atom.relation)
            .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
        let mut matched = false;
        for row in 0..relation.row_count {
            proof.rows_scanned += 1;
            if static_atom_row_matches(relation, atom, RowId(row as u32), inputs)? {
                matched = true;
                break;
            }
        }
        if !matched {
            proof.empty = true;
            return Ok(proof);
        }
    }
    Ok(proof)
}

fn static_semijoin_proves_empty(
    image: &crate::QueryImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
) -> Result<StaticEmptyProof> {
    let mut proof = StaticEmptyProof::default();
    if query.atoms.len() < 2 {
        return Ok(proof);
    }
    let _span = tracing::debug_span!("bumbledb.query.static_semijoin.prove").entered();
    let mut candidates: BTreeMap<VarId, CandidateSet> = BTreeMap::new();

    for atom in &query.atoms {
        if !atom_has_static_constraint(query, atom) {
            continue;
        }
        let relation = image
            .relation_by_id(atom.relation)
            .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
        let Some(atom_candidates) = enumerate_static_atom_candidates(
            relation,
            query,
            atom,
            inputs,
            &candidates,
            &mut proof,
            false,
            STATIC_SEMIJOIN_MAX_CANDIDATES,
        )?
        else {
            continue;
        };
        if atom_candidates.is_empty() && !atom_variables(atom).is_empty() {
            proof.empty = true;
            return Ok(proof);
        }
        if merge_atom_candidates(&mut candidates, atom_candidates) {
            proof.candidate_values = total_candidate_values(&candidates);
            if proof.candidate_values as usize > STATIC_SEMIJOIN_MAX_CANDIDATES {
                return Ok(StaticEmptyProof {
                    empty: false,
                    ..proof
                });
            }
        }
        if has_empty_candidate(&candidates) {
            proof.empty = true;
            return Ok(proof);
        }
    }

    if candidates.is_empty() {
        return Ok(proof);
    }

    for _ in 0..STATIC_SEMIJOIN_MAX_ROUNDS {
        proof.rounds += 1;
        let mut changed = false;
        for atom in &query.atoms {
            if !atom_has_static_constraint(query, atom)
                && !atom
                    .fields
                    .iter()
                    .any(|field| matches!(field.term, NormTerm::Var(var) if candidates.contains_key(&var)))
            {
                continue;
            }
            let relation = image
                .relation_by_id(atom.relation)
                .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
            let Some(atom_candidates) = enumerate_static_atom_candidates(
                relation,
                query,
                atom,
                inputs,
                &candidates,
                &mut proof,
                true,
                STATIC_SEMIJOIN_MAX_CANDIDATES,
            )?
            else {
                continue;
            };
            proof.atoms_checked += 1;
            if atom_candidates.is_empty() && !atom_variables(atom).is_empty() {
                proof.empty = true;
                return Ok(proof);
            }
            if merge_atom_candidates(&mut candidates, atom_candidates) {
                changed = true;
                proof.candidate_values = total_candidate_values(&candidates);
                if proof.candidate_values as usize > STATIC_SEMIJOIN_MAX_CANDIDATES {
                    return Ok(StaticEmptyProof {
                        empty: false,
                        ..proof
                    });
                }
            }
            if has_empty_candidate(&candidates) {
                proof.empty = true;
                return Ok(proof);
            }
        }
        if !changed {
            return Ok(proof);
        }
    }
    Ok(StaticEmptyProof {
        empty: false,
        ..proof
    })
}

fn atom_has_static_constraint(query: &NormalizedQuery, atom: &NormAtom) -> bool {
    atom.fields
        .iter()
        .any(|field| matches!(field.term, NormTerm::Input(_) | NormTerm::Literal(_)))
        || query.predicates.iter().any(|predicate| {
            static_predicate_variable(predicate)
                .is_some_and(|variable| atom_has_variable(atom, variable))
        })
}

fn static_predicate_variable(predicate: &NormPredicate) -> Option<VarId> {
    match (&predicate.operands[0], &predicate.operands[1]) {
        (NormOperand::Var(variable), NormOperand::Literal(_))
        | (NormOperand::Literal(_), NormOperand::Var(variable))
        | (NormOperand::Var(variable), NormOperand::Input(_))
        | (NormOperand::Input(_), NormOperand::Var(variable)) => Some(*variable),
        _ => None,
    }
}

fn atom_has_variable(atom: &NormAtom, variable: VarId) -> bool {
    atom.fields
        .iter()
        .any(|field| matches!(field.term, NormTerm::Var(var) if var == variable))
}

#[expect(
    clippy::too_many_arguments,
    reason = "static candidate enumeration shares proof and budget state"
)]
fn enumerate_static_atom_candidates(
    relation: &RelationImage,
    query: &NormalizedQuery,
    atom: &NormAtom,
    inputs: &EncodedInputs,
    candidates: &BTreeMap<VarId, CandidateSet>,
    proof: &mut StaticEmptyProof,
    use_candidates: bool,
    max_candidates: usize,
) -> Result<Option<BTreeMap<VarId, BTreeSet<EncodedOwned>>>> {
    if let Some((index, probes)) =
        static_semijoin_prefixes(relation, query, atom, inputs, candidates, use_candidates)?
    {
        let mut out = empty_atom_candidate_map(atom);
        for probe in probes {
            if proof.prefixes_probed >= STATIC_SEMIJOIN_MAX_PROBES {
                return Ok(None);
            }
            proof.prefixes_probed += 1;
            for entry in static_semijoin_probe_entries(index, &probe) {
                if static_atom_entry_matches(index, entry, query, atom, inputs, candidates)? {
                    collect_atom_entry_candidates(index, entry, atom, &mut out)?;
                    if total_raw_candidate_values(&out) > max_candidates {
                        return Ok(None);
                    }
                }
            }
        }
        return Ok(Some(out));
    }

    if relation.row_count > STATIC_SEMIJOIN_SCAN_THRESHOLD {
        return Ok(None);
    }
    let mut out = empty_atom_candidate_map(atom);
    for row in 0..relation.row_count {
        if proof.rows_scanned >= STATIC_SEMIJOIN_MAX_SCANNED_ROWS {
            return Ok(None);
        }
        proof.rows_scanned += 1;
        let row = RowId(row as u32);
        if static_atom_row_matches_with_candidates(relation, query, atom, row, inputs, candidates)?
        {
            collect_atom_row_candidates(relation, atom, row, &mut out)?;
            if total_raw_candidate_values(&out) > max_candidates {
                return Ok(None);
            }
        }
    }
    Ok(Some(out))
}

fn static_semijoin_prefixes<'a>(
    relation: &'a RelationImage,
    query: &'a NormalizedQuery,
    atom: &'a NormAtom,
    inputs: &'a EncodedInputs,
    candidates: &'a BTreeMap<VarId, CandidateSet>,
    use_candidates: bool,
) -> Result<Option<StaticSemijoinProbes<'a>>> {
    let mut best: Option<(&RelationIndexImage, Vec<StaticSemijoinProbe>)> = None;
    for index in relation.indexes() {
        if atom
            .fields
            .iter()
            .any(|field| !index.contains_field(field.field))
        {
            continue;
        }
        let mut prefix_values: Vec<Vec<&EncodedOwned>> = Vec::new();
        let mut range_constraint = None;
        for field in &index.fields {
            let Some(atom_field) = atom
                .fields
                .iter()
                .find(|atom_field| atom_field.field == *field)
            else {
                break;
            };
            match static_field_constraint(atom_field, inputs, candidates, use_candidates) {
                Some(FieldConstraint::Single(value)) => prefix_values.push(vec![value]),
                Some(FieldConstraint::Candidates(values)) => {
                    if values.is_empty() {
                        return Ok(Some((index, Vec::new())));
                    }
                    prefix_values.push(values.iter().collect());
                }
                None => {
                    range_constraint = static_field_range_constraint(atom_field, query);
                    break;
                }
            }
        }
        if prefix_values.is_empty() && range_constraint.is_none() {
            continue;
        }
        let prefix_count = prefix_values
            .iter()
            .try_fold(1usize, |count, values| count.checked_mul(values.len()))
            .unwrap_or(STATIC_SEMIJOIN_MAX_PROBES as usize + 1);
        if prefix_count > STATIC_SEMIJOIN_MAX_PROBES as usize {
            continue;
        }
        let mut prefixes = Vec::with_capacity(prefix_count);
        build_static_prefixes(&prefix_values, 0, Vec::new(), &mut prefixes);
        let probes = if let Some(range) = range_constraint {
            static_range_probes(prefixes, range)
        } else {
            prefixes
                .into_iter()
                .map(StaticSemijoinProbe::Prefix)
                .collect()
        };
        if best
            .as_ref()
            .is_none_or(|(_, existing)| probes.len() < existing.len())
        {
            best = Some((index, probes));
        }
    }
    Ok(best)
}

fn static_semijoin_probe_entries<'a>(
    index: &'a RelationIndexImage,
    probe: &'a StaticSemijoinProbe,
) -> Box<dyn Iterator<Item = &'a [u8]> + 'a> {
    match probe {
        StaticSemijoinProbe::Prefix(prefix) => Box::new(index.entries_with_prefix(prefix)),
        StaticSemijoinProbe::Range {
            lower,
            upper,
            upper_inclusive,
        } => Box::new(index.entries_with_prefix_bounds(lower, upper.as_deref(), *upper_inclusive)),
    }
}

fn static_range_probes(
    mut prefixes: Vec<Vec<u8>>,
    range: StaticRangeConstraint,
) -> Vec<StaticSemijoinProbe> {
    if prefixes.is_empty() {
        prefixes.push(Vec::new());
    }
    prefixes
        .into_iter()
        .map(|prefix| {
            let mut lower = prefix.clone();
            if let Some((value, _)) = &range.lower {
                lower.extend_from_slice(value.as_bytes());
            }
            let upper = range.upper.as_ref().map(|(value, _)| {
                let mut upper = prefix;
                upper.extend_from_slice(value.as_bytes());
                upper
            });
            StaticSemijoinProbe::Range {
                lower,
                upper,
                upper_inclusive: range
                    .upper
                    .as_ref()
                    .is_some_and(|(_, inclusive)| *inclusive),
            }
        })
        .collect()
}

fn static_field_range_constraint(
    field: &NormAtomField,
    query: &NormalizedQuery,
) -> Option<StaticRangeConstraint> {
    let NormTerm::Var(variable) = field.term else {
        return None;
    };
    let mut range = StaticRangeConstraint {
        lower: None,
        upper: None,
    };
    for predicate in &query.predicates {
        if !encoded_comparison_supported(predicate.op, &predicate.value_type) {
            continue;
        }
        match (&predicate.operands[0], &predicate.operands[1]) {
            (NormOperand::Var(left), NormOperand::Literal(right)) if *left == variable => {
                apply_static_range_bound(&mut range, predicate.op, right);
            }
            (NormOperand::Literal(left), NormOperand::Var(right)) if *right == variable => {
                apply_static_range_bound(&mut range, reverse_comparison(predicate.op), left);
            }
            _ => {}
        }
    }
    (range.lower.is_some() || range.upper.is_some()).then_some(range)
}

fn apply_static_range_bound(
    range: &mut StaticRangeConstraint,
    operator: ComparisonOperator,
    value: &EncodedOwned,
) {
    match operator {
        ComparisonOperator::Eq => {
            merge_static_lower_bound(range, value, true);
            merge_static_upper_bound(range, value, true);
        }
        ComparisonOperator::Gt => merge_static_lower_bound(range, value, false),
        ComparisonOperator::Gte => merge_static_lower_bound(range, value, true),
        ComparisonOperator::Lt => merge_static_upper_bound(range, value, false),
        ComparisonOperator::Lte => merge_static_upper_bound(range, value, true),
        ComparisonOperator::NotEq => {}
    }
}

fn merge_static_lower_bound(
    range: &mut StaticRangeConstraint,
    value: &EncodedOwned,
    inclusive: bool,
) {
    let replace = range
        .lower
        .as_ref()
        .is_none_or(|(existing, existing_inclusive)| {
            value.as_bytes() > existing.as_bytes()
                || (value.as_bytes() == existing.as_bytes() && *existing_inclusive && !inclusive)
        });
    if replace {
        range.lower = Some((value.clone(), inclusive));
    }
}

fn merge_static_upper_bound(
    range: &mut StaticRangeConstraint,
    value: &EncodedOwned,
    inclusive: bool,
) {
    let replace = range
        .upper
        .as_ref()
        .is_none_or(|(existing, existing_inclusive)| {
            value.as_bytes() < existing.as_bytes()
                || (value.as_bytes() == existing.as_bytes() && *existing_inclusive && !inclusive)
        });
    if replace {
        range.upper = Some((value.clone(), inclusive));
    }
}

fn reverse_comparison(operator: ComparisonOperator) -> ComparisonOperator {
    match operator {
        ComparisonOperator::Eq => ComparisonOperator::Eq,
        ComparisonOperator::NotEq => ComparisonOperator::NotEq,
        ComparisonOperator::Lt => ComparisonOperator::Gt,
        ComparisonOperator::Lte => ComparisonOperator::Gte,
        ComparisonOperator::Gt => ComparisonOperator::Lt,
        ComparisonOperator::Gte => ComparisonOperator::Lte,
    }
}

fn static_field_constraint<'a>(
    field: &'a NormAtomField,
    inputs: &'a EncodedInputs,
    candidates: &'a BTreeMap<VarId, CandidateSet>,
    use_candidates: bool,
) -> Option<FieldConstraint<'a>> {
    match &field.term {
        NormTerm::Input(input) => inputs.get(*input).map(FieldConstraint::Single),
        NormTerm::Literal(literal) => Some(FieldConstraint::Single(literal)),
        NormTerm::Var(variable) if use_candidates => candidates
            .get(variable)
            .map(|set| FieldConstraint::Candidates(&set.values)),
        NormTerm::Var(_) | NormTerm::Wildcard => None,
    }
}

fn build_static_prefixes(
    values: &[Vec<&EncodedOwned>],
    depth: usize,
    mut current: Vec<u8>,
    out: &mut Vec<Vec<u8>>,
) {
    if depth == values.len() {
        out.push(current);
        return;
    }
    for value in &values[depth] {
        let len = current.len();
        current.extend_from_slice(value.as_bytes());
        build_static_prefixes(values, depth + 1, current.clone(), out);
        current.truncate(len);
    }
}

fn empty_atom_candidate_map(atom: &NormAtom) -> BTreeMap<VarId, BTreeSet<EncodedOwned>> {
    let mut out = BTreeMap::new();
    for field in &atom.fields {
        if let NormTerm::Var(variable) = field.term {
            out.entry(variable).or_insert_with(BTreeSet::new);
        }
    }
    out
}

fn static_atom_entry_matches(
    index: &RelationIndexImage,
    entry: &[u8],
    query: &NormalizedQuery,
    atom: &NormAtom,
    inputs: &EncodedInputs,
    candidates: &BTreeMap<VarId, CandidateSet>,
) -> Result<bool> {
    for field in &atom.fields {
        let Some(bytes) = index.component_bytes(entry, field.field) else {
            return Ok(false);
        };
        if !static_atom_field_bytes_match(field, bytes, inputs, candidates) {
            return Ok(false);
        }
    }
    static_atom_predicates_match(query, atom, inputs, |field| {
        index.component_bytes(entry, field.field)
    })
}

fn static_atom_row_matches_with_candidates(
    relation: &RelationImage,
    query: &NormalizedQuery,
    atom: &NormAtom,
    row: RowId,
    inputs: &EncodedInputs,
    candidates: &BTreeMap<VarId, CandidateSet>,
) -> Result<bool> {
    for field in &atom.fields {
        let bytes = relation
            .encoded_bytes(row, field.field)
            .ok_or_else(|| Error::internal("missing static semijoin atom field"))?;
        if !static_atom_field_bytes_match(field, bytes, inputs, candidates) {
            return Ok(false);
        }
    }
    static_atom_predicates_match(query, atom, inputs, |field| {
        relation.encoded_bytes(row, field.field)
    })
}

fn static_atom_field_bytes_match(
    field: &NormAtomField,
    bytes: &[u8],
    inputs: &EncodedInputs,
    candidates: &BTreeMap<VarId, CandidateSet>,
) -> bool {
    match &field.term {
        NormTerm::Input(input) => inputs
            .get(*input)
            .is_some_and(|value| value.as_bytes() == bytes),
        NormTerm::Literal(literal) => literal.as_bytes() == bytes,
        NormTerm::Var(variable) => candidates
            .get(variable)
            .is_none_or(|set| set.values.iter().any(|value| value.as_bytes() == bytes)),
        NormTerm::Wildcard => true,
    }
}

fn static_atom_predicates_match<'a>(
    query: &'a NormalizedQuery,
    atom: &'a NormAtom,
    inputs: &'a EncodedInputs,
    encoded_field: impl Fn(&'a NormAtomField) -> Option<&'a [u8]>,
) -> Result<bool> {
    for predicate in &query.predicates {
        let left = static_operand_bytes(&predicate.operands[0], atom, inputs, &encoded_field);
        let right = static_operand_bytes(&predicate.operands[1], atom, inputs, &encoded_field);
        let (Some(left), Some(right)) = (left, right) else {
            continue;
        };
        if encoded_comparison_supported(predicate.op, &predicate.value_type)
            && !compare_encoded_values(left, predicate.op, right)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn static_operand_bytes<'a>(
    operand: &'a NormOperand,
    atom: &'a NormAtom,
    inputs: &'a EncodedInputs,
    encoded_field: &impl Fn(&'a NormAtomField) -> Option<&'a [u8]>,
) -> Option<&'a [u8]> {
    match operand {
        NormOperand::Var(variable) => atom.fields.iter().find_map(|field| {
            matches!(field.term, NormTerm::Var(var) if var == *variable)
                .then(|| encoded_field(field))
                .flatten()
        }),
        NormOperand::Input(input) => inputs.get(*input).map(EncodedOwned::as_bytes),
        NormOperand::Literal(literal) => Some(literal.as_bytes()),
    }
}

fn collect_atom_entry_candidates(
    index: &RelationIndexImage,
    entry: &[u8],
    atom: &NormAtom,
    out: &mut BTreeMap<VarId, BTreeSet<EncodedOwned>>,
) -> Result<()> {
    for field in &atom.fields {
        if let NormTerm::Var(variable) = field.term {
            let bytes = index
                .component_bytes(entry, field.field)
                .ok_or_else(|| Error::internal("missing static semijoin index component"))?;
            out.entry(variable)
                .or_default()
                .insert(encoded_owned_for_width(
                    field.value_type.encoded_width(),
                    bytes,
                )?);
        }
    }
    Ok(())
}

fn collect_atom_row_candidates(
    relation: &RelationImage,
    atom: &NormAtom,
    row: RowId,
    out: &mut BTreeMap<VarId, BTreeSet<EncodedOwned>>,
) -> Result<()> {
    for field in &atom.fields {
        if let NormTerm::Var(variable) = field.term {
            let bytes = relation
                .encoded_bytes(row, field.field)
                .ok_or_else(|| Error::internal("missing static semijoin row field"))?;
            out.entry(variable)
                .or_default()
                .insert(encoded_owned_for_width(
                    field.value_type.encoded_width(),
                    bytes,
                )?);
        }
    }
    Ok(())
}

fn merge_atom_candidates(
    candidates: &mut BTreeMap<VarId, CandidateSet>,
    atom_candidates: BTreeMap<VarId, BTreeSet<EncodedOwned>>,
) -> bool {
    let mut changed = false;
    for (variable, values) in atom_candidates {
        match candidates.get_mut(&variable) {
            Some(existing) => {
                let intersection = existing
                    .values
                    .intersection(&values)
                    .cloned()
                    .collect::<BTreeSet<_>>();
                if intersection.len() != existing.values.len() {
                    existing.values = intersection;
                    changed = true;
                }
            }
            None => {
                candidates.insert(variable, CandidateSet::new(values));
                changed = true;
            }
        }
    }
    changed
}

fn has_empty_candidate(candidates: &BTreeMap<VarId, CandidateSet>) -> bool {
    candidates.values().any(|set| set.values.is_empty())
}

fn total_candidate_values(candidates: &BTreeMap<VarId, CandidateSet>) -> u64 {
    candidates.values().map(|set| set.values.len() as u64).sum()
}

fn total_raw_candidate_values(candidates: &BTreeMap<VarId, BTreeSet<EncodedOwned>>) -> usize {
    candidates.values().map(BTreeSet::len).sum()
}

fn static_atom_row_matches(
    relation: &RelationImage,
    atom: &NormAtom,
    row: RowId,
    inputs: &EncodedInputs,
) -> Result<bool> {
    for field in &atom.fields {
        let expected = match &field.term {
            NormTerm::Input(input) => inputs.get(*input),
            NormTerm::Literal(literal) => Some(literal),
            NormTerm::Var(_) | NormTerm::Wildcard => None,
        };
        let Some(expected) = expected else {
            continue;
        };
        let bytes = relation
            .encoded_bytes(row, field.field)
            .ok_or_else(|| Error::internal("missing static atom field"))?;
        if expected.as_bytes() != bytes {
            return Ok(false);
        }
    }
    Ok(true)
}

fn static_empty_plan(
    query: &NormalizedQuery,
    query_image_cache: QueryImageCacheDiagnostics,
    planner_stats: PlannerStatsCacheDiagnostics,
    prepared_plan_cache: PreparedPlanCacheDiagnostics,
) -> QueryPlan {
    QueryPlan {
        variable_order: Vec::new(),
        variable_estimates: Vec::new(),
        missing_indexes: Vec::new(),
        optimizer: OptimizerTrace {
            chosen: "static_empty".to_owned(),
            candidates: Vec::new(),
        },
        plan_family: PlanFamily::StaticEmpty,
        query_image_cache,
        planner_stats,
        prepared_plan_cache,
        node_rows: Vec::new(),
        node_timings: Vec::new(),
        free_join: FreeJoinPlan {
            nodes: Vec::new(),
            output: query.output.clone(),
            estimates: PlanEstimates::default(),
        },
        direct_kernel: None,
        runtime_kind: QueryRuntimeKind::StaticEmpty,
        timings: QueryTimings::default(),
        allocations: QueryAllocationStats::default(),
        counters: PlanCounters::default(),
        uses_indexed_multiway_join: query.atoms.len() > 1,
    }
}

fn static_empty_output_from_typed(
    query: &TypedQuery,
    query_image_cache: QueryImageCacheDiagnostics,
    timings: QueryTimings,
    total_start: Instant,
    total_alloc_start: allocation::AllocationSnapshot,
    cache_hit: bool,
) -> QueryOutput {
    QueryOutput {
        result: QueryResultSet::new(
            result_columns_from_typed(query),
            empty_output_rows(&output_plan_from_typed_find(query)),
        ),
        plan: static_empty_plan_from_typed(
            query,
            query_image_cache,
            timings,
            total_start,
            total_alloc_start,
            cache_hit,
        ),
    }
}

fn empty_output_rows(output: &OutputPlan) -> Vec<Vec<Value>> {
    match output {
        OutputPlan::Aggregate(plan) if is_global_count_plan(plan) => vec![vec![Value::U64(0)]],
        OutputPlan::Project(_) | OutputPlan::Aggregate(_) => Vec::new(),
    }
}

fn static_empty_plan_from_typed(
    query: &TypedQuery,
    query_image_cache: QueryImageCacheDiagnostics,
    mut timings: QueryTimings,
    total_start: Instant,
    total_alloc_start: allocation::AllocationSnapshot,
    cache_hit: bool,
) -> QueryPlan {
    let mut counters = PlanCounters::default();
    if cache_hit {
        counters.static_empty_cache_hits = 1;
    }
    finish_timings(&mut timings, total_start);
    let allocations =
        QueryAllocationStats::default().with_total(allocation_delta_since(total_alloc_start));
    QueryPlan {
        variable_order: Vec::new(),
        variable_estimates: Vec::new(),
        missing_indexes: Vec::new(),
        optimizer: OptimizerTrace {
            chosen: "static_empty".to_owned(),
            candidates: Vec::new(),
        },
        plan_family: PlanFamily::StaticEmpty,
        query_image_cache,
        planner_stats: PlannerStatsCacheDiagnostics::default(),
        prepared_plan_cache: PreparedPlanCacheDiagnostics::default(),
        node_rows: Vec::new(),
        node_timings: Vec::new(),
        free_join: FreeJoinPlan {
            nodes: Vec::new(),
            output: output_plan_from_typed_find(query),
            estimates: PlanEstimates::default(),
        },
        direct_kernel: None,
        runtime_kind: QueryRuntimeKind::StaticEmpty,
        timings,
        allocations,
        counters,
        uses_indexed_multiway_join: typed_relation_clause_count(query) > 1,
    }
}

fn result_columns_from_typed(query: &TypedQuery) -> Vec<ResultColumn> {
    query
        .find
        .iter()
        .map(|term| match term {
            TypedFindTerm::Variable { variable } => {
                ResultColumn::Variable(query.variables[*variable].name.clone())
            }
            TypedFindTerm::Aggregate {
                function, variable, ..
            } => ResultColumn::Aggregate {
                function: *function,
                variable: query.variables[*variable].name.clone(),
            },
        })
        .collect()
}

fn output_plan_from_typed_find(query: &TypedQuery) -> OutputPlan {
    if query
        .find
        .iter()
        .any(|term| matches!(term, TypedFindTerm::Aggregate { .. }))
    {
        let mut group_vars = Vec::new();
        let mut aggregates = Vec::new();
        for term in &query.find {
            match term {
                TypedFindTerm::Variable { variable } => group_vars.push(VarId(*variable as u16)),
                TypedFindTerm::Aggregate {
                    function,
                    variable,
                    domain,
                    value_type,
                } => aggregates.push(AggregateTerm {
                    function: *function,
                    var: VarId(*variable as u16),
                    domain_vars: domain
                        .iter()
                        .map(|variable| VarId(*variable as u16))
                        .collect(),
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
            vars: query
                .find
                .iter()
                .filter_map(|term| match term {
                    TypedFindTerm::Variable { variable } => Some(VarId(*variable as u16)),
                    TypedFindTerm::Aggregate { .. } => None,
                })
                .collect(),
        })
    }
}

fn typed_relation_clause_count(query: &TypedQuery) -> usize {
    query
        .clauses
        .iter()
        .filter(|clause| matches!(clause, TypedClause::Relation(_)))
        .count()
}

// Direct kernels own simple materialized scans/chains/ranges before speculative proof.
#[expect(
    clippy::too_many_arguments,
    reason = "early direct materialized path needs current execution diagnostics"
)]
fn try_execute_direct_materialized_kernel(
    image: &crate::QueryImage,
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    encoded_inputs: &EncodedInputs,
    query_image_cache: QueryImageCacheDiagnostics,
    planner_stats: PlannerStatsCacheDiagnostics,
    prepared_plan_cache: PreparedPlanCacheDiagnostics,
    mut timings: QueryTimings,
    mut allocations: QueryAllocationStats,
    total_start: Instant,
    total_alloc_start: allocation::AllocationSnapshot,
) -> Result<Option<QueryOutput>> {
    if !matches!(query.output, OutputPlan::Project(_)) {
        return Ok(None);
    }
    let Some(direct_kernel) = try_direct_kernel(query) else {
        return Ok(None);
    };
    let plan_family = match &direct_kernel.kind {
        DirectKernel::ChainProbe(_) => PlanFamily::IndexNestedLoop,
        DirectKernel::PrefixRange(_) => PlanFamily::Direct,
    };
    let direct_summary = direct_kernel.summary.clone();
    let mut plan = ExecutionPlan {
        variable_order_ids: (0..query.vars.len()).collect(),
        relation_atoms: query.atoms.clone(),
        comparisons: query.predicates.clone(),
        direct_kernel: Some(direct_kernel),
        summary: QueryPlan {
            variable_order: query.vars.iter().map(|var| var.name.clone()).collect(),
            variable_estimates: Vec::new(),
            missing_indexes: Vec::new(),
            optimizer: OptimizerTrace {
                chosen: "direct_materialized".to_owned(),
                candidates: vec![PlanCandidate {
                    name: "direct_materialized".to_owned(),
                    family: plan_family,
                    implementations: Vec::new(),
                    cost: CostKey {
                        estimated_micros: 1,
                        setup_micros: 0,
                        memory_bytes: 0,
                        materialization_penalty: 0,
                        candidate_rank: 0,
                        implementation_mask: 0,
                    },
                    selected: true,
                    rejected_reason: "selected direct shape before static proof".to_owned(),
                }],
            },
            plan_family,
            query_image_cache,
            planner_stats,
            prepared_plan_cache,
            node_rows: Vec::new(),
            node_timings: Vec::new(),
            free_join: FreeJoinPlan {
                nodes: Vec::new(),
                output: query.output.clone(),
                estimates: PlanEstimates::default(),
            },
            direct_kernel: Some(direct_summary),
            runtime_kind: QueryRuntimeKind::Unknown,
            timings: QueryTimings::default(),
            allocations: QueryAllocationStats::default(),
            counters: PlanCounters::default(),
            uses_indexed_multiway_join: query.atoms.len() > 1,
        },
    };
    let mut sink = OutputSink::new(&plan.summary.free_join.output);
    let execute_start = Instant::now();
    let execute_alloc_start = allocation::snapshot();
    execute_direct_kernel(
        image,
        txn,
        schema,
        query,
        encoded_inputs,
        &mut plan,
        &mut sink,
    )?;
    timings.execute_micros = elapsed_micros(execute_start);
    allocations.execute = allocation_delta_since(execute_alloc_start);

    let columns = result_columns(query);
    let sink_finish_start = Instant::now();
    let sink_finish_alloc_start = allocation::snapshot();
    let rows = sink.finish(txn, query, &mut plan.summary.counters)?;
    timings.sink_finish_micros = elapsed_micros(sink_finish_start);
    allocations.sink_finish = allocation_delta_since(sink_finish_alloc_start);
    plan.summary.timings = timings;
    plan.summary.allocations = allocations;
    plan.summary.counters.output_rows = rows.len() as u64;
    finish_timings(&mut plan.summary.timings, total_start);
    plan.summary.allocations = plan
        .summary
        .allocations
        .with_total(allocation_delta_since(total_alloc_start));
    Ok(Some(QueryOutput {
        result: QueryResultSet::new(columns, rows),
        plan: plan.summary,
    }))
}

#[expect(
    clippy::too_many_arguments,
    reason = "direct storage path needs current execution diagnostics"
)]
fn try_execute_direct_storage_project(
    txn: &ReadTxn<'_>,
    schema: &StorageSchema,
    query: &NormalizedQuery,
    encoded_inputs: &EncodedInputs,
    raw_inputs: &InputBindings,
    mut timings: QueryTimings,
    mut allocations: QueryAllocationStats,
    total_start: Instant,
    total_alloc_start: allocation::AllocationSnapshot,
) -> Result<Option<QueryOutput>> {
    if query.atoms.len() != 1 || !matches!(query.output, OutputPlan::Project(_)) {
        return Ok(None);
    }
    let atom = &query.atoms[0];
    let Some(access) = direct_storage_access(schema, query, atom, raw_inputs)? else {
        return Ok(None);
    };

    let execute_start = Instant::now();
    let execute_alloc_start = allocation::snapshot();
    let mut counters = PlanCounters::default();
    let mut sink = OutputSink::new(&query.output);
    counters.direct_kernel_probes += 1;
    let index_name = match &access {
        DirectStorageAccess::Prefix { index_name, .. }
        | DirectStorageAccess::Range { index_name, .. } => index_name.clone(),
    };
    let scan = match access {
        DirectStorageAccess::Prefix { values, .. } => txn.scan_prefix(
            schema,
            &atom.relation_name,
            &index_name,
            &FieldValues::new(&atom.relation_name, values),
        )?,
        DirectStorageAccess::Range { start, end, .. } => {
            txn.scan_range(schema, &atom.relation_name, &index_name, start, end)?
        }
    };
    for item in scan {
        let item = item?;
        counters.direct_kernel_rows += 1;
        let mut binding = EncodedBinding::new(query.vars.len());
        counters.direct_bind_attempts += 1;
        if !bind_direct_storage_row(txn, query, encoded_inputs, atom, &item.row, &mut binding)? {
            continue;
        }
        counters.direct_bind_successes += 1;
        let before = counters.comparisons_evaluated;
        if !comparisons_ready_pass(
            txn,
            &query.predicates,
            query,
            encoded_inputs,
            &binding,
            &mut counters,
        )? {
            counters.direct_kernel_predicates = counters
                .direct_kernel_predicates
                .saturating_add(counters.comparisons_evaluated.saturating_sub(before));
            continue;
        }
        counters.direct_kernel_predicates = counters
            .direct_kernel_predicates
            .saturating_add(counters.comparisons_evaluated.saturating_sub(before));
        counters.bindings_yielded += 1;
        counters.bindings_completed += 1;
        counters.direct_storage_output_rows += 1;
        if !sink.emit_direct_project(query, &binding, &mut counters)? {
            sink.emit(txn, query, &binding, &mut counters)?;
        }
    }
    timings.execute_micros = elapsed_micros(execute_start);
    timings.direct_storage_micros = timings.execute_micros;
    allocations.execute = allocation_delta_since(execute_alloc_start);

    let finish_start = Instant::now();
    let finish_alloc_start = allocation::snapshot();
    let rows = sink.finish(txn, query, &mut counters)?;
    timings.sink_finish_micros = elapsed_micros(finish_start);
    allocations.sink_finish = allocation_delta_since(finish_alloc_start);
    counters.output_rows = rows.len() as u64;
    finish_timings(&mut timings, total_start);
    allocations = allocations.with_total(allocation_delta_since(total_alloc_start));

    Ok(Some(QueryOutput {
        result: QueryResultSet::new(result_columns(query), rows),
        plan: QueryPlan {
            variable_order: query.vars.iter().map(|var| var.name.clone()).collect(),
            variable_estimates: Vec::new(),
            missing_indexes: Vec::new(),
            optimizer: OptimizerTrace {
                chosen: "direct_storage".to_owned(),
                candidates: vec![PlanCandidate {
                    name: "direct_storage".to_owned(),
                    family: PlanFamily::Direct,
                    implementations: Vec::new(),
                    cost: CostKey {
                        estimated_micros: counters.direct_kernel_rows.max(1),
                        setup_micros: 0,
                        memory_bytes: 0,
                        materialization_penalty: counters.materialized_output_values,
                        candidate_rank: 0,
                        implementation_mask: 0,
                    },
                    selected: true,
                    rejected_reason: "selected direct shape before query image".to_owned(),
                }],
            },
            plan_family: PlanFamily::Direct,
            query_image_cache: txn.query_images.diagnostics(),
            planner_stats: PlannerStatsCacheDiagnostics::default(),
            prepared_plan_cache: PreparedPlanCacheDiagnostics::default(),
            node_rows: Vec::new(),
            node_timings: Vec::new(),
            free_join: FreeJoinPlan {
                nodes: Vec::new(),
                output: query.output.clone(),
                estimates: PlanEstimates::default(),
            },
            direct_kernel: Some(DirectKernelSummary {
                kind: DirectKernelKind::PrefixRange,
                target: format!("{}.{}", atom.relation_name, index_name),
                steps: 1,
            }),
            runtime_kind: QueryRuntimeKind::DirectKernel,
            timings,
            allocations,
            counters,
            uses_indexed_multiway_join: false,
        },
    }))
}

fn direct_storage_access(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atom: &NormAtom,
    raw_inputs: &InputBindings,
) -> Result<Option<DirectStorageAccess>> {
    if let Some(access) = direct_storage_prefix(schema, query, atom, raw_inputs)? {
        return Ok(Some(access));
    }
    direct_storage_range(schema, query, atom, raw_inputs)
}

fn direct_storage_prefix(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atom: &NormAtom,
    raw_inputs: &InputBindings,
) -> Result<Option<DirectStorageAccess>> {
    let paths = schema.access_paths(&atom.relation_name)?;
    let mut best = None;
    for path in paths {
        let mut values = Vec::new();
        for field_name in &path.leading_fields {
            let Some(field) = atom
                .fields
                .iter()
                .find(|field| &field.field_name == field_name)
            else {
                break;
            };
            let NormTerm::Input(input) = field.term else {
                break;
            };
            let input = &query.inputs[input.0 as usize];
            let Some(value) = raw_inputs.value(&input.name) else {
                return Err(Error::missing_input(&input.name));
            };
            values.push((field.field_name.clone(), value.clone()));
        }
        if values.is_empty() {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|(_, existing): &(String, Vec<(String, Value)>)| {
                values.len() > existing.len()
            })
        {
            best = Some((path.index_name, values));
        }
    }
    Ok(best.map(|(index_name, values)| DirectStorageAccess::Prefix { index_name, values }))
}

fn direct_storage_range(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atom: &NormAtom,
    raw_inputs: &InputBindings,
) -> Result<Option<DirectStorageAccess>> {
    let Some((variable, start, end)) = direct_storage_range_bounds(query, raw_inputs)? else {
        return Ok(None);
    };
    let Some(field) = atom
        .fields
        .iter()
        .find(|field| matches!(field.term, NormTerm::Var(var) if var.0 as usize == variable))
    else {
        return Ok(None);
    };
    let Some(path) = schema
        .access_paths(&atom.relation_name)?
        .into_iter()
        .find(|path| {
            path.kind == IndexKind::Range
                && path
                    .leading_fields
                    .first()
                    .is_some_and(|leading| leading == &field.field_name)
        })
    else {
        return Ok(None);
    };
    Ok(Some(DirectStorageAccess::Range {
        index_name: path.index_name,
        start,
        end,
    }))
}

fn direct_storage_range_bounds(
    query: &NormalizedQuery,
    raw_inputs: &InputBindings,
) -> Result<Option<DirectRangeBounds>> {
    if query.predicates.is_empty() {
        return Ok(None);
    }
    let mut variable = None;
    let mut start = None;
    let mut end = None;
    for predicate in &query.predicates {
        let (candidate, bound, var_is_left) = match (&predicate.operands[0], &predicate.operands[1])
        {
            (NormOperand::Var(var), NormOperand::Input(input)) => {
                let input = &query.inputs[input.0 as usize];
                let value = raw_inputs
                    .value(&input.name)
                    .ok_or_else(|| Error::missing_input(&input.name))?
                    .clone();
                (var.0 as usize, value, true)
            }
            (NormOperand::Input(input), NormOperand::Var(var)) => {
                let input = &query.inputs[input.0 as usize];
                let value = raw_inputs
                    .value(&input.name)
                    .ok_or_else(|| Error::missing_input(&input.name))?
                    .clone();
                (var.0 as usize, value, false)
            }
            _ => return Ok(None),
        };
        if variable.is_some_and(|existing| existing != candidate) {
            return Ok(None);
        }
        variable = Some(candidate);
        match (predicate.op, var_is_left) {
            (ComparisonOperator::Gte, true) | (ComparisonOperator::Lte, false) => {
                start = Some(bound)
            }
            (ComparisonOperator::Lt, true) | (ComparisonOperator::Gt, false) => end = Some(bound),
            _ => return Ok(None),
        }
    }
    Ok(variable.map(|variable| (variable, start, end)))
}

fn bind_direct_storage_row(
    txn: &ReadTxn<'_>,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    row: &crate::Row,
    binding: &mut EncodedBinding,
) -> Result<bool> {
    for field in &atom.fields {
        let Some(value) = row.value(&field.field_name) else {
            return Ok(false);
        };
        match &field.term {
            NormTerm::Var(variable) => {
                let encoded =
                    txn.encode_query_value(&query.vars[variable.0 as usize].value_type, value)?;
                let encoded = encoded_owned_for_width(
                    query.vars[variable.0 as usize].value_type.encoded_width(),
                    &encoded,
                )?;
                if !binding.bind(variable.0 as usize, encoded) {
                    return Ok(false);
                }
            }
            NormTerm::Input(input) => {
                let input = inputs
                    .get(*input)
                    .ok_or_else(|| Error::internal("missing direct storage input"))?;
                let encoded = txn.encode_query_value(&field.value_type, value)?;
                if encoded.as_slice() != input.as_bytes() {
                    return Ok(false);
                }
            }
            NormTerm::Literal(literal) => {
                let encoded = txn.encode_query_value(&field.value_type, value)?;
                if encoded.as_slice() != literal.as_bytes() {
                    return Ok(false);
                }
            }
            NormTerm::Wildcard => {}
        }
    }
    Ok(true)
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
    if plan.direct_kernel.is_some() {
        plan.summary.runtime_kind = QueryRuntimeKind::DirectKernel;
        return execute_direct_kernel(image, txn, schema, query, inputs, plan, sink);
    }
    if !plan.summary.free_join.is_pure_lftj() {
        return Err(Error::internal("non-pure free join plan has no runtime"));
    }
    plan.summary.runtime_kind = QueryRuntimeKind::Lftj;
    execute_lftj(image, txn, query, inputs, plan, sink)
}

fn try_direct_kernel(query: &NormalizedQuery) -> Option<DirectKernelPlan> {
    try_direct_prefix_range_kernel(query).or_else(|| try_direct_chain_kernel(query))
}

fn try_direct_prefix_range_kernel(query: &NormalizedQuery) -> Option<DirectKernelPlan> {
    if query.atoms.len() != 1 || !matches!(query.output, OutputPlan::Project(_)) {
        return None;
    }
    let atom = &query.atoms[0];
    let range_variable = direct_range_variable(query)?;
    if !atom_contains_variable(atom, range_variable) {
        return None;
    }
    let mut prefix_fields = SmallVec::new();
    let mut prefix_terms = SmallVec::new();
    for field in &atom.fields {
        match &field.term {
            NormTerm::Input(_) | NormTerm::Literal(_) => {
                prefix_fields.push(field.field);
                prefix_terms.push(field.term.clone());
            }
            NormTerm::Var(variable) if variable.0 as usize == range_variable => {}
            NormTerm::Var(_) | NormTerm::Wildcard => {}
        }
    }
    if prefix_fields.is_empty() {
        return None;
    }
    Some(DirectKernelPlan {
        kind: DirectKernel::PrefixRange(DirectPrefixRangePlan {
            atom_id: atom.id.0 as usize,
            relation: atom.relation,
            index_name: direct_index_name(&atom.relation_name, "prefix_range"),
            prefix_fields,
            prefix_terms,
        }),
        summary: DirectKernelSummary {
            kind: DirectKernelKind::PrefixRange,
            target: atom.relation_name.clone(),
            steps: 1,
        },
    })
}

fn direct_range_variable(query: &NormalizedQuery) -> Option<usize> {
    let mut range_variable = None;
    for predicate in &query.predicates {
        let variable = match (&predicate.operands[0], &predicate.operands[1]) {
            (NormOperand::Var(variable), NormOperand::Input(_) | NormOperand::Literal(_))
            | (NormOperand::Input(_) | NormOperand::Literal(_), NormOperand::Var(variable)) => {
                variable.0 as usize
            }
            _ => return None,
        };
        if range_variable.is_some_and(|existing| existing != variable) {
            return None;
        }
        range_variable = Some(variable);
    }
    range_variable
}

fn try_direct_chain_kernel(query: &NormalizedQuery) -> Option<DirectKernelPlan> {
    if query.atoms.len() < 2
        || !query.predicates.is_empty()
        || !matches!(query.output, OutputPlan::Project(_))
    {
        return None;
    }
    let mut bound = BTreeSet::new();
    let mut existence_checks = Vec::new();
    let mut steps = Vec::new();
    for atom in &query.atoms {
        let variables = atom_variables(atom);
        let unbound = variables
            .iter()
            .copied()
            .filter(|variable| !bound.contains(variable))
            .collect::<SmallParticipants>();
        if unbound.is_empty() {
            let mut fields = SmallVec::new();
            let mut terms = SmallVec::new();
            for field in &atom.fields {
                if direct_term_is_bound(&field.term, &bound) {
                    fields.push(field.field);
                    terms.push(field.term.clone());
                }
            }
            if fields.is_empty() {
                return None;
            }
            existence_checks.push(DirectExistenceCheck {
                atom_id: atom.id.0 as usize,
                relation: atom.relation,
                index_name: direct_index_name(&atom.relation_name, "chain_exists"),
                fields,
                terms,
            });
            continue;
        }
        if unbound.len() != 1 {
            return None;
        }
        let bind_var = unbound[0];
        let bind_field = atom
            .fields
            .iter()
            .find(|field| matches!(field.term, NormTerm::Var(variable) if variable.0 as usize == bind_var))?
            .field;
        let mut prefix_fields = SmallVec::new();
        let mut prefix_terms = SmallVec::new();
        for field in &atom.fields {
            if direct_term_is_bound(&field.term, &bound) {
                prefix_fields.push(field.field);
                prefix_terms.push(field.term.clone());
            }
        }
        if prefix_fields.is_empty() {
            return None;
        }
        steps.push(DirectChainStep {
            atom_id: atom.id.0 as usize,
            relation: atom.relation,
            index_name: direct_index_name(&atom.relation_name, "chain_step"),
            prefix_fields,
            prefix_terms,
            bind_var,
            bind_field,
        });
        bound.insert(bind_var);
    }
    if steps.is_empty() {
        return None;
    }
    Some(DirectKernelPlan {
        kind: DirectKernel::ChainProbe(DirectChainProbePlan {
            existence_checks,
            steps,
        }),
        summary: DirectKernelSummary {
            kind: DirectKernelKind::ChainProbe,
            target: query
                .atoms
                .iter()
                .map(|atom| atom.relation_name.as_str())
                .collect::<Vec<_>>()
                .join("->"),
            steps: query.atoms.len(),
        },
    })
}

fn direct_term_is_bound(term: &NormTerm, bound: &BTreeSet<usize>) -> bool {
    match term {
        NormTerm::Var(variable) => bound.contains(&(variable.0 as usize)),
        NormTerm::Input(_) | NormTerm::Literal(_) => true,
        NormTerm::Wildcard => false,
    }
}

fn direct_index_name(relation: &str, kind: &str) -> String {
    format!("{relation}_direct_{kind}")
}

fn execute_direct_kernel<'txn, 'query, S: TupleSink>(
    image: &crate::QueryImage,
    txn: &ReadTxn<'txn>,
    schema: &StorageSchema,
    query: &'query NormalizedQuery,
    inputs: &EncodedInputs,
    plan: &mut ExecutionPlan,
    sink: &mut S,
) -> Result<()> {
    let Some(direct) = plan.direct_kernel.clone() else {
        return Err(Error::internal("missing direct kernel plan"));
    };
    match direct.kind {
        DirectKernel::PrefixRange(kernel) => {
            plan.summary.runtime_kind = QueryRuntimeKind::DirectKernel;
            execute_direct_prefix_range(image, txn, query, inputs, plan, sink, &kernel)
        }
        DirectKernel::ChainProbe(kernel) => {
            plan.summary.runtime_kind = QueryRuntimeKind::IndexNestedLoop;
            let mut executor = DirectChainExecutor {
                image,
                txn,
                schema,
                query,
                inputs,
                plan,
                sink,
                kernel: &kernel,
                binding: EncodedBinding::new(query.vars.len()),
            };
            executor.execute()
        }
    }
}

fn execute_direct_prefix_range<'txn, 'query, S: TupleSink>(
    image: &crate::QueryImage,
    txn: &ReadTxn<'txn>,
    query: &'query NormalizedQuery,
    inputs: &EncodedInputs,
    plan: &mut ExecutionPlan,
    sink: &mut S,
    kernel: &DirectPrefixRangePlan,
) -> Result<()> {
    let atom = &plan.relation_atoms[kernel.atom_id];
    let index = direct_hash_index(
        image,
        kernel.relation,
        &kernel.prefix_fields,
        None,
        &kernel.index_name,
        &mut plan.summary.counters,
    )?;
    let prefix = direct_prefix(
        &kernel.prefix_terms,
        inputs,
        &EncodedBinding::new(query.vars.len()),
    )?;
    let refs = encoded_refs(&prefix);
    let row_count = AccessSource::HashTrie(index.as_ref()).count(&refs)?;
    plan.summary.counters.direct_kernel_probes += 1;
    if row_count == 0 {
        return Ok(());
    }
    let relation = direct_relation(image, kernel.relation)?;
    let mut binding = EncodedBinding::new(query.vars.len());
    for row in index.rows_for_prefix(&refs) {
        plan.summary.counters.direct_kernel_rows += 1;
        let bound = bind_atom_variables(relation, atom, row, &mut binding)?;
        if !direct_row_satisfies_atom(relation, atom, row, inputs, &binding)? {
            unbind_variables(&mut binding, &bound);
            continue;
        }
        let before = plan.summary.counters.comparisons_evaluated;
        let keep = comparisons_ready_pass(
            txn,
            &plan.comparisons,
            query,
            inputs,
            &binding,
            &mut plan.summary.counters,
        )?;
        plan.summary.counters.direct_kernel_predicates = plan
            .summary
            .counters
            .direct_kernel_predicates
            .saturating_add(
                plan.summary
                    .counters
                    .comparisons_evaluated
                    .saturating_sub(before),
            );
        if keep {
            plan.summary.counters.bindings_yielded += 1;
            let _span = tracing::trace_span!("bumbledb.query.sink.emit").entered();
            sink.emit(txn, query, &binding, &mut plan.summary.counters)?;
        }
        unbind_variables(&mut binding, &bound);
    }
    Ok(())
}

struct DirectChainExecutor<'txn, 'input, 'query, 'plan, S: TupleSink> {
    image: &'input crate::QueryImage,
    txn: &'input ReadTxn<'txn>,
    schema: &'input StorageSchema,
    query: &'query NormalizedQuery,
    inputs: &'input EncodedInputs,
    plan: &'plan mut ExecutionPlan,
    sink: &'plan mut S,
    kernel: &'input DirectChainProbePlan,
    binding: EncodedBinding,
}

impl<S: TupleSink> DirectChainExecutor<'_, '_, '_, '_, S> {
    fn execute(&mut self) -> Result<()> {
        for check in &self.kernel.existence_checks {
            if let Some(rows) =
                self.image_rows_for_terms(check.atom_id, &check.fields, &check.terms)?
            {
                if !rows.iter().try_fold(false, |found, row| {
                    if found {
                        Ok(true)
                    } else {
                        self.image_row_satisfies_atom(&self.plan.relation_atoms[check.atom_id], row)
                    }
                })? {
                    return Ok(());
                }
                continue;
            }
            if let Some(rows) = self.storage_rows_for_terms(
                check.relation,
                check.atom_id,
                &check.fields,
                &check.terms,
            )? {
                if !rows.iter().try_fold(false, |found, row| {
                    if found {
                        Ok(true)
                    } else {
                        self.storage_row_satisfies_atom(
                            &self.plan.relation_atoms[check.atom_id],
                            row,
                        )
                    }
                })? {
                    return Ok(());
                }
                continue;
            }
            let index = direct_hash_index(
                self.image,
                check.relation,
                &check.fields,
                None,
                &check.index_name,
                &mut self.plan.summary.counters,
            )?;
            let prefix = direct_prefix(&check.terms, self.inputs, &self.binding)?;
            let refs = encoded_refs(&prefix);
            self.plan.summary.counters.direct_kernel_probes += 1;
            if !AccessSource::HashTrie(index.as_ref()).exists(&refs)? {
                return Ok(());
            }
            let relation = direct_relation(self.image, check.relation)?;
            let mut found = false;
            for row in index.rows_for_prefix(&refs) {
                if direct_row_satisfies_atom(
                    relation,
                    &self.plan.relation_atoms[check.atom_id],
                    row,
                    self.inputs,
                    &self.binding,
                )? {
                    found = true;
                    break;
                }
            }
            if !found {
                return Ok(());
            }
        }
        self.execute_step(0)
    }

    fn execute_step(&mut self, depth: usize) -> Result<()> {
        if depth == self.kernel.steps.len() {
            let before = self.plan.summary.counters.comparisons_evaluated;
            let keep = comparisons_ready_pass(
                self.txn,
                &self.plan.comparisons,
                self.query,
                self.inputs,
                &self.binding,
                &mut self.plan.summary.counters,
            )?;
            self.plan.summary.counters.direct_kernel_predicates = self
                .plan
                .summary
                .counters
                .direct_kernel_predicates
                .saturating_add(
                    self.plan
                        .summary
                        .counters
                        .comparisons_evaluated
                        .saturating_sub(before),
                );
            if keep {
                self.plan.summary.counters.bindings_yielded += 1;
                self.plan.summary.counters.bindings_completed += 1;
                self.plan.summary.counters.direct_chain_output_rows += 1;
                self.plan.summary.counters.direct_chain_output_values = self
                    .plan
                    .summary
                    .counters
                    .direct_chain_output_values
                    .saturating_add(self.query.find.len() as u64);
                self.plan.summary.counters.direct_binding_reuses += 1;
                let _span = tracing::trace_span!("bumbledb.query.sink.emit").entered();
                if !self.sink.emit_direct_project(
                    self.query,
                    &self.binding,
                    &mut self.plan.summary.counters,
                )? {
                    self.sink.emit(
                        self.txn,
                        self.query,
                        &self.binding,
                        &mut self.plan.summary.counters,
                    )?;
                }
            }
            return Ok(());
        }
        let step = &self.kernel.steps[depth];
        self.plan.summary.counters.direct_chain_steps += 1;
        if let Some(rows) =
            self.image_rows_for_terms(step.atom_id, &step.prefix_fields, &step.prefix_terms)?
        {
            for row in rows {
                self.plan.summary.counters.direct_chain_step_rows += 1;
                let atom = &self.plan.relation_atoms[step.atom_id];
                if !self.image_row_satisfies_atom(atom, &row)? {
                    continue;
                }
                let Some(value) = row.get(step.bind_field) else {
                    return Err(Error::internal("missing direct chain image bind field"));
                };
                let encoded = encoded_owned_for_width(
                    self.query.vars[step.bind_var].value_type.encoded_width(),
                    value.as_bytes(),
                )?;
                self.plan.summary.counters.direct_bind_attempts += 1;
                if !self.binding.bind(step.bind_var, encoded) {
                    continue;
                }
                self.plan.summary.counters.direct_bind_successes += 1;
                if let Some(rows) = self.plan.summary.node_rows.get_mut(depth) {
                    rows.actual_rows = rows.actual_rows.saturating_add(1);
                }
                self.execute_step(depth + 1)?;
                self.binding.unbind(step.bind_var);
            }
            return Ok(());
        }
        if let Some(rows) = self.storage_rows_for_terms(
            step.relation,
            step.atom_id,
            &step.prefix_fields,
            &step.prefix_terms,
        )? {
            for row in rows {
                self.plan.summary.counters.direct_chain_step_rows += 1;
                let atom = &self.plan.relation_atoms[step.atom_id];
                if !self.storage_row_satisfies_atom(atom, &row)? {
                    continue;
                }
                let Some(field_name) = atom
                    .fields
                    .iter()
                    .find(|field| field.field == step.bind_field)
                    .map(|field| field.field_name.as_str())
                else {
                    return Err(Error::internal("missing direct chain storage bind field"));
                };
                let value = row
                    .value(field_name)
                    .ok_or_else(|| Error::internal("missing direct chain storage row value"))?;
                let encoded = self
                    .txn
                    .encode_query_value(&self.query.vars[step.bind_var].value_type, value)?;
                let encoded = encoded_owned_for_width(
                    self.query.vars[step.bind_var].value_type.encoded_width(),
                    &encoded,
                )?;
                self.plan.summary.counters.direct_bind_attempts += 1;
                if !self.binding.bind(step.bind_var, encoded) {
                    continue;
                }
                self.plan.summary.counters.direct_bind_successes += 1;
                if let Some(rows) = self.plan.summary.node_rows.get_mut(depth) {
                    rows.actual_rows = rows.actual_rows.saturating_add(1);
                }
                self.execute_step(depth + 1)?;
                self.binding.unbind(step.bind_var);
            }
            return Ok(());
        }
        let index = direct_hash_index(
            self.image,
            step.relation,
            &step.prefix_fields,
            None,
            &step.index_name,
            &mut self.plan.summary.counters,
        )?;
        let prefix = direct_prefix(&step.prefix_terms, self.inputs, &self.binding)?;
        let refs = encoded_refs(&prefix);
        let row_count = index.count(&refs);
        self.plan.summary.counters.direct_kernel_probes += 1;
        if row_count == 0 {
            return Ok(());
        }
        let relation = direct_relation(self.image, step.relation)?;
        for row in index.rows_for_prefix(&refs) {
            self.plan.summary.counters.direct_kernel_rows += 1;
            self.plan.summary.counters.direct_chain_step_rows += 1;
            if !direct_row_satisfies_atom(
                relation,
                &self.plan.relation_atoms[step.atom_id],
                row,
                self.inputs,
                &self.binding,
            )? {
                continue;
            }
            let bytes = relation
                .encoded_bytes(row, step.bind_field)
                .ok_or_else(|| Error::internal("missing direct chain bind field"))?;
            let value = encoded_owned_for_width(
                self.query.vars[step.bind_var].value_type.encoded_width(),
                bytes,
            )?;
            self.plan.summary.counters.direct_bind_attempts += 1;
            if !self.binding.bind(step.bind_var, value) {
                continue;
            }
            self.plan.summary.counters.direct_bind_successes += 1;
            if let Some(rows) = self.plan.summary.node_rows.get_mut(depth) {
                rows.actual_rows = rows.actual_rows.saturating_add(1);
            }
            self.execute_step(depth + 1)?;
            self.binding.unbind(step.bind_var);
        }
        Ok(())
    }

    fn storage_rows_for_terms(
        &mut self,
        relation_id: crate::RelationId,
        atom_id: usize,
        fields: &[FieldId],
        terms: &[NormTerm],
    ) -> Result<Option<Vec<Row>>> {
        let atom = &self.plan.relation_atoms[atom_id];
        let Some(index_name) = direct_storage_index_for_fields(self.schema, atom, fields)? else {
            return Ok(None);
        };
        let mut values = Vec::new();
        for (field_id, term) in fields.iter().zip(terms) {
            let Some(atom_field) = atom.fields.iter().find(|field| field.field == *field_id) else {
                return Err(Error::internal("missing direct chain prefix field"));
            };
            let value = self.storage_term_value(term, &atom_field.value_type)?;
            values.push((atom_field.field_name.clone(), value));
        }
        let scan = self.txn.scan_prefix(
            self.schema,
            &atom.relation_name,
            &index_name,
            &FieldValues::new(&atom.relation_name, values),
        )?;
        let rows = scan
            .map(|item| item.map(|item| item.row))
            .collect::<Result<Vec<_>>>()?;
        self.plan.summary.counters.direct_kernel_probes += 1;
        self.plan.summary.counters.direct_kernel_rows = self
            .plan
            .summary
            .counters
            .direct_kernel_rows
            .saturating_add(rows.len() as u64);
        let _ = relation_id;
        Ok(Some(rows))
    }

    fn image_rows_for_terms(
        &mut self,
        atom_id: usize,
        fields: &[FieldId],
        terms: &[NormTerm],
    ) -> Result<Option<Vec<DirectImageRow>>> {
        let atom = &self.plan.relation_atoms[atom_id];
        let relation = direct_relation(self.image, atom.relation)?;
        let Some(index) = direct_image_index_for_fields(relation, atom, fields) else {
            return Ok(None);
        };
        let prefix = direct_prefix(terms, self.inputs, &self.binding)?;
        let prefix = prefix
            .iter()
            .flat_map(|value| value.as_bytes().iter().copied())
            .collect::<Vec<_>>();
        let mut rows = Vec::new();
        for entry in index.entries_with_prefix(&prefix) {
            let mut row = DirectImageRow {
                fields: SmallVec::new(),
            };
            for field in &atom.fields {
                let Some(bytes) = index.component_bytes(entry, field.field) else {
                    return Ok(None);
                };
                row.fields.push((
                    field.field,
                    encoded_owned_from_slice(&field.value_type, bytes)?,
                ));
            }
            rows.push(row);
        }
        self.plan.summary.counters.direct_kernel_probes += 1;
        self.plan.summary.counters.direct_kernel_rows = self
            .plan
            .summary
            .counters
            .direct_kernel_rows
            .saturating_add(rows.len() as u64);
        Ok(Some(rows))
    }

    fn image_row_satisfies_atom(&self, atom: &NormAtom, row: &DirectImageRow) -> Result<bool> {
        for field in &atom.fields {
            let Some(value) = row.get(field.field) else {
                return Ok(false);
            };
            let bytes = value.as_bytes();
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
                NormTerm::Literal(literal) => {
                    if literal.as_bytes() != bytes {
                        return Ok(false);
                    }
                }
                NormTerm::Wildcard => {}
            }
        }
        Ok(true)
    }

    fn storage_term_value(&self, term: &NormTerm, value_type: &ValueType) -> Result<Value> {
        match term {
            NormTerm::Var(variable) => {
                let value = self
                    .binding
                    .get(variable.0 as usize)
                    .ok_or_else(|| Error::internal("missing direct chain bound variable"))?;
                self.txn.decode_query_value(value_type, value.as_bytes())
            }
            NormTerm::Input(input) => {
                let value = self
                    .inputs
                    .get(*input)
                    .ok_or_else(|| Error::internal("missing direct chain input"))?;
                self.txn.decode_query_value(value_type, value.as_bytes())
            }
            NormTerm::Literal(literal) => {
                self.txn.decode_query_value(value_type, literal.as_bytes())
            }
            NormTerm::Wildcard => Err(Error::internal("wildcard cannot be a direct chain prefix")),
        }
    }

    fn storage_row_satisfies_atom(&self, atom: &NormAtom, row: &Row) -> Result<bool> {
        for field in &atom.fields {
            let Some(value) = row.value(&field.field_name) else {
                return Ok(false);
            };
            let encoded = self.txn.encode_query_value(&field.value_type, value)?;
            match &field.term {
                NormTerm::Var(variable) => {
                    if let Some(bound) = self.binding.get(variable.0 as usize)
                        && bound.as_bytes() != encoded.as_slice()
                    {
                        return Ok(false);
                    }
                }
                NormTerm::Input(input) => {
                    let Some(input) = self.inputs.get(*input) else {
                        return Ok(false);
                    };
                    if input.as_bytes() != encoded.as_slice() {
                        return Ok(false);
                    }
                }
                NormTerm::Literal(literal) => {
                    if literal.as_bytes() != encoded.as_slice() {
                        return Ok(false);
                    }
                }
                NormTerm::Wildcard => {}
            }
        }
        Ok(true)
    }
}

fn direct_storage_index_for_fields(
    schema: &StorageSchema,
    atom: &NormAtom,
    fields: &[FieldId],
) -> Result<Option<String>> {
    let field_names = fields
        .iter()
        .map(|field_id| {
            atom.fields
                .iter()
                .find(|field| field.field == *field_id)
                .map(|field| field.field_name.clone())
                .ok_or_else(|| Error::internal("missing direct chain field name"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(schema
        .access_paths(&atom.relation_name)?
        .into_iter()
        .find(|path| {
            path.leading_fields.len() >= field_names.len()
                && path
                    .leading_fields
                    .iter()
                    .zip(&field_names)
                    .all(|(leading, field)| leading == field)
        })
        .map(|path| path.index_name))
}

fn direct_image_index_for_fields<'a>(
    relation: &'a RelationImage,
    atom: &NormAtom,
    fields: &[FieldId],
) -> Option<&'a crate::query_image::RelationIndexImage> {
    relation.indexes().iter().find(|index| {
        index.fields.len() >= fields.len()
            && index
                .fields
                .iter()
                .zip(fields)
                .all(|(left, right)| left == right)
            && atom
                .fields
                .iter()
                .all(|field| index.contains_field(field.field))
    })
}

fn direct_hash_index(
    image: &crate::QueryImage,
    relation_id: crate::RelationId,
    fields: &[FieldId],
    access: Option<AccessId>,
    index_name: &str,
    counters: &mut PlanCounters,
) -> Result<Arc<HashTrieIndex>> {
    let relation = direct_relation(image, relation_id)?;
    let key = HashTrieKey::new(&image.key(), relation_id, access, fields, LeafMode::Rows);
    let cached = image.cached_hash_trie(key, || {
        crate::query_image::build_hash_trie_index(
            relation,
            IndexSpec::new(index_name, fields.iter().copied()),
        )
    })?;
    if !cached.hit {
        counters.hash_index_builds += 1;
        counters.hash_index_build_rows = counters
            .hash_index_build_rows
            .saturating_add(relation.row_count as u64);
    }
    Ok(cached.index)
}

fn direct_prefix(
    terms: &[NormTerm],
    inputs: &EncodedInputs,
    binding: &EncodedBinding,
) -> Result<SmallEncodedPrefix> {
    let mut prefix = SmallVec::new();
    for term in terms {
        match term {
            NormTerm::Input(input) => prefix.push(
                inputs
                    .get(*input)
                    .cloned()
                    .ok_or_else(|| Error::internal("missing direct input"))?,
            ),
            NormTerm::Literal(value) => prefix.push(value.clone()),
            NormTerm::Var(variable) => prefix.push(
                binding
                    .get(variable.0 as usize)
                    .cloned()
                    .ok_or_else(|| Error::internal("missing direct bound variable"))?,
            ),
            NormTerm::Wildcard => {
                return Err(Error::internal("wildcard cannot be a direct prefix"));
            }
        }
    }
    Ok(prefix)
}

fn direct_relation(
    image: &crate::QueryImage,
    relation_id: crate::RelationId,
) -> Result<&RelationImage> {
    image
        .relation_by_id(relation_id)
        .ok_or_else(|| Error::internal(format!("missing direct relation {}", relation_id.0)))
}

fn bind_atom_variables(
    relation: &RelationImage,
    atom: &NormAtom,
    row: RowId,
    binding: &mut EncodedBinding,
) -> Result<SmallParticipants> {
    let mut bound = SmallParticipants::new();
    for field in &atom.fields {
        let NormTerm::Var(variable) = field.term else {
            continue;
        };
        let variable = variable.0 as usize;
        let bytes = relation
            .encoded_bytes(row, field.field)
            .ok_or_else(|| Error::internal("missing direct variable field"))?;
        let value = encoded_owned_for_width(field.value_type.encoded_width(), bytes)?;
        if !binding.bind(variable, value) {
            continue;
        }
        if !bound.contains(&variable) {
            bound.push(variable);
        }
    }
    Ok(bound)
}

fn unbind_variables(binding: &mut EncodedBinding, variables: &[usize]) {
    for variable in variables {
        binding.unbind(*variable);
    }
}

fn direct_row_satisfies_atom(
    relation: &RelationImage,
    atom: &NormAtom,
    row: RowId,
    inputs: &EncodedInputs,
    binding: &EncodedBinding,
) -> Result<bool> {
    for field in &atom.fields {
        let bytes = relation
            .encoded_bytes(row, field.field)
            .ok_or_else(|| Error::internal("missing direct atom field"))?;
        match &field.term {
            NormTerm::Var(variable) => {
                if let Some(bound) = binding.get(variable.0 as usize)
                    && bound.as_bytes() != bytes
                {
                    return Ok(false);
                }
            }
            NormTerm::Input(input) => {
                let Some(input) = inputs.get(*input) else {
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
        if lftj_prefix_proves_empty(
            image,
            txn,
            query,
            inputs,
            &plan.relation_atoms,
            &plan.variable_order_ids,
            &mut plan.summary.counters,
        )? {
            None
        } else {
            Some(build_lftj_atom_plans(
                image,
                query,
                inputs,
                &plan.relation_atoms,
                &plan.variable_order_ids,
                &mut plan.summary.counters,
            )?)
        }
    };
    plan.summary.timings.lftj_build_micros = plan
        .summary
        .timings
        .lftj_build_micros
        .saturating_add(elapsed_micros(build_start));
    plan.summary.allocations.lftj_build = allocation_delta_since(build_alloc_start);
    let Some(atom_plans) = atom_plans else {
        return Ok(());
    };
    if atom_plans.iter().any(|atom| atom.row_count == 0) {
        return Ok(());
    }
    let runtime = LftjRuntime {
        participants_by_variable: lftj_participants_by_variable(query.vars.len(), &atom_plans),
        iters: atom_plans.iter().map(|atom| atom.source.iter()).collect(),
    };
    let execute_start = Instant::now();
    let execute_alloc_start = allocation::snapshot();
    let result = {
        let _span =
            tracing::debug_span!("bumbledb.query.lftj.execute", variables = query.vars.len())
                .entered();
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

fn lftj_prefix_proves_empty(
    image: &crate::QueryImage,
    txn: &ReadTxn<'_>,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atoms: &[NormAtom],
    variable_order_ids: &[usize],
    counters: &mut PlanCounters,
) -> Result<bool> {
    if query.predicates.is_empty()
        && !atoms.iter().any(|atom| {
            atom.fields
                .iter()
                .any(|field| matches!(field.term, NormTerm::Input(_) | NormTerm::Literal(_)))
        })
    {
        return Ok(false);
    }
    let max_depth = variable_order_ids.len().saturating_sub(1).min(3);
    for depth in 0..=max_depth {
        let prefix_vars = variable_order_ids
            .iter()
            .take(depth + 1)
            .copied()
            .collect::<BTreeSet<_>>();
        let prefix_atoms = atoms
            .iter()
            .filter(|atom| {
                let variables = atom_variables(atom);
                if depth == 0 {
                    variables
                        .iter()
                        .any(|variable| prefix_vars.contains(variable))
                } else {
                    !variables.is_empty()
                        && variables
                            .iter()
                            .all(|variable| prefix_vars.contains(variable))
                }
            })
            .cloned()
            .collect::<Vec<_>>();
        if prefix_atoms.is_empty() {
            continue;
        }
        let atom_plans = build_lftj_atom_plans(
            image,
            query,
            inputs,
            &prefix_atoms,
            variable_order_ids,
            counters,
        )?;
        if atom_plans.iter().any(|atom| atom.row_count == 0) {
            return Ok(true);
        }
        if !lftj_prefix_has_binding(txn, query, inputs, variable_order_ids, &atom_plans, depth)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn lftj_prefix_has_binding(
    txn: &ReadTxn<'_>,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    variable_order_ids: &[usize],
    atom_plans: &[LftjAtomPlan],
    max_depth: usize,
) -> Result<bool> {
    let participants_by_variable = lftj_participants_by_variable(query.vars.len(), atom_plans);
    let iters = atom_plans.iter().map(|atom| atom.source.iter()).collect();
    let mut probe = LftjPrefixProbe {
        txn,
        query,
        inputs,
        variable_order_ids,
        max_depth,
        participants_by_variable,
        iters,
        binding: EncodedBinding::new(query.vars.len()),
        counters: PlanCounters::default(),
    };
    probe.execute(0)
}

struct LftjPrefixProbe<'txn, 'input, 'query, 'image> {
    txn: &'input ReadTxn<'txn>,
    query: &'query NormalizedQuery,
    inputs: &'input EncodedInputs,
    variable_order_ids: &'input [usize],
    max_depth: usize,
    participants_by_variable: Vec<SmallParticipants>,
    iters: Vec<LftjTrieIter<'image>>,
    binding: EncodedBinding,
    counters: PlanCounters,
}

impl LftjPrefixProbe<'_, '_, '_, '_> {
    fn execute(&mut self, depth: usize) -> Result<bool> {
        if depth > self.max_depth {
            return Ok(true);
        }
        let variable = self.variable_order_ids[depth];
        let participants = self
            .participants_by_variable
            .get(variable)
            .cloned()
            .unwrap_or_default();
        if participants.is_empty() {
            return Ok(true);
        }

        for atom_id in &participants {
            self.iters[*atom_id].open();
        }
        let mut leapfrog = LeapfrogState::new(participants.clone());
        leapfrog.init(&mut self.iters, &mut self.counters)?;
        while !leapfrog.at_end {
            let value = leapfrog.key(&self.iters, &mut self.counters)?;
            if self.binding.bind(variable, value) {
                let keep = comparisons_ready_pass(
                    self.txn,
                    &self.query.predicates,
                    self.query,
                    self.inputs,
                    &self.binding,
                    &mut self.counters,
                )?;
                if keep && self.execute(depth + 1)? {
                    self.binding.unbind(variable);
                    for atom_id in participants.iter().rev() {
                        self.iters[*atom_id].up();
                    }
                    return Ok(true);
                }
                self.binding.unbind(variable);
            }
            leapfrog.next(&mut self.iters, &mut self.counters)?;
        }
        for atom_id in participants.iter().rev() {
            self.iters[*atom_id].up();
        }
        Ok(false)
    }
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
                self.plan.summary.counters.bindings_completed += 1;
                self.plan.summary.counters.lftj_completed_bindings += 1;
                let _span = tracing::trace_span!("bumbledb.query.sink.emit").entered();
                if !self.sink.emit_project_batch(
                    self.query,
                    &self.binding,
                    &mut self.plan.summary.counters,
                )? {
                    self.sink.emit(
                        self.txn,
                        self.query,
                        &self.binding,
                        &mut self.plan.summary.counters,
                    )?;
                }
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
            self.plan.summary.counters.lftj_open_calls += 1;
        }

        let mut leapfrog = LeapfrogState::new(participants.clone());
        leapfrog.init(&mut self.runtime.iters, &mut self.plan.summary.counters)?;
        while !leapfrog.at_end {
            let value = leapfrog.key(&self.runtime.iters, &mut self.plan.summary.counters)?;
            self.plan.summary.counters.variable_candidates += 1;
            self.plan.summary.counters.lftj_candidate_values += 1;
            if self.binding.bind(variable, value) {
                self.plan.summary.counters.lftj_bind_successes += 1;
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
            } else {
                self.plan.summary.counters.lftj_bind_rejects += 1;
            }
            leapfrog.next(&mut self.runtime.iters, &mut self.plan.summary.counters)?;
        }

        for atom_id in participants.iter().rev() {
            self.runtime.iters[*atom_id].up();
            self.plan.summary.counters.trie_up += 1;
            self.plan.summary.counters.lftj_up_calls += 1;
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

    fn init(&mut self, iters: &mut [LftjTrieIter<'_>], counters: &mut PlanCounters) -> Result<()> {
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
        iters: &[LftjTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<()> {
        let mut error = None;
        self.iter_ids.sort_by(|left, right| {
            if error.is_some() {
                return std::cmp::Ordering::Equal;
            }
            let Some(left) = key_ref_opt(&iters[*left], counters) else {
                error = Some(missing_trie_key_error());
                return std::cmp::Ordering::Equal;
            };
            let Some(right) = key_ref_opt(&iters[*right], counters) else {
                error = Some(missing_trie_key_error());
                return std::cmp::Ordering::Equal;
            };
            compare_encoded_ref(left, right)
        });
        if let Some(error) = error {
            return Err(error);
        }
        Ok(())
    }

    fn key(&self, iters: &[LftjTrieIter<'_>], counters: &mut PlanCounters) -> Result<EncodedOwned> {
        self.iter_ids
            .first()
            .map(|id| key_owned(&iters[*id], counters))
            .transpose()?
            .ok_or_else(|| Error::internal("leapfrog join has no iterators"))
    }

    fn next(&mut self, iters: &mut [LftjTrieIter<'_>], counters: &mut PlanCounters) -> Result<()> {
        if self.at_end {
            return Ok(());
        }
        let id = self.iter_ids[self.p];
        iters[id].next();
        counters.trie_next += 1;
        counters.lftj_next_calls += 1;
        if iters[id].at_end() {
            self.at_end = true;
            return Ok(());
        }
        self.p = (self.p + 1) % self.iter_ids.len();
        self.search(iters, counters)
    }

    fn search(
        &mut self,
        iters: &mut [LftjTrieIter<'_>],
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
            let Some(current) = key_ref_opt(&iters[id], counters) else {
                return Err(missing_trie_key_error());
            };
            if compare_encoded_ref_owned(current, &max) == std::cmp::Ordering::Equal {
                return Ok(());
            }
            iters[id].seek(max.as_ref());
            counters.trie_seek += 1;
            counters.lftj_seek_calls += 1;
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

fn key_owned(iter: &LftjTrieIter<'_>, counters: &mut PlanCounters) -> Result<EncodedOwned> {
    key_owned_opt(iter, counters).ok_or_else(missing_trie_key_error)
}

fn key_owned_opt(iter: &LftjTrieIter<'_>, counters: &mut PlanCounters) -> Option<EncodedOwned> {
    key_ref_opt(iter, counters).map(EncodedOwned::from_ref)
}

fn key_ref_opt<'a>(
    iter: &'a LftjTrieIter<'a>,
    counters: &mut PlanCounters,
) -> Option<crate::EncodedRef<'a>> {
    let key = iter.key()?;
    counters.trie_key_reads += 1;
    counters.lftj_key_reads += 1;
    Some(key)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EncodedWidth {
    W1,
    W8,
    W16,
}

fn encoded_width_for_len(len: usize) -> Option<EncodedWidth> {
    match len {
        1 => Some(EncodedWidth::W1),
        8 => Some(EncodedWidth::W8),
        16 => Some(EncodedWidth::W16),
        _ => None,
    }
}

fn compare_encoded_ref(
    left: crate::EncodedRef<'_>,
    right: crate::EncodedRef<'_>,
) -> std::cmp::Ordering {
    compare_encoded_bytes(left.as_bytes(), right.as_bytes())
}

fn compare_encoded_ref_owned(
    left: crate::EncodedRef<'_>,
    right: &EncodedOwned,
) -> std::cmp::Ordering {
    compare_encoded_bytes(left.as_bytes(), right.as_bytes())
}

fn compare_encoded_bytes(left: &[u8], right: &[u8]) -> std::cmp::Ordering {
    match (encoded_width_for_len(left.len()), left.len() == right.len()) {
        (Some(EncodedWidth::W1), true) => left[0].cmp(&right[0]),
        (Some(EncodedWidth::W8), true) => {
            let mut left_bytes = [0u8; 8];
            let mut right_bytes = [0u8; 8];
            left_bytes.copy_from_slice(left);
            right_bytes.copy_from_slice(right);
            let left = u64::from_be_bytes(left_bytes);
            let right = u64::from_be_bytes(right_bytes);
            left.cmp(&right)
        }
        (Some(EncodedWidth::W16), true) | (None, _) | (_, false) => left.cmp(right),
    }
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
        .relation_by_id(atom.relation)
        .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
    let variables = atom_variables_in_plan_order(atom, variable_order_ids);
    let cache_key = lftj_atom_cache_key(atom, &variables, variable_order_ids, inputs);
    let cached = image.cached_sorted_trie(cache_key, || {
        if let Some(build) =
            build_durable_lftj_sorted_trie(source, query, inputs, atom, &variables)?
        {
            Ok(build)
        } else {
            build_lftj_sorted_trie(source, query, inputs, atom, &variables)
        }
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
            .saturating_add(cached.source_rows_scanned);
        counters.atom_temp_relation_rows = counters
            .atom_temp_relation_rows
            .saturating_add(cached.index.stats.row_count as u64);
        counters.lftj_atom_source_rows_scanned = counters
            .lftj_atom_source_rows_scanned
            .saturating_add(cached.source_rows_scanned);
        counters.lftj_atom_rows_retained = counters
            .lftj_atom_rows_retained
            .saturating_add(cached.rows_retained);
        counters.lftj_atom_bytes_copied = counters
            .lftj_atom_bytes_copied
            .saturating_add(cached.bytes_copied);
        counters.lftj_atom_scan_micros = counters
            .lftj_atom_scan_micros
            .saturating_add(cached.scan_micros);
        counters.lftj_atom_column_micros = counters
            .lftj_atom_column_micros
            .saturating_add(cached.column_micros);
        counters.lftj_atom_sort_micros = counters
            .lftj_atom_sort_micros
            .saturating_add(cached.sort_micros);
    }
    Ok(LftjAtomPlan {
        variables,
        row_count: cached.index.stats.row_count,
        source: LftjAtomSource::SortedTrie(cached.index.clone()),
    })
}

fn atom_has_local_comparison(query: &NormalizedQuery, variables: &[usize]) -> bool {
    query.predicates.iter().any(|predicate| {
        predicate.operands.iter().any(|operand| {
            matches!(operand, NormOperand::Var(variable) if variables.contains(&(variable.0 as usize)))
        })
    })
}

fn build_durable_lftj_sorted_trie(
    source: &RelationImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
) -> Result<Option<SortedTrieBuild>> {
    if variables.is_empty() || atom_has_local_comparison(query, variables) {
        return Ok(None);
    }
    for index in source.indexes() {
        if !atom
            .fields
            .iter()
            .all(|field| index.contains_field(field.field))
        {
            continue;
        }
        let mut prefix = Vec::new();
        let mut cursor = 0usize;
        while let Some(field) = index.fields.get(cursor) {
            let Some(atom_field) = atom
                .fields
                .iter()
                .find(|atom_field| atom_field.field == *field)
            else {
                break;
            };
            match &atom_field.term {
                NormTerm::Input(input) => {
                    let Some(input) = inputs.get(*input) else {
                        return Err(Error::internal("missing normalized input"));
                    };
                    prefix.extend_from_slice(input.as_bytes());
                    cursor += 1;
                }
                NormTerm::Literal(literal) => {
                    prefix.extend_from_slice(literal.as_bytes());
                    cursor += 1;
                }
                NormTerm::Var(_) | NormTerm::Wildcard => break,
            }
        }
        let prefix_field_count = cursor;
        let mut fields = Vec::new();
        let mut eligible = true;
        for variable in variables {
            let Some(atom_field) = atom.fields.iter().find(
                |field| matches!(field.term, NormTerm::Var(id) if id.0 as usize == *variable),
            ) else {
                eligible = false;
                break;
            };
            if index.fields.get(cursor) != Some(&atom_field.field) {
                eligible = false;
                break;
            }
            fields.push(atom_field.field);
            cursor += 1;
        }
        if !eligible {
            continue;
        }
        if atom.fields.iter().any(|field| match &field.term {
            NormTerm::Input(_) | NormTerm::Literal(_) => {
                !index.fields[..prefix_field_count].contains(&field.field)
            }
            NormTerm::Var(variable) => !variables.contains(&(variable.0 as usize)),
            NormTerm::Wildcard => false,
        }) {
            continue;
        }
        return build_sorted_trie_from_relation_index(source.id, index, &prefix, &fields).map(Some);
    }
    Ok(None)
}

fn build_sorted_trie_from_relation_index(
    relation: crate::RelationId,
    index: &crate::query_image::RelationIndexImage,
    prefix: &[u8],
    fields: &[FieldId],
) -> Result<SortedTrieBuild> {
    let start = Instant::now();
    let range = index.prefix_range(prefix);
    let row_count = range.end.saturating_sub(range.start);
    let order = (0..row_count)
        .map(|row| RowId(row as u32))
        .collect::<Vec<_>>();
    let levels = durable_sorted_trie_levels(index, range.start, row_count, fields)?;
    let distinct_by_depth = levels
        .iter()
        .map(|level| level.keys.len())
        .collect::<Vec<_>>();
    let mut avg_fanout_by_depth = Vec::new();
    let mut max_fanout_by_depth = Vec::new();
    for level in &levels {
        let mut group_sizes = BTreeMap::<u32, usize>::new();
        for parent in &level.parent {
            *group_sizes.entry(*parent).or_insert(0) += 1;
        }
        let max = group_sizes.values().copied().max().unwrap_or(0);
        let avg = if group_sizes.is_empty() {
            0.0
        } else {
            group_sizes.values().sum::<usize>() as f64 / group_sizes.len() as f64
        };
        max_fanout_by_depth.push(max);
        avg_fanout_by_depth.push(avg);
    }
    let trie = SortedTrieIndex {
        relation,
        name: format!("durable_{}_lftj", index.access.0),
        fields: fields.to_vec(),
        order,
        levels,
        stats: crate::TrieStats {
            row_count,
            distinct_by_depth,
            avg_fanout_by_depth,
            max_fanout_by_depth,
            build_micros: start.elapsed().as_micros(),
        },
    };
    Ok(SortedTrieBuild {
        index: trie,
        source_rows_scanned: row_count as u64,
        rows_retained: row_count as u64,
        bytes_copied: 0,
        scan_micros: 0,
        column_micros: 0,
        sort_micros: start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
    })
}

fn durable_sorted_trie_levels(
    index: &crate::query_image::RelationIndexImage,
    base: usize,
    row_count: usize,
    fields: &[FieldId],
) -> Result<Vec<crate::TrieLevel>> {
    let mut levels = Vec::new();
    let mut parents = vec![(0usize, row_count, u32::MAX)];
    for field in fields {
        let mut level = crate::TrieLevel {
            field: *field,
            keys: Vec::new(),
            ranges: Vec::new(),
            parent: Vec::new(),
        };
        let mut next_parents = Vec::new();
        for (parent_start, parent_end, parent_index) in parents {
            let mut start = parent_start;
            while start < parent_end {
                let key = durable_index_component_owned(index, base + start, *field)?;
                let mut end = start + 1;
                while end < parent_end {
                    let next = durable_index_component_owned(index, base + end, *field)?;
                    if next != key {
                        break;
                    }
                    end += 1;
                }
                let entry_index = level.keys.len() as u32;
                level.keys.push(key);
                level.ranges.push(RowRange {
                    start: RowId(start as u32),
                    end: RowId(end as u32),
                });
                level.parent.push(parent_index);
                next_parents.push((start, end, entry_index));
                start = end;
            }
        }
        parents = next_parents;
        levels.push(level);
    }
    Ok(levels)
}

fn durable_index_component_owned(
    index: &crate::query_image::RelationIndexImage,
    position: usize,
    field: FieldId,
) -> Result<EncodedOwned> {
    let entry = index
        .entry_at(position)
        .ok_or_else(|| Error::internal("missing durable index entry"))?;
    let bytes = index
        .component_bytes(entry, field)
        .ok_or_else(|| Error::internal("missing durable index trie field"))?;
    encoded_owned_for_width(bytes.len(), bytes)
}

fn build_lftj_sorted_trie(
    source: &RelationImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
) -> Result<SortedTrieBuild> {
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
    let mut builders = encoded_column_builders(&fields, 0)?;
    let mut included_rows = 0usize;
    let source_rows_scanned;

    let mut bytes_copied = 0u64;
    let scan_start = Instant::now();
    {
        let _span = tracing::debug_span!("bumbledb.query.lftj.build.scan_filter_copy").entered();
        if let Some(indexed) =
            append_indexed_lftj_atom_values(&mut builders, source, query, inputs, atom, variables)?
        {
            source_rows_scanned = indexed.source_rows_scanned;
            included_rows = indexed.rows_retained as usize;
            bytes_copied = bytes_copied.saturating_add(indexed.bytes_appended);
        } else {
            source_rows_scanned = source.row_count as u64;
            for row in 0..source.row_count {
                let row = RowId(row as u32);
                let Some(slots) =
                    atom_row_value_slots(source, inputs, atom, row, query.vars.len())?
                else {
                    continue;
                };
                if !atom_local_comparisons_pass_slots(query, inputs, &slots)? {
                    continue;
                }
                included_rows += 1;
                bytes_copied = bytes_copied.saturating_add(append_atom_slots(
                    &mut builders,
                    &slots,
                    variables,
                )?);
            }
        }
    }
    let scan_micros = elapsed_micros(scan_start).min(u128::from(u64::MAX)) as u64;

    let row_count = if variables.is_empty() {
        included_rows
    } else {
        builders[0].len()
    };
    let encoded_column_bytes = builders
        .iter()
        .map(EncodedColumnBuilder::byte_len)
        .sum::<usize>();
    let column_start = Instant::now();
    let columns = {
        let _span = tracing::debug_span!("bumbledb.query.lftj.build.column_image").entered();
        finish_column_builders(builders)
    };
    let column_micros = elapsed_micros(column_start).min(u128::from(u64::MAX)) as u64;
    let relation = RelationImage {
        id: source.id,
        name: atom.relation_name.clone(),
        row_count,
        fields,
        columns,
        indexes: Vec::new(),
        sorted_index_count: 0,
        hash_index_count: 0,
        stats: RelationStats {
            row_count,
            field_count: variables.len(),
            encoded_column_bytes,
        },
    };
    let sort_start = Instant::now();
    let trie = {
        let _span = tracing::debug_span!("bumbledb.query.lftj.build.sorted_trie").entered();
        crate::query_image::build_sorted_trie_index(
            &relation,
            IndexSpec::new(
                format!("{}_lftj", atom.relation_name),
                (0..variables.len()).map(|id| FieldId(id as u16)),
            ),
        )?
    };
    let sort_micros = elapsed_micros(sort_start).min(u128::from(u64::MAX)) as u64;
    Ok(SortedTrieBuild {
        index: trie,
        source_rows_scanned,
        rows_retained: row_count as u64,
        bytes_copied,
        scan_micros,
        column_micros,
        sort_micros,
    })
}

struct IndexedPrefixAppendStats {
    source_rows_scanned: u64,
    rows_retained: u64,
    bytes_appended: u64,
}

type AtomValueSlots = SmallVec<[Option<EncodedOwned>; 8]>;

fn append_indexed_lftj_atom_values(
    builders: &mut [EncodedColumnBuilder],
    source: &RelationImage,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    variables: &[usize],
) -> Result<Option<IndexedPrefixAppendStats>> {
    let mut best = None;
    for index in source.indexes() {
        if !atom
            .fields
            .iter()
            .all(|field| index.contains_field(field.field))
        {
            continue;
        }
        let mut prefix = Vec::new();
        let mut prefix_fields = 0usize;
        for field in &index.fields {
            let Some(atom_field) = atom
                .fields
                .iter()
                .find(|atom_field| atom_field.field == *field)
            else {
                break;
            };
            let expected = match &atom_field.term {
                NormTerm::Input(input) => inputs.get(*input),
                NormTerm::Literal(literal) => Some(literal),
                NormTerm::Var(_) | NormTerm::Wildcard => None,
            };
            let Some(expected) = expected else {
                break;
            };
            prefix.extend_from_slice(expected.as_bytes());
            prefix_fields += 1;
        }
        if prefix_fields == 0 {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|(fields, _, _): &(usize, Vec<u8>, usize)| prefix_fields > *fields)
        {
            best = Some((prefix_fields, prefix, index.access.0 as usize));
        }
    }
    let Some((_, prefix, access)) = best else {
        return Ok(None);
    };
    let index = source
        .indexes()
        .iter()
        .find(|index| index.access.0 as usize == access)
        .ok_or_else(|| Error::internal("missing selected LFTJ atom index"))?;
    let mut source_rows_scanned = 0u64;
    let mut rows_retained = 0u64;
    let mut bytes_appended = 0u64;
    let _span = tracing::trace_span!(
        "bumbledb.query.lftj_atom.indexed_prefix",
        relation = %source.name,
        prefix_bytes = prefix.len()
    )
    .entered();
    for entry in index.entries_with_prefix(&prefix) {
        source_rows_scanned += 1;
        if let Some(slots) =
            atom_index_entry_value_slots(index, inputs, atom, entry, query.vars.len())?
            && atom_local_comparisons_pass_slots(query, inputs, &slots)?
        {
            rows_retained += 1;
            bytes_appended =
                bytes_appended.saturating_add(append_atom_slots(builders, &slots, variables)?);
        }
    }
    Ok(Some(IndexedPrefixAppendStats {
        source_rows_scanned,
        rows_retained,
        bytes_appended,
    }))
}

fn atom_index_entry_value_slots(
    index: &crate::query_image::RelationIndexImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    entry: &[u8],
    variable_count: usize,
) -> Result<Option<AtomValueSlots>> {
    let mut slots = empty_atom_slots(variable_count);
    for field in &atom.fields {
        let bytes = index
            .component_bytes(entry, field.field)
            .ok_or_else(|| Error::internal("missing atom field in relation index image"))?;
        match &field.term {
            NormTerm::Var(variable) => {
                let variable = variable.0 as usize;
                if !bind_atom_slot(&mut slots, variable, &field.value_type, bytes)? {
                    return Ok(None);
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
    Ok(Some(slots))
}

fn empty_atom_slots(variable_count: usize) -> AtomValueSlots {
    std::iter::repeat_with(|| None)
        .take(variable_count)
        .collect()
}

fn bind_atom_slot(
    slots: &mut AtomValueSlots,
    variable: usize,
    value_type: &ValueType,
    bytes: &[u8],
) -> Result<bool> {
    let slot = slots
        .get_mut(variable)
        .ok_or_else(|| Error::internal("atom variable id out of bounds"))?;
    if let Some(existing) = slot {
        return Ok(existing.as_bytes() == bytes);
    }
    *slot = Some(encoded_owned_for_width(value_type.encoded_width(), bytes)?);
    Ok(true)
}

fn append_atom_slots(
    builders: &mut [EncodedColumnBuilder],
    slots: &AtomValueSlots,
    variables: &[usize],
) -> Result<u64> {
    let mut bytes_appended = 0u64;
    for (column, variable) in variables.iter().enumerate() {
        let value = slots
            .get(*variable)
            .and_then(Option::as_ref)
            .ok_or_else(|| Error::internal("missing LFTJ variable value"))?;
        builders
            .get_mut(column)
            .ok_or_else(|| Error::internal("missing LFTJ column builder"))?
            .append_encoded_owned(value)?;
        bytes_appended = bytes_appended.saturating_add(value.as_bytes().len() as u64);
    }
    Ok(bytes_appended)
}

fn atom_local_comparisons_pass_slots(
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    slots: &AtomValueSlots,
) -> Result<bool> {
    for predicate in &query.predicates {
        let mut saw_local_variable = false;
        let mut encoded: [Option<&[u8]>; 2] = [None, None];
        for (index, operand) in predicate.operands.iter().enumerate() {
            let Some(out) = encoded.get_mut(index) else {
                return Err(Error::internal("comparison operand index out of bounds"));
            };
            *out = match operand {
                NormOperand::Var(variable) => {
                    let Some(value) = slots.get(variable.0 as usize).and_then(Option::as_ref)
                    else {
                        break;
                    };
                    saw_local_variable = true;
                    Some(value.as_bytes())
                }
                NormOperand::Input(input) => {
                    let Some(input) = inputs.get(*input) else {
                        break;
                    };
                    Some(input.as_bytes())
                }
                NormOperand::Literal(literal) => Some(literal.as_bytes()),
            };
        }
        let [Some(left), Some(right)] = encoded else {
            continue;
        };
        if !saw_local_variable {
            continue;
        }
        if encoded_comparison_supported(predicate.op, &predicate.value_type)
            && !compare_encoded_values(left, predicate.op, right)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn lftj_atom_cache_key(
    atom: &NormAtom,
    variables: &[usize],
    _variable_order_ids: &[usize],
    inputs: &EncodedInputs,
) -> LftjAtomKey {
    let mut hasher = blake3::Hasher::new();
    hash_bytes_len_prefixed(&mut hasher, b"bumbledb.lftj_atom.v1");
    hash_u16(&mut hasher, atom.relation.0);
    hash_u64(&mut hasher, variables.len() as u64);
    for variable in variables {
        let field = atom
            .fields
            .iter()
            .find(|field| matches!(field.term, NormTerm::Var(id) if id.0 as usize == *variable))
            .map(|field| field.field.0)
            .unwrap_or(u16::MAX);
        hash_u16(&mut hasher, field);
    }
    hash_u64(&mut hasher, atom.fields.len() as u64);
    for field in &atom.fields {
        hash_u16(&mut hasher, field.field.0);
        hash_value_type(&mut hasher, &field.value_type);
        match &field.term {
            NormTerm::Var(variable) => {
                hash_u8(&mut hasher, 1);
                let ordinal = variables
                    .iter()
                    .position(|candidate| *candidate == variable.0 as usize)
                    .unwrap_or(usize::MAX);
                hash_u64(&mut hasher, ordinal as u64);
            }
            NormTerm::Input(input) => {
                hash_u8(&mut hasher, 2);
                hash_u16(&mut hasher, input.0);
                if let Some(value) = inputs.get(*input) {
                    hash_encoded_owned(&mut hasher, value);
                } else {
                    hash_u8(&mut hasher, 0);
                }
            }
            NormTerm::Literal(value) => {
                hash_u8(&mut hasher, 3);
                hash_encoded_owned(&mut hasher, value);
            }
            NormTerm::Wildcard => hash_u8(&mut hasher, 4),
        }
    }
    LftjAtomKey(*hasher.finalize().as_bytes())
}

fn atom_variables_in_plan_order(atom: &NormAtom, variable_order_ids: &[usize]) -> Vec<usize> {
    variable_order_ids
        .iter()
        .copied()
        .filter(|variable| atom_contains_variable(atom, *variable))
        .collect()
}

fn atom_row_value_slots(
    relation: &RelationImage,
    inputs: &EncodedInputs,
    atom: &NormAtom,
    row: RowId,
    variable_count: usize,
) -> Result<Option<AtomValueSlots>> {
    let mut slots = empty_atom_slots(variable_count);
    for field in &atom.fields {
        let bytes = relation
            .encoded_bytes(row, field.field)
            .ok_or_else(|| Error::internal("missing atom field in relation image"))?;
        match &field.term {
            NormTerm::Var(variable) => {
                let variable = variable.0 as usize;
                if !bind_atom_slot(&mut slots, variable, &field.value_type, bytes)? {
                    return Ok(None);
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
    Ok(Some(slots))
}

fn plan_query(
    schema: &StorageSchema,
    query: &mut NormalizedQuery,
    image: &crate::QueryImage,
    query_image_cache: QueryImageCacheDiagnostics,
    prepared_plan_cache: PreparedPlanCacheDiagnostics,
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
    let mut execution_plan = ExecutionPlan {
        variable_order_ids,
        relation_atoms: query.atoms.clone(),
        comparisons: query.predicates.clone(),
        direct_kernel: None,
        summary: QueryPlan {
            variable_order,
            variable_estimates,
            missing_indexes,
            optimizer,
            plan_family: PlanFamily::FreeJoinLftj,
            query_image_cache,
            planner_stats,
            prepared_plan_cache,
            node_rows,
            node_timings,
            free_join,
            direct_kernel: None,
            runtime_kind: QueryRuntimeKind::Unknown,
            timings: QueryTimings::default(),
            allocations: QueryAllocationStats::default(),
            counters: PlanCounters::default(),
            uses_indexed_multiway_join,
        },
    };
    if let Some(direct_kernel) = try_direct_kernel(query) {
        execution_plan.summary.plan_family = match direct_kernel.kind {
            DirectKernel::ChainProbe(_) => PlanFamily::IndexNestedLoop,
            DirectKernel::PrefixRange(_) => PlanFamily::Direct,
        };
        execution_plan.summary.direct_kernel = Some(direct_kernel.summary.clone());
        execution_plan.direct_kernel = Some(direct_kernel);
    } else {
        execution_plan.summary.plan_family =
            plan_family_for_chosen(&execution_plan.summary.optimizer.chosen);
    }
    Ok(execution_plan)
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

fn plan_family_for_chosen(chosen: &str) -> PlanFamily {
    match chosen {
        "index_nested_loop" => PlanFamily::IndexNestedLoop,
        "pure_lftj" | "aggregate_pushdown" => PlanFamily::FreeJoinLftj,
        "direct_storage" => PlanFamily::Direct,
        "static_empty" => PlanFamily::StaticEmpty,
        _ => PlanFamily::Unknown,
    }
}

fn choose_variable_order(
    schema: &StorageSchema,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
    comparisons: &[&NormPredicate],
    stats: &PlannerStats,
) -> Result<(Vec<usize>, Vec<VariableCost>)> {
    let mut remaining = vec![true; query.vars.len()];
    let mut remaining_count = query.vars.len();
    let mut bound = BTreeSet::new();
    let mut order = Vec::with_capacity(query.vars.len());
    let mut costs = Vec::with_capacity(query.vars.len());

    while remaining_count != 0 {
        let mut best = None;
        for (variable, is_remaining) in remaining.iter().copied().enumerate() {
            if !is_remaining {
                continue;
            }
            let cost = estimate_variable_cost(schema, atoms, comparisons, stats, &bound, variable)?;
            if best.as_ref().is_none_or(|best: &VariableCost| {
                variable_cost_order_key(&cost, query) < variable_cost_order_key(best, query)
            }) {
                best = Some(cost);
            }
        }
        let best = best.ok_or_else(|| Error::internal("query has no remaining variables"))?;
        remaining[best.variable] = false;
        remaining_count -= 1;
        bound.insert(best.variable);
        order.push(best.variable);
        costs.push(best);
    }

    Ok((order, costs))
}

type VariableCostOrderKey<'a> = (
    u64,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<usize>,
    std::cmp::Reverse<usize>,
    &'a str,
);

fn variable_cost_order_key<'a>(
    cost: &'a VariableCost,
    query: &'a NormalizedQuery,
) -> VariableCostOrderKey<'a> {
    (
        cost.estimated_candidates,
        std::cmp::Reverse(cost.static_constraints),
        std::cmp::Reverse(cost.bound_constraints),
        std::cmp::Reverse(cost.relation_constraints),
        std::cmp::Reverse(cost.degree),
        query.vars[cost.variable].name.as_str(),
    )
}

fn estimate_variable_cost(
    schema: &StorageSchema,
    atoms: &[&NormAtom],
    comparisons: &[&NormPredicate],
    stats: &PlannerStats,
    bound: &BTreeSet<usize>,
    variable: usize,
) -> Result<VariableCost> {
    let mut has_constrained_stream = false;
    let mut has_unconstrained_payload_stream = false;
    for atom in atoms
        .iter()
        .copied()
        .filter(|atom| atom_contains_variable(atom, variable))
    {
        let relation_constraints = atom_bound_constraint_count(atom, variable, bound);
        let static_constraints = atom_static_constraint_count(atom, variable)
            + comparison_static_constraint_count(comparisons, variable, bound);
        let has_unbound_other = atom_has_unbound_other_variable_id(atom, variable, bound);
        let strength = relation_constraints + static_constraints;
        has_constrained_stream |= strength > 0;
        has_unconstrained_payload_stream |= strength == 0 && has_unbound_other;
    }
    let mut best_access: Option<AccessEstimate> = None;
    let mut relation_constraints = 0usize;
    let mut static_constraints = comparison_static_constraint_count(comparisons, variable, bound);
    let mut bound_constraints = comparison_bound_constraint_count(comparisons, variable, bound);

    for atom in atoms
        .iter()
        .copied()
        .filter(|atom| atom_contains_variable(atom, variable))
    {
        let strength = atom_bound_constraint_count(atom, variable, bound)
            + atom_static_constraint_count(atom, variable)
            + comparison_static_constraint_count(comparisons, variable, bound);
        let has_unbound_other = atom_has_unbound_other_variable_id(atom, variable, bound);
        relation_constraints += 1;
        static_constraints += atom_static_constraint_count(atom, variable);
        bound_constraints += atom_bound_constraint_count(atom, variable, bound);
        if has_constrained_stream && strength == 0 && has_unbound_other {
            continue;
        }
        let estimate = estimate_atom_variable_access(schema, stats, bound, atom, variable)?;
        if best_access.as_ref().is_none_or(|best| {
            (
                estimate.estimated_rows,
                std::cmp::Reverse(estimate.prefix_len),
                std::cmp::Reverse(estimate.current_is_next),
                estimate.access_label(),
            ) < (
                best.estimated_rows,
                std::cmp::Reverse(best.prefix_len),
                std::cmp::Reverse(best.current_is_next),
                best.access_label(),
            )
        }) {
            best_access = Some(estimate);
        }
    }

    let degree = atoms
        .iter()
        .filter(|atom| atom_contains_variable(atom, variable))
        .count();
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
                if path.kind == IndexKind::Range {
                    relation_rows.max(1).div_ceil(4)
                } else {
                    index_stats
                        .distinct_by_depth
                        .first()
                        .copied()
                        .unwrap_or(index_stats.rows)
                        .max(1) as u64
                }
            } else {
                index_stats.fanout_after_prefix(prefix_len)
            }
        } else {
            index_stats.estimated_rows_for_prefix(prefix_len)
        };
        if matches!(path.kind, IndexKind::TupleSet | IndexKind::Unique)
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
    for primary in first_unique_fields(relation) {
        if !fields.iter().any(|field| field == primary) {
            fields.push(primary.clone());
        }
    }
    fields
}

fn first_unique_fields(relation: &bumbledb_core::schema::RelationDescriptor) -> &[String] {
    relation
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            bumbledb_core::schema::ConstraintDescriptor::Unique { fields, .. } => {
                Some(fields.as_slice())
            }
            bumbledb_core::schema::ConstraintDescriptor::ForeignKey { .. } => None,
        })
        .unwrap_or(&[])
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
        variable_costs,
        stats,
        lftj_impls,
        cyclic,
    )?);

    if has_aggregate(query) {
        candidates.push(build_plan_candidate(
            "aggregate_pushdown",
            schema,
            query,
            atoms,
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
    let chosen_candidate = candidates
        .iter()
        .find(|candidate| candidate.name == chosen)
        .ok_or_else(|| Error::internal("chosen optimizer candidate missing"))?;
    let plan = build_free_join_plan(
        schema,
        query,
        atoms,
        variable_order_ids,
        &chosen_candidate.implementations,
        stats,
        chosen_candidate.estimates.clone(),
    )?;
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
            family: candidate.family,
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
    family: PlanFamily,
    cost: CostKey,
    estimates: PlanEstimates,
}

#[expect(
    clippy::too_many_arguments,
    reason = "optimizer candidate builder mirrors the full planning context"
)]
fn build_plan_candidate(
    name: &str,
    _schema: &StorageSchema,
    query: &NormalizedQuery,
    atoms: &[&NormAtom],
    variable_costs: &[VariableCost],
    stats: &PlannerStats,
    implementations: Vec<NodeImpl>,
    cyclic: bool,
) -> Result<OptimizerCandidate> {
    let estimates = estimate_free_join_plan(name, query, atoms, variable_costs, stats, cyclic);
    let cost = CostKey {
        estimated_micros: estimates
            .iterator_ops
            .saturating_add(estimates.hash_build_rows / HASH_BUILD_ROWS_PER_MICRO)
            .saturating_add(estimates.materialized_values),
        setup_micros: estimated_setup_micros(name, &estimates),
        memory_bytes: estimates.memory_bytes,
        materialization_penalty: estimates.materialized_values,
        candidate_rank: candidate_rank(name),
        implementation_mask: implementation_mask(&implementations),
    };
    Ok(OptimizerCandidate {
        name: name.to_owned(),
        implementations,
        family: plan_family_for_chosen(name),
        cost,
        estimates,
    })
}

fn candidate_rank(name: &str) -> u8 {
    match name {
        "pure_lftj" => 0,
        "aggregate_pushdown" => 1,
        _ => u8::MAX,
    }
}

fn implementation_mask(implementations: &[NodeImpl]) -> u64 {
    implementations
        .iter()
        .take(16)
        .enumerate()
        .fold(0u64, |mask, (index, implementation)| {
            let code = match implementation {
                NodeImpl::SortedLeapfrog => 1,
            };
            mask | ((code as u64) << (index * 4))
        })
}

fn estimated_setup_micros(name: &str, estimates: &PlanEstimates) -> u64 {
    let query_image_cost = estimates.output_rows.clamp(1, 1_000);
    let hash_cost = estimates.hash_build_rows / HASH_BUILD_ROWS_PER_MICRO;
    let sorted_cost = if name == "pure_lftj" || name == "aggregate_pushdown" {
        estimates.iterator_ops / 10
    } else {
        0
    };
    query_image_cost
        .saturating_add(hash_cost)
        .saturating_add(sorted_cost)
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
    atoms: &[&NormAtom],
    variable_costs: &[VariableCost],
    stats: &PlannerStats,
    cyclic: bool,
) -> PlanEstimates {
    let mut iterator_ops = 0u64;
    let mut hash_build_rows = 0u64;
    for cost in variable_costs {
        let variable_ops =
            cost.estimated_candidates
                .max(1)
                .saturating_mul(if cyclic { 1 } else { 3 });
        iterator_ops = iterator_ops.saturating_add(variable_ops);
    }
    for atom in atoms {
        if atom_variables(atom).is_empty() {
            hash_build_rows =
                hash_build_rows.saturating_add(stats.relation_rows(&atom.relation_name));
        }
    }

    if cyclic && name != "pure_lftj" && name != "aggregate_pushdown" {
        iterator_ops = iterator_ops.saturating_mul(8);
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
            NormFindTerm::Aggregate {
                variable, domain, ..
            } => {
                aggregate_vars.push(*variable);
                aggregate_vars.extend(domain.iter().copied());
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
                    domain,
                    value_type,
                } => aggregates.push(AggregateTerm {
                    function: *function,
                    var: *variable,
                    domain_vars: domain.clone(),
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
        if !compare_values(&left, comparison.op, &right) {
            counters.comparisons_failed += 1;
            return Ok(false);
        }
    }
    Ok(true)
}

fn operand_encoded_value(
    operand: &NormOperand,
    _value_type: &ValueType,
    inputs: &EncodedInputs,
    binding: &EncodedBinding,
) -> Option<EncodedOwned> {
    match operand {
        NormOperand::Var(variable) => binding.get(variable.0 as usize).cloned(),
        NormOperand::Input(input) => inputs.get(*input).cloned(),
        NormOperand::Literal(literal) => Some(literal.clone()),
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
    schema: &StorageSchema,
    query: &'a TypedQuery,
    inputs: &'a InputBindings,
    input: usize,
) -> Result<&'a Value> {
    let input = &query.inputs[input];
    let value = inputs
        .get(&input.name)
        .ok_or_else(|| Error::missing_input(&input.name))?;
    if !value_matches_type(schema, value, &input.value_type) {
        return Err(Error::query_input_type_mismatch(
            &input.name,
            value_type_name(&input.value_type),
            value.kind_name(),
        ));
    }
    Ok(value)
}

fn validate_inputs(
    schema: &StorageSchema,
    query: &TypedQuery,
    inputs: &InputBindings,
) -> Result<()> {
    for input in &query.inputs {
        input_value(schema, query, inputs, input.id)?;
    }
    Ok(())
}

fn value_matches_type(schema: &StorageSchema, value: &Value, value_type: &ValueType) -> bool {
    if let (Value::Enum(code), ValueType::Enum { name }) = (value, value_type) {
        return schema.descriptor().enum_contains_code(name, *code);
    }
    matches!(
        (value, value_type),
        (Value::Bool(_), ValueType::Bool)
            | (Value::U64(_), ValueType::U64)
            | (Value::I64(_), ValueType::I64)
            | (Value::Serial(_), ValueType::Serial { .. })
            | (Value::Timestamp(_), ValueType::TimestampMicros)
            | (Value::Decimal(_), ValueType::Decimal { .. })
            | (Value::Enum(_), ValueType::Enum { .. })
            | (Value::String(_), ValueType::String)
            | (Value::Bytes(_), ValueType::Bytes)
    )
}

fn literal_to_value(literal: &TypedLiteral) -> Result<Value> {
    let value = match (&literal.literal, &literal.value_type) {
        (Literal::Bool(value), ValueType::Bool) => Value::Bool(*value),
        (Literal::String(value), ValueType::String) => Value::String(value.clone()),
        (Literal::Integer(value), ValueType::U64) => Value::U64(*value as u64),
        (Literal::Integer(value), ValueType::I64) => Value::I64(*value as i64),
        (Literal::Integer(value), ValueType::Serial { .. }) => Value::Serial(*value as u64),
        (Literal::Integer(value), ValueType::Enum { .. }) => Value::Enum(*value as u8),
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
                domain,
                value_type,
            } => NormFindTerm::Aggregate {
                function: *function,
                variable: VarId(*variable as u16),
                domain: domain
                    .iter()
                    .map(|variable| VarId(*variable as u16))
                    .collect(),
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
            NormOperand::Literal(encode_owned_value(txn, value_type, &value)?)
        }
    })
}

fn encode_literal(txn: &ReadTxn<'_>, literal: &TypedLiteral) -> Result<EncodedOwned> {
    let value = literal_to_value(literal)?;
    encode_owned_value(txn, &literal.value_type, &value)
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
    encoded_owned_from_slice(value_type, &bytes)
}

fn encoded_owned_from_slice(value_type: &ValueType, bytes: &[u8]) -> Result<EncodedOwned> {
    match value_type.encoded_width() {
        1 => Ok(EncodedOwned::One(exact_encoded_array::<1>(bytes)?)),
        8 => Ok(EncodedOwned::Eight(exact_encoded_array::<8>(bytes)?)),
        16 => Ok(EncodedOwned::Sixteen(exact_encoded_array::<16>(bytes)?)),
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
    schema: &StorageSchema,
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
            if !value_matches_type(schema, value, &input.value_type) {
                return Err(Error::query_input_type_mismatch(
                    &input.name,
                    value_type_name(&input.value_type),
                    value.kind_name(),
                ));
            }
            encode_owned_value(txn, &input.value_type, value)
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

    fn emit_direct_project(
        &mut self,
        _query: &NormalizedQuery,
        _binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<bool> {
        counters.direct_batch_fallback_rows += 1;
        Ok(false)
    }

    fn emit_project_batch(
        &mut self,
        _query: &NormalizedQuery,
        _binding: &EncodedBinding,
        _counters: &mut PlanCounters,
    ) -> Result<bool> {
        Ok(false)
    }

    fn finish(
        self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>>
    where
        Self: Sized;
}

// Output sinks own projection, aggregation, count-only, and dedup materialization.
#[derive(Clone, Debug)]
enum OutputSink {
    CountRows(CountRowsSink),
    Project(EncodedProjectSink),
    Aggregate(AggregateSink),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SinkMode {
    Materialize,
    CountRowsOnly,
}

impl OutputSink {
    fn new(output: &OutputPlan) -> Self {
        Self::new_with_mode(output, SinkMode::Materialize)
    }

    fn new_count_rows(output: &OutputPlan) -> Self {
        Self::new_with_mode(output, SinkMode::CountRowsOnly)
    }

    fn new_with_mode(output: &OutputPlan, mode: SinkMode) -> Self {
        if mode == SinkMode::CountRowsOnly {
            return OutputSink::CountRows(CountRowsSink::new(output));
        }
        match output {
            OutputPlan::Project(plan) => OutputSink::Project(EncodedProjectSink::new(plan)),
            OutputPlan::Aggregate(plan) => OutputSink::Aggregate(AggregateSink::new(plan)),
        }
    }

    fn finish_count(self) -> Result<usize> {
        let OutputSink::CountRows(sink) = self else {
            return Err(Error::internal(
                "count rows requested from materializing sink",
            ));
        };
        Ok(sink.finish_count())
    }

    fn emit_direct_project(
        &mut self,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<bool> {
        let OutputSink::Project(sink) = self else {
            counters.direct_batch_fallback_rows += 1;
            return Ok(false);
        };
        let row_width = sink.push_binding(query, binding, counters)?;
        counters.direct_batch_rows += 1;
        counters.direct_batch_row_bytes = counters.direct_batch_row_bytes.saturating_add(row_width);
        Ok(true)
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
        counters.sink_emit_calls += 1;
        match self {
            OutputSink::CountRows(sink) => sink.emit(txn, query, binding, counters),
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
            OutputSink::CountRows(sink) => sink.finish(txn, query, counters),
            OutputSink::Project(sink) => sink.finish(txn, query, counters),
            OutputSink::Aggregate(sink) => sink.finish(txn, query, counters),
        }
    }

    fn emit_direct_project(
        &mut self,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<bool> {
        let OutputSink::Project(sink) = self else {
            counters.direct_batch_fallback_rows += 1;
            return Ok(false);
        };
        let row_width = sink.push_binding(query, binding, counters)?;
        counters.direct_batch_rows += 1;
        counters.direct_batch_row_bytes = counters.direct_batch_row_bytes.saturating_add(row_width);
        Ok(true)
    }

    fn emit_project_batch(
        &mut self,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<bool> {
        let OutputSink::Project(sink) = self else {
            return Ok(false);
        };
        sink.push_binding(query, binding, counters)?;
        Ok(true)
    }
}

fn is_global_count_plan(plan: &AggregatePlan) -> bool {
    plan.group_vars.is_empty()
        && plan.aggregates.len() == 1
        && matches!(
            plan.aggregates[0].function,
            AggregateFunction::CountDomain | AggregateFunction::CountDistinct
        )
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

    fn push_binding(
        &mut self,
        _query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<u64> {
        let mut row = SmallEncodedRow::new();
        let mut row_width = 0u64;
        for variable in &self.vars {
            let value = bound_encoded_variable(binding, variable.0 as usize)?;
            row_width = row_width.saturating_add(value.as_bytes().len() as u64);
            row.push(value.clone());
        }
        counters.encoded_project_rows_seen += 1;
        if self.rows.insert(row) {
            counters.encoded_project_rows_inserted =
                counters.encoded_project_rows_inserted.saturating_add(1);
            counters.encoded_project_row_bytes =
                counters.encoded_project_row_bytes.saturating_add(row_width);
            return Ok(row_width);
        }
        Ok(0)
    }
}

impl TupleSink for EncodedProjectSink {
    fn emit(
        &mut self,
        _txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<()> {
        self.push_binding(query, binding, counters).map(|_| ())
    }

    fn finish(
        self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>> {
        let EncodedProjectSink { vars, rows } = self;
        let _span = tracing::debug_span!("bumbledb.query.project", rows = rows.len(),).entered();
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        rows.into_iter()
            .map(|row| {
                vars.iter()
                    .zip(row)
                    .map(|(variable, value)| {
                        counters.project_decode_values += 1;
                        decode_output_value(
                            txn,
                            &query.vars[variable.0 as usize].value_type,
                            value,
                            counters,
                        )
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct CountRowsSink {
    output: OutputPlan,
    global_count: u64,
    project_rows: BTreeSet<SmallEncodedRow>,
    aggregate_groups: BTreeSet<SmallEncodedRow>,
}

impl CountRowsSink {
    fn new(output: &OutputPlan) -> Self {
        Self {
            output: output.clone(),
            global_count: 0,
            project_rows: BTreeSet::new(),
            aggregate_groups: BTreeSet::new(),
        }
    }

    fn finish_count(self) -> usize {
        match self.output {
            OutputPlan::Project(_) => self.project_rows.len(),
            OutputPlan::Aggregate(plan) if is_global_count_plan(&plan) => 1,
            OutputPlan::Aggregate(_) => self.aggregate_groups.len(),
        }
    }
}

impl TupleSink for CountRowsSink {
    fn emit(
        &mut self,
        _txn: &ReadTxn<'_>,
        _query: &NormalizedQuery,
        binding: &EncodedBinding,
        _counters: &mut PlanCounters,
    ) -> Result<()> {
        match &self.output {
            OutputPlan::Project(plan) => {
                let row = plan
                    .vars
                    .iter()
                    .map(|variable| bound_encoded_variable(binding, variable.0 as usize).cloned())
                    .collect::<Result<SmallEncodedRow>>()?;
                self.project_rows.insert(row);
            }
            OutputPlan::Aggregate(plan) => {
                if is_global_count_plan(plan) {
                    self.global_count = self.global_count.saturating_add(1);
                    return Ok(());
                }
                let key = plan
                    .group_vars
                    .iter()
                    .map(|variable| bound_encoded_variable(binding, variable.0 as usize).cloned())
                    .collect::<Result<SmallEncodedRow>>()?;
                self.aggregate_groups.insert(key);
            }
        }
        Ok(())
    }

    fn finish(
        self,
        _txn: &ReadTxn<'_>,
        _query: &NormalizedQuery,
        _counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>> {
        Ok(Vec::new())
    }
}

#[derive(Clone, Debug)]
struct AggregateSink {
    group_vars: Vec<VarId>,
    terms: Vec<AggregateTerm>,
    groups: BTreeMap<SmallEncodedRow, Vec<AggregateState>>,
    seen_domains: BTreeMap<(SmallEncodedRow, usize), BTreeSet<SmallEncodedRow>>,
}

impl AggregateSink {
    fn new(plan: &AggregatePlan) -> Self {
        Self {
            group_vars: plan.group_vars.clone(),
            terms: plan.aggregates.clone(),
            groups: BTreeMap::new(),
            seen_domains: BTreeMap::new(),
        }
    }

    fn group_key(&self, binding: &EncodedBinding) -> Result<SmallEncodedRow> {
        self.group_vars
            .iter()
            .map(|variable| bound_encoded_variable(binding, variable.0 as usize).cloned())
            .collect()
    }

    fn domain_key(term: &AggregateTerm, binding: &EncodedBinding) -> Result<SmallEncodedRow> {
        term.domain_vars
            .iter()
            .map(|variable| bound_encoded_variable(binding, variable.0 as usize).cloned())
            .collect()
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
        counters.aggregate_emit_calls += 1;
        let key = self.group_key(binding)?;
        let states = ensure_aggregate_group(&mut self.groups, &self.terms, key.clone());
        for (ordinal, (state, term)) in states.iter_mut().zip(&self.terms).enumerate() {
            let domain_key = Self::domain_key(term, binding)?;
            let seen = self.seen_domains.entry((key.clone(), ordinal)).or_default();
            if !seen.insert(domain_key) {
                continue;
            }
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
        let mut groups = self.groups;
        if groups.is_empty()
            && self.group_vars.is_empty()
            && self.terms.len() == 1
            && matches!(
                self.terms[0].function,
                AggregateFunction::CountDomain | AggregateFunction::CountDistinct
            )
        {
            groups.insert(
                SmallEncodedRow::new(),
                initial_aggregate_states(&self.terms),
            );
        }
        for (key, states) in groups {
            let mut row = Vec::new();
            let mut key_iter = key.into_iter();
            let mut state_iter = states.into_iter();
            for term in &query.find {
                match term {
                    NormFindTerm::Variable { variable } => {
                        let value = key_iter
                            .next()
                            .ok_or_else(|| Error::internal("aggregate group key is missing"))?;
                        row.push(decode_output_value(
                            txn,
                            &query.vars[variable.0 as usize].value_type,
                            value,
                            counters,
                        )?);
                    }
                    NormFindTerm::Aggregate { value_type, .. } => {
                        counters.materialized_output_values += 1;
                        let state = state_iter
                            .next()
                            .ok_or_else(|| Error::internal("aggregate state is missing"))?;
                        row.push(state.finish_encoded(txn, value_type, counters)?);
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

fn bound_encoded_variable(binding: &EncodedBinding, variable: usize) -> Result<&EncodedOwned> {
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
    value_type: &ValueType,
    value: EncodedOwned,
    counters: &mut PlanCounters,
) -> Result<Value> {
    counters.materialized_output_values += 1;
    record_decode(value_type, counters);
    txn.decode_query_value(value_type, value.as_bytes())
}

#[derive(Clone, Debug)]
enum AggregateState {
    Count(u64),
    SumU64(u64),
    SumI64(i64),
    SumDecimal(i128),
    EncodedMin(Option<EncodedOwned>),
    EncodedMax(Option<EncodedOwned>),
    Min(Option<Value>),
    Max(Option<Value>),
}

impl AggregateState {
    fn new(function: AggregateFunction, value_type: ValueType) -> Self {
        match (function, value_type) {
            (AggregateFunction::CountDomain | AggregateFunction::CountDistinct, _) => {
                AggregateState::Count(0)
            }
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
        let AggregateState::Count(count) = self else {
            return Err(Error::internal("count aggregate state mismatch"));
        };
        *count = count
            .checked_add(1)
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

    fn finish_encoded(
        self,
        txn: &ReadTxn<'_>,
        value_type: &ValueType,
        counters: &mut PlanCounters,
    ) -> Result<Value> {
        Ok(match self {
            AggregateState::EncodedMin(Some(value)) | AggregateState::EncodedMax(Some(value)) => {
                record_decode(value_type, counters);
                txn.decode_query_value(value_type, value.as_bytes())?
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
        ValueType::TimestampMicros => "timestamp".to_owned(),
        ValueType::Decimal { scale } => format!("decimal(scale={scale})"),
        ValueType::Enum { name } => name.clone(),
        ValueType::String => "string".to_owned(),
        ValueType::Bytes => "bytes".to_owned(),
        ValueType::Serial {
            type_name,
            owning_relation,
        } => format!("{type_name}@{owning_relation}"),
    }
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
