#![allow(dead_code)]

use std::time::Instant;

use crate::diagnostics::{AllocationDelta, AllocationSnapshot};
#[cfg(any(debug_assertions, feature = "query-tracing"))]
use crate::diagnostics::{allocation_delta, allocation_snapshot};

pub const QUERY_TRACING_ENABLED: bool = cfg!(any(debug_assertions, feature = "query-tracing"));

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExecutionModePublic {
    #[default]
    Scalar,
    Vectorized {
        batch_size: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryExecutionOptions {
    pub allocation_tracking: bool,
    pub execution_mode: ExecutionModePublic,
}

impl Default for QueryExecutionOptions {
    fn default() -> Self {
        Self {
            allocation_tracking: false,
            execution_mode: ExecutionModePublic::Scalar,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProfiledQueryResult {
    pub result: crate::QueryResultSet,
    pub trace: QueryTrace,
}

#[derive(Clone, Debug)]
pub struct QueryTrace {
    pub spans: Vec<TraceSpan>,
    pub counters: TraceCounters,
    pub metadata: QueryTraceMetadata,
    #[allow(dead_code)]
    origin: Instant,
    next_id: u64,
    #[allow(dead_code)]
    stack: Vec<ActiveSpan>,
}

impl QueryTrace {
    pub fn new() -> Self {
        Self {
            spans: Vec::new(),
            counters: TraceCounters::default(),
            metadata: QueryTraceMetadata::default(),
            origin: Instant::now(),
            next_id: 0,
            stack: Vec::new(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        QUERY_TRACING_ENABLED
    }

    #[cfg(any(debug_assertions, feature = "query-tracing"))]
    pub(crate) fn start_span(
        &mut self,
        phase: TracePhase,
        label: impl Into<String>,
    ) -> Option<TraceSpanId> {
        let id = TraceSpanId(self.next_id);
        self.next_id += 1;
        let parent_id = self.stack.last().map(|span| span.id.0);
        self.stack.push(ActiveSpan {
            id,
            parent_id,
            phase,
            label: label.into(),
            start_nanos: self.origin.elapsed().as_nanos(),
            started_at: Instant::now(),
            start_allocs: allocation_snapshot(),
        });
        Some(id)
    }

    #[cfg(not(any(debug_assertions, feature = "query-tracing")))]
    pub(crate) fn start_span(
        &mut self,
        _phase: TracePhase,
        _label: impl Into<String>,
    ) -> Option<TraceSpanId> {
        None
    }

    #[cfg(any(debug_assertions, feature = "query-tracing"))]
    pub(crate) fn finish_span(&mut self, id: TraceSpanId, counters: TraceCounters) {
        let Some(active) = self.stack.pop() else {
            return;
        };
        if active.id != id {
            self.stack.clear();
            return;
        }
        self.counters.merge(&counters);
        self.spans.push(TraceSpan {
            id: active.id.0,
            parent_id: active.parent_id,
            phase: active.phase,
            label: active.label,
            start_nanos: active.start_nanos,
            elapsed_nanos: active.started_at.elapsed().as_nanos(),
            allocs: allocation_delta(active.start_allocs, allocation_snapshot()),
            counters,
        });
    }

    #[cfg(not(any(debug_assertions, feature = "query-tracing")))]
    pub(crate) fn finish_span(&mut self, _id: TraceSpanId, _counters: TraceCounters) {}

    #[cfg(any(debug_assertions, feature = "query-tracing"))]
    pub(crate) fn add_counters(&mut self, counters: &TraceCounters) {
        self.counters.merge(counters);
    }

    #[cfg(not(any(debug_assertions, feature = "query-tracing")))]
    pub(crate) fn add_counters(&mut self, _counters: &TraceCounters) {}
}

impl Default for QueryTrace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueryTraceMetadata {
    pub selected_plan_family: String,
    pub node_count: usize,
    pub cover_policy: String,
    pub execution_mode: String,
    pub output_mode: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TraceSpanId(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceSpan {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub phase: TracePhase,
    pub label: String,
    pub start_nanos: u128,
    pub elapsed_nanos: u128,
    pub allocs: AllocationDelta,
    pub counters: TraceCounters,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TracePhase {
    Normalize,
    PlanSelect,
    PlannerStats,
    BaseImageCacheLookup,
    BaseImageLoad,
    SourceFilterEncode,
    ColtBuild,
    ColtIter,
    ColtForce,
    ColtGet,
    CoverChoice,
    ExecuteNode,
    ProbeSibling,
    BindingExtend,
    SinkConsume,
    SinkFinish,
    DecodeValue,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TraceCounters {
    pub base_image_cache_hits: u64,
    pub base_image_cache_misses: u64,
    pub live_rows_scanned: u64,
    pub column_values_loaded: u64,
    pub loaded_bytes: u64,
    pub source_filters_encoded: u64,
    pub source_filter_false_decisions: u64,
    pub source_filter_rows_tested: u64,
    pub source_filter_survivors: u64,
    pub colt_nodes_created: u64,
    pub colt_nodes_forced: u64,
    pub colt_offsets_scanned: u64,
    pub colt_map_entries_built: u64,
    pub tuples_yielded: u64,
    pub batches_yielded: u64,
    pub cover_choices: u64,
    pub probe_calls: u64,
    pub probe_misses: u64,
    pub recursive_node_entries: u64,
    pub max_recursion_depth: u64,
    pub frame_pushes: u64,
    pub frame_pops: u64,
    pub binding_copies: u64,
    pub binding_writes: u64,
    pub binding_conflicts: u64,
    pub source_replacements: u64,
    pub source_frame_changes: u64,
    pub sink_consumes: u64,
    pub projection_duplicates_suppressed: u64,
    pub decoded_values: u64,
}

impl TraceCounters {
    pub fn merge(&mut self, other: &Self) {
        self.base_image_cache_hits += other.base_image_cache_hits;
        self.base_image_cache_misses += other.base_image_cache_misses;
        self.live_rows_scanned += other.live_rows_scanned;
        self.column_values_loaded += other.column_values_loaded;
        self.loaded_bytes += other.loaded_bytes;
        self.source_filters_encoded += other.source_filters_encoded;
        self.source_filter_false_decisions += other.source_filter_false_decisions;
        self.source_filter_rows_tested += other.source_filter_rows_tested;
        self.source_filter_survivors += other.source_filter_survivors;
        self.colt_nodes_created += other.colt_nodes_created;
        self.colt_nodes_forced += other.colt_nodes_forced;
        self.colt_offsets_scanned += other.colt_offsets_scanned;
        self.colt_map_entries_built += other.colt_map_entries_built;
        self.tuples_yielded += other.tuples_yielded;
        self.batches_yielded += other.batches_yielded;
        self.cover_choices += other.cover_choices;
        self.probe_calls += other.probe_calls;
        self.probe_misses += other.probe_misses;
        self.recursive_node_entries += other.recursive_node_entries;
        self.max_recursion_depth = self.max_recursion_depth.max(other.max_recursion_depth);
        self.frame_pushes += other.frame_pushes;
        self.frame_pops += other.frame_pops;
        self.binding_copies += other.binding_copies;
        self.binding_writes += other.binding_writes;
        self.binding_conflicts += other.binding_conflicts;
        self.source_replacements += other.source_replacements;
        self.source_frame_changes += other.source_frame_changes;
        self.sink_consumes += other.sink_consumes;
        self.projection_duplicates_suppressed += other.projection_duplicates_suppressed;
        self.decoded_values += other.decoded_values;
    }
}

#[derive(Clone, Debug)]
struct ActiveSpan {
    id: TraceSpanId,
    parent_id: Option<u64>,
    phase: TracePhase,
    label: String,
    start_nanos: u128,
    started_at: Instant,
    start_allocs: AllocationSnapshot,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::with_allocation_tracking_for_test;

    #[test]
    fn query_trace_constructs_without_storage() {
        let trace = QueryTrace::new();

        assert!(trace.is_enabled());
        assert!(trace.spans.is_empty());
        assert_eq!(trace.counters, TraceCounters::default());
    }

    #[test]
    fn nested_span_parent_ids_are_preserved() {
        let mut trace = QueryTrace::new();
        let parent = trace.start_span(TracePhase::Normalize, "normalize");
        assert!(parent.is_some());
        let Some(parent) = parent else {
            return;
        };
        let child = trace.start_span(TracePhase::PlanSelect, "plan");
        assert!(child.is_some());
        let Some(child) = child else { return };

        trace.finish_span(child, TraceCounters::default());
        trace.finish_span(parent, TraceCounters::default());

        assert_eq!(trace.spans.len(), 2);
        assert_eq!(trace.spans[0].parent_id, Some(parent.0));
        assert_eq!(trace.spans[1].parent_id, None);
    }

    #[test]
    fn compile_time_tracing_records_counters_in_debug_tests() {
        let mut trace = QueryTrace::new();

        let span = trace.start_span(TracePhase::Normalize, "normalize");
        trace.add_counters(&TraceCounters {
            tuples_yielded: 10,
            ..TraceCounters::default()
        });

        assert!(span.is_some());
        assert_eq!(trace.counters.tuples_yielded, 10);
    }

    #[test]
    fn allocation_delta_is_attached_to_span() {
        with_allocation_tracking_for_test(|| {
            let mut trace = QueryTrace::new();
            let span = trace.start_span(TracePhase::ColtIter, "allocating span");
            assert!(span.is_some());
            let Some(span) = span else { return };

            let values = Vec::<u64>::with_capacity(64);

            trace.finish_span(span, TraceCounters::default());
            assert!(values.capacity() >= 64);
            assert_eq!(trace.spans.len(), 1);
            assert!(trace.spans[0].allocs.alloc_calls > 0);
            assert!(trace.spans[0].allocs.allocated_bytes >= 64 * 8);
        });
    }

    #[test]
    fn counters_merge_by_addition() {
        let mut counters = TraceCounters {
            tuples_yielded: 1,
            probe_calls: 2,
            max_recursion_depth: 3,
            ..TraceCounters::default()
        };
        counters.merge(&TraceCounters {
            tuples_yielded: 4,
            probe_calls: 5,
            max_recursion_depth: 2,
            ..TraceCounters::default()
        });

        assert_eq!(counters.tuples_yielded, 5);
        assert_eq!(counters.probe_calls, 7);
        assert_eq!(counters.max_recursion_depth, 3);
    }
}
