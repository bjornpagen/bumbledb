use std::fmt::Write as _;

use super::*;

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
    /// Sink finalization/materialization time.
    pub sink_finish_micros: u128,
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
    pub(super) fn write_explain(self, out: &mut String, phase: &str) {
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
    pub(super) fn with_total(mut self, total: AllocationPhaseStats) -> Self {
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
    /// Number of LFTJ atom sources backed directly by durable access slices.
    pub lftj_lazy_access_slices: u64,
    /// Number of encoded projection facts observed before set insertion.
    pub encoded_project_facts_seen: u64,
    /// Number of encoded projection facts inserted into the result set.
    pub encoded_project_facts_inserted: u64,
    /// Number of encoded fact bytes observed by projection sink.
    pub encoded_project_fact_bytes: u64,
    /// Number of projection values decoded at output boundary.
    pub project_decode_values: u64,
}
