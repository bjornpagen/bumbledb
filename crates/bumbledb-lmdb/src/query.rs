use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;

use smallvec::SmallVec;

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_ir::{
    ComparisonOperator, Literal, TypedClause, TypedComparison, TypedFindTerm, TypedLiteral,
    TypedOperand, TypedQuery, TypedRelationAtom, TypedTerm,
};
use bumbledb_core::schema::{IndexKind, ValueType};

use crate::query_image::{FactId, FactRange};
use crate::{
    AtomId, EncodedOwned, Error, FieldId, FreeJoinPlan, IndexSpec, LinearIter, NodeId, OutputPlan,
    PlanNode, ProjectPlan, ReadTxn, RelationImage, RelationStats, Result, SortedTrieIndex,
    StorageSchema, SubAtom, TrieIter, Value, VarId,
};

use crate::QueryImageCacheDiagnostics;
use crate::allocation::{self, ALLOCATION_SIZE_CLASS_COUNT, AllocationDelta};
use crate::planner_stats::{PlannerIndexStats, PlannerRelationStats, PlannerStatsCacheDiagnostics};
use crate::query_image::{
    EncodedColumnBuilder, LftjAtomKey, QueryImageScope, SortedTrieBuild, encoded_column_builders,
    finish_column_builders,
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

/// One fact in a query result set.
pub type ResultFact = Vec<Value>;

/// Duplicate-free query result set in canonical fact order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryResultSet {
    /// Result columns in projection order.
    pub columns: Vec<ResultColumn>,
    /// Result facts in canonical order.
    pub facts: Vec<ResultFact>,
}

impl QueryResultSet {
    /// Builds a canonical result set from possibly unordered facts.
    pub fn new(columns: Vec<ResultColumn>, mut facts: Vec<ResultFact>) -> Self {
        facts.sort();
        facts.dedup();
        Self { columns, facts }
    }

    /// Number of facts in the set.
    pub fn cardinality(&self) -> usize {
        self.facts.len()
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
}

/// Physical query plan summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryPlan {
    /// Deterministic Free Join variable binding order.
    pub variable_order: Vec<String>,
    /// Query image cache diagnostics after acquiring this query image.
    pub query_image_cache: QueryImageCacheDiagnostics,
    /// Planner statistics cache diagnostics after planning.
    pub planner_stats: PlannerStatsCacheDiagnostics,
    /// Free Join physical plan IR.
    pub free_join: FreeJoinPlan,
    /// Coarse query phase timings.
    pub timings: QueryTimings,
    /// Allocation summary for this query, disabled by default.
    pub allocations: QueryAllocationStats,
    /// Execution counters.
    pub counters: PlanCounters,
}

impl QueryPlan {
    /// Renders this physical plan and its current execution counters.
    pub fn explain(&self) -> String {
        let mut out = String::new();
        out.push_str("QueryPlan\n");
        out.push_str(&format!("variable_order: {:?}\n", self.variable_order));
        out.push_str("timings:\n");
        out.push_str(&format!(
            "  query_timing total_micros={} validate_inputs_micros={} normalize_micros={} encode_inputs_micros={} query_image_micros={} plan_micros={} lftj_build_micros={} execute_micros={} lftj_execute_micros={} sink_emit_micros={} sink_finish_micros={} decode_micros={} unaccounted_micros={}\n",
            self.timings.total_micros,
            self.timings.validate_inputs_micros,
            self.timings.normalize_micros,
            self.timings.encode_inputs_micros,
            self.timings.query_image_micros,
            self.timings.plan_micros,
            self.timings.lftj_build_micros,
            self.timings.execute_micros,
            self.timings.lftj_execute_micros,
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
        self.allocations.execute.write_explain(&mut out, "execute");
        self.allocations
            .sink_finish
            .write_explain(&mut out, "sink_finish");
        out.push_str("planner:\n");
        out.push_str(&format!(
            "  query_image_cache cached_images={} hits={} misses={} builds={} build_micros={}\n",
            self.query_image_cache.cached_images,
            self.query_image_cache.hits,
            self.query_image_cache.misses,
            self.query_image_cache.builds,
            self.query_image_cache.build_micros
        ));
        out.push_str(&format!(
            "  planner_stats cached_relations={} hits={} misses={} builds={} build_micros={} field_stats_built={} index_stats_built={} stats_from_access_images={} stats_exact_scans={}\n",
            self.planner_stats.cached_relations,
            self.planner_stats.hits,
            self.planner_stats.misses,
            self.planner_stats.builds,
            self.planner_stats.build_micros,
            self.planner_stats.field_stats_built,
            self.planner_stats.index_stats_built,
            self.planner_stats.stats_from_access_images,
            self.planner_stats.stats_exact_scans
        ));
        out.push_str("free_join_plan:\n");
        for node in &self.free_join.nodes {
            out.push_str(&format!(
                "  free_join_node id={} bind_vars={:?} subatoms={}\n",
                node.id.0,
                node.bind_vars.iter().map(|var| var.0).collect::<Vec<_>>(),
                node.subatoms.len()
            ));
            for subatom in &node.subatoms {
                out.push_str(&format!(
                    "    free_join_subatom atom={} relation={} fields={:?} vars={:?}\n",
                    subatom.atom_id.0,
                    subatom.relation.0,
                    subatom
                        .fields
                        .iter()
                        .map(|field| field.0)
                        .collect::<Vec<_>>(),
                    subatom.vars.iter().map(|var| var.0).collect::<Vec<_>>()
                ));
            }
        }
        out.push_str("counters:\n");
        out.push_str(&format!("  cursor_seeks: {}\n", self.counters.cursor_seeks));
        out.push_str(&format!(
            "  facts_scanned: {}\n",
            self.counters.facts_scanned
        ));
        out.push_str(&format!(
            "  facts_matched: {}\n",
            self.counters.facts_matched
        ));
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
            "  lftj_lazy_access_slices: {}\n",
            self.counters.lftj_lazy_access_slices
        ));
        out.push_str(&format!(
            "  lftj_eager_builds_avoided: {}\n",
            self.counters.lftj_eager_builds_avoided
        ));
        out.push_str(&format!(
            "  atom_temp_relation_builds: {}\n",
            self.counters.atom_temp_relation_builds
        ));
        out.push_str(&format!(
            "  atom_temp_relation_source_facts: {}\n",
            self.counters.atom_temp_relation_source_facts
        ));
        out.push_str(&format!(
            "  atom_temp_relation_facts: {}\n",
            self.counters.atom_temp_relation_facts
        ));
        out.push_str(&format!(
            "  lftj_atom_source_facts_scanned: {}\n",
            self.counters.lftj_atom_source_facts_scanned
        ));
        out.push_str(&format!(
            "  lftj_atom_facts_retained: {}\n",
            self.counters.lftj_atom_facts_retained
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
        out.push_str(&format!("  output_facts: {}\n", self.counters.output_facts));
        out
    }
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
    /// Runtime execution time before sink finish.
    pub execute_micros: u128,
    /// LFTJ recursive execution time.
    pub lftj_execute_micros: u128,
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
        self.validate_inputs_micros
            .saturating_add(self.normalize_micros)
            .saturating_add(self.encode_inputs_micros)
            .saturating_add(self.query_image_micros)
            .saturating_add(self.plan_micros)
            .saturating_add(self.lftj_build_micros)
            .saturating_add(self.execute_micros)
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
    /// Runtime execution allocation delta.
    pub execute: AllocationPhaseStats,
    /// LFTJ execution allocation delta.
    pub lftj_execute: AllocationPhaseStats,
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

/// Execution counters for the Free Join query executor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanCounters {
    /// Number of encoded index scan openings.
    pub cursor_seeks: u64,
    /// Number of encoded index entries inspected.
    pub facts_scanned: u64,
    /// Number of encoded index entries accepted by currently bound constraints.
    pub facts_matched: u64,
    /// Number of complete encoded bindings yielded before projection/aggregation.
    pub bindings_yielded: u64,
    /// Number of comparison predicates evaluated.
    pub comparisons_evaluated: u64,
    /// Number of comparison predicate failures.
    pub comparisons_failed: u64,
    /// Number of final output facts.
    pub output_facts: u64,
    /// Number of complete bindings that reached an output boundary.
    pub bindings_completed: u64,
    /// Number of sink emit calls.
    pub sink_emit_calls: u64,
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
    /// Number of LFTJ atom sources backed directly by durable access slices.
    pub lftj_lazy_access_slices: u64,
    /// Number of eager sorted trie atom builds avoided by lazy access slices.
    pub lftj_eager_builds_avoided: u64,
    /// Number of temporary atom relation images built on cache misses.
    pub atom_temp_relation_builds: u64,
    /// Number of source facts inspected while building temporary atom relations.
    pub atom_temp_relation_source_facts: u64,
    /// Number of facts retained in temporary atom relations.
    pub atom_temp_relation_facts: u64,
    /// Number of source facts inspected by LFTJ atom build subphase tracing.
    pub lftj_atom_source_facts_scanned: u64,
    /// Number of facts retained by LFTJ atom build subphase tracing.
    pub lftj_atom_facts_retained: u64,
    /// Number of encoded bytes copied by LFTJ atom build subphase tracing.
    pub lftj_atom_bytes_copied: u64,
    /// LFTJ atom scan/filter/copy microseconds.
    pub lftj_atom_scan_micros: u64,
    /// LFTJ atom temporary column construction microseconds.
    pub lftj_atom_column_micros: u64,
    /// LFTJ atom sorted trie construction microseconds.
    pub lftj_atom_sort_micros: u64,
    /// Number of encoded projection facts observed before set insertion.
    pub encoded_project_facts_seen: u64,
    /// Number of encoded projection facts inserted into the result set.
    pub encoded_project_facts_inserted: u64,
    /// Number of encoded fact bytes observed by projection sink.
    pub encoded_project_fact_bytes: u64,
    /// Number of projection values decoded at output boundary.
    pub project_decode_values: u64,
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
    comparisons: Vec<NormPredicate>,
    summary: QueryPlan,
}

#[derive(Clone, Debug)]
struct PlannerStats {
    relations: BTreeMap<String, Arc<PlannerRelationStats>>,
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

    fn relation_facts(&self, relation: &str) -> u64 {
        self.relations
            .get(relation)
            .map(|stats| stats.facts as u64)
            .unwrap_or(1)
            .max(1)
    }

    fn index_stats(&self, relation: &str, index: &str) -> Option<&PlannerIndexStats> {
        self.relations.get(relation)?.indexes.get(index)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VariableOrderScore {
    variable: usize,
    candidate_estimate: u64,
    static_constraints: usize,
    bound_constraints: usize,
    relation_constraints: usize,
    degree: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VariableAccessScore {
    relation: String,
    index: String,
    fact_estimate: u64,
    prefix_len: usize,
    current_is_next: bool,
}

impl VariableAccessScore {
    fn access_label(&self) -> String {
        format!("{}.{}", self.relation, self.index)
    }
}

struct LftjAtomPlan<'a> {
    variables: Vec<usize>,
    source: LftjAtomSource<'a>,
    fact_count: usize,
}

enum LftjAtomSource<'a> {
    SortedTrie(Arc<SortedTrieIndex>),
    LazyAccess(LazyAccessSlice<'a>),
}

impl<'a> LftjAtomSource<'a> {
    fn iter(&'a self) -> LftjTrieIter<'a> {
        match self {
            LftjAtomSource::SortedTrie(index) => LftjTrieIter::Sorted(index.iter()),
            LftjAtomSource::LazyAccess(slice) => LftjTrieIter::Lazy(slice.iter()),
        }
    }
}

enum LftjTrieIter<'a> {
    Sorted(crate::SortedTrieIter<'a>),
    Lazy(LazyAccessIter<'a>),
}

impl LinearIter for LftjTrieIter<'_> {
    fn key(&self) -> Option<crate::EncodedRef<'_>> {
        match self {
            LftjTrieIter::Sorted(iter) => iter.key(),
            LftjTrieIter::Lazy(iter) => iter.key(),
        }
    }

    fn next(&mut self) {
        match self {
            LftjTrieIter::Sorted(iter) => iter.next(),
            LftjTrieIter::Lazy(iter) => iter.next(),
        }
    }

    fn seek(&mut self, target: crate::EncodedRef<'_>) {
        match self {
            LftjTrieIter::Sorted(iter) => iter.seek(target),
            LftjTrieIter::Lazy(iter) => iter.seek(target),
        }
    }

    fn at_end(&self) -> bool {
        match self {
            LftjTrieIter::Sorted(iter) => iter.at_end(),
            LftjTrieIter::Lazy(iter) => iter.at_end(),
        }
    }
}

impl TrieIter for LftjTrieIter<'_> {
    fn open(&mut self) {
        match self {
            LftjTrieIter::Sorted(iter) => iter.open(),
            LftjTrieIter::Lazy(iter) => iter.open(),
        }
    }

    fn up(&mut self) {
        match self {
            LftjTrieIter::Sorted(iter) => iter.up(),
            LftjTrieIter::Lazy(iter) => iter.up(),
        }
    }

    fn depth(&self) -> usize {
        match self {
            LftjTrieIter::Sorted(iter) => iter.depth(),
            LftjTrieIter::Lazy(iter) => iter.depth(),
        }
    }

    fn current_fact_range(&self) -> FactRange {
        match self {
            LftjTrieIter::Sorted(iter) => iter.current_fact_range(),
            LftjTrieIter::Lazy(iter) => iter.current_fact_range(),
        }
    }

    fn count(&self) -> usize {
        match self {
            LftjTrieIter::Sorted(iter) => iter.count(),
            LftjTrieIter::Lazy(iter) => iter.count(),
        }
    }
}

struct LazyAccessSlice<'a> {
    index: &'a crate::query_image::RelationIndexImage,
    fields: Vec<FieldId>,
    range: Range<usize>,
    fact_count: usize,
}

impl<'a> LazyAccessSlice<'a> {
    fn iter(&'a self) -> LazyAccessIter<'a> {
        LazyAccessIter {
            index: self.index,
            fields: &self.fields,
            root: self.range.clone(),
            stack: SmallVec::new(),
        }
    }
}

struct LazyAccessIter<'a> {
    index: &'a crate::query_image::RelationIndexImage,
    fields: &'a [FieldId],
    root: Range<usize>,
    stack: SmallVec<[LazyAccessFrame; 4]>,
}

#[derive(Clone, Copy)]
struct LazyAccessFrame {
    depth: usize,
    begin: usize,
    end: usize,
    pos: usize,
}

impl LazyAccessIter<'_> {
    fn current_frame(&self) -> Option<&LazyAccessFrame> {
        self.stack.last()
    }

    fn current_frame_mut(&mut self) -> Option<&mut LazyAccessFrame> {
        self.stack.last_mut()
    }

    fn component_at(&self, position: usize, field: FieldId) -> Option<crate::EncodedRef<'_>> {
        let entry = self.index.entry_at(position)?;
        let bytes = self.index.component_bytes(entry, field)?;
        encoded_ref_for_width(bytes)
    }

    fn group_bounds(&self, frame: LazyAccessFrame) -> Range<usize> {
        if frame.pos >= frame.end {
            return frame.end..frame.end;
        }
        let field = self.fields[frame.depth];
        let Some(key) = self
            .component_at(frame.pos, field)
            .map(|value| EncodedOwned::from_ref(value))
        else {
            return frame.end..frame.end;
        };
        let mut end = frame.pos + 1;
        while end < frame.end {
            let Some(next) = self.component_at(end, field) else {
                break;
            };
            if compare_encoded_ref_owned(next, &key) != std::cmp::Ordering::Equal {
                break;
            }
            end += 1;
        }
        frame.pos..end
    }

    fn group_start(&self, frame: LazyAccessFrame, position: usize) -> usize {
        if position >= frame.end {
            return frame.end;
        }
        let field = self.fields[frame.depth];
        let Some(key) = self
            .component_at(position, field)
            .map(|value| EncodedOwned::from_ref(value))
        else {
            return position;
        };
        let mut start = position;
        while start > frame.begin {
            let Some(prev) = self.component_at(start - 1, field) else {
                break;
            };
            if compare_encoded_ref_owned(prev, &key) != std::cmp::Ordering::Equal {
                break;
            }
            start -= 1;
        }
        start
    }
}

impl LinearIter for LazyAccessIter<'_> {
    fn key(&self) -> Option<crate::EncodedRef<'_>> {
        let frame = self.current_frame()?;
        if frame.pos >= frame.end || frame.depth >= self.fields.len() {
            return None;
        }
        self.component_at(frame.pos, self.fields[frame.depth])
    }

    fn next(&mut self) {
        let Some(frame) = self.current_frame().copied() else {
            return;
        };
        let end = self.group_bounds(frame).end;
        if let Some(frame) = self.current_frame_mut() {
            frame.pos = end;
        }
    }

    fn seek(&mut self, target: crate::EncodedRef<'_>) {
        let Some(frame) = self.current_frame().copied() else {
            return;
        };
        if frame.depth >= self.fields.len() {
            return;
        }
        let field = self.fields[frame.depth];
        let mut low = frame.pos;
        let mut high = frame.end;
        while low < high {
            let mid = low + (high - low) / 2;
            let Some(value) = self.component_at(mid, field) else {
                high = mid;
                continue;
            };
            if compare_encoded_ref(value, target) == std::cmp::Ordering::Less {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        let pos = self.group_start(frame, low);
        if let Some(frame) = self.current_frame_mut() {
            frame.pos = pos;
        }
    }

    fn at_end(&self) -> bool {
        self.current_frame()
            .is_none_or(|frame| frame.pos >= frame.end)
    }
}

impl TrieIter for LazyAccessIter<'_> {
    fn open(&mut self) {
        let depth = self.stack.len();
        if depth >= self.fields.len() {
            self.stack.push(LazyAccessFrame {
                depth,
                begin: 0,
                end: 0,
                pos: 0,
            });
            return;
        }
        let range = if depth == 0 {
            self.root.clone()
        } else if let Some(parent) = self.current_frame().copied() {
            self.group_bounds(parent)
        } else {
            0..0
        };
        self.stack.push(LazyAccessFrame {
            depth,
            begin: range.start,
            end: range.end,
            pos: range.start,
        });
    }

    fn up(&mut self) {
        self.stack.pop();
    }

    fn depth(&self) -> usize {
        self.current_frame().map_or(0, |frame| frame.depth)
    }

    fn current_fact_range(&self) -> FactRange {
        let Some(frame) = self.current_frame().copied() else {
            return FactRange {
                start: FactId(0),
                end: FactId(0),
            };
        };
        let range = self.group_bounds(frame);
        FactRange {
            start: FactId(range.start as u32),
            end: FactId(range.end as u32),
        }
    }

    fn count(&self) -> usize {
        let Some(frame) = self.current_frame().copied() else {
            return 0;
        };
        let range = self.group_bounds(frame);
        range.end.saturating_sub(range.start)
    }
}

struct LftjRuntime<'a> {
    participants_by_variable: Vec<SmallParticipants>,
    iters: Vec<LftjTrieIter<'a>>,
}

type SmallParticipants = SmallVec<[usize; 4]>;
type SmallEncodedFact = SmallVec<[EncodedOwned; 8]>;
type LazyAccessShape = (Vec<u8>, usize, Vec<FieldId>);
include!("query/api.rs");

include!("query/timing.rs");

include!("query/hash.rs");

include!("query/lftj_runtime.rs");

include!("query/lftj_access.rs");

include!("query/planner.rs");

include!("query/values.rs");

include!("query/normalize.rs");

include!("query/sinks.rs");

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
