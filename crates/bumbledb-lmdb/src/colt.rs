#![allow(dead_code)]

use std::cell::UnsafeCell;
use std::ops::ControlFlow;
use std::ptr::NonNull;

use crate::base_image::RelationBaseImageRef;
use crate::colt_filter::source_filter_matches;
pub(crate) use crate::colt_filter::{SourceFilter, SourceFilterOp};
pub(crate) use crate::colt_schema::tuple_schemas_for_atom;
use crate::query::model::AtomOccurrenceId;
use crate::query::trace::{QueryTrace, TraceCounters, TracePhase};
use crate::tuple::{EncodedTupleRef, GhtSource, InlineTuple, TupleBatch, TupleCursor, TupleSchema};

#[path = "colt/arena.rs"]
mod arena;
#[path = "colt/ght.rs"]
mod ght;
#[path = "colt/key.rs"]
mod key;

use arena::{ColtArena, ColtMapId, ColtNodeId, NodeData};
use key::KeyRef;
pub(crate) use key::{KeyOwned, KeyScratch};

#[derive(Clone, Copy)]
pub(crate) struct ColtSource {
    state: NonNull<UnsafeCell<ColtState>>,
    node: ColtNodeId,
}

pub(crate) struct ColtSourceOwner {
    states: Vec<UnsafeCell<ColtState>>,
}

pub(crate) struct OwnedColtSource {
    _owner: ColtSourceOwner,
    source: ColtSource,
}

pub(super) struct ColtState {
    arena: ColtArena,
    atom: AtomOccurrenceId,
    base: RelationBaseImageRef,
    schemas: Vec<TupleSchema>,
    vars_by_level: Vec<Vec<usize>>,
    nodes: Vec<ColtNode>,
    counters: ColtCounters,
}

pub(super) struct ColtNode {
    level: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ColtCounters {
    pub(crate) nodes_created: usize,
    pub(crate) nodes_forced: usize,
    pub(crate) offsets_scanned: usize,
    pub(crate) hash_maps_built: usize,
    pub(crate) map_entries_built: usize,
    pub(crate) get_calls: usize,
    pub(crate) misses: usize,
    pub(crate) iter_calls: usize,
}

impl ColtSource {
    #[allow(clippy::new_ret_no_self)]
    pub(crate) fn new(
        atom: AtomOccurrenceId,
        base: RelationBaseImageRef,
        schemas: Vec<TupleSchema>,
    ) -> OwnedColtSource {
        Self::new_filtered(atom, base, schemas, Vec::new())
    }

    pub(crate) fn new_filtered(
        atom: AtomOccurrenceId,
        base: RelationBaseImageRef,
        schemas: Vec<TupleSchema>,
        filters: Vec<SourceFilter>,
    ) -> OwnedColtSource {
        let mut owner = ColtSourceOwner::new();
        let source =
            owner.add_filtered_with_trace(atom, base, schemas, filters, None, String::new());
        OwnedColtSource {
            _owner: owner,
            source,
        }
    }

    pub(crate) fn new_filtered_traced(
        atom: AtomOccurrenceId,
        base: RelationBaseImageRef,
        schemas: Vec<TupleSchema>,
        filters: Vec<SourceFilter>,
        trace: &mut QueryTrace,
    ) -> OwnedColtSource {
        let mut owner = ColtSourceOwner::new();
        let source =
            owner.add_filtered_with_trace(atom, base, schemas, filters, Some(trace), String::new());
        OwnedColtSource {
            _owner: owner,
            source,
        }
    }

    pub(crate) fn counters(&self) -> ColtCounters {
        self.state().counters.clone()
    }

    pub(crate) fn try_for_each_tuple_traced<E, F>(
        &self,
        trace: &mut QueryTrace,
        label: impl Into<String>,
        mut f: F,
    ) -> std::result::Result<(), E>
    where
        F: FnMut(EncodedTupleRef<'_>, &mut QueryTrace) -> std::result::Result<ControlFlow<()>, E>,
    {
        let before = self.counters();
        let span = trace.start_span(TracePhase::ColtIter, label);
        let mut tuples = 0usize;
        let result = self.try_for_each_tuple(|tuple| {
            tuples += 1;
            f(tuple, trace)
        });
        let after = self.counters();
        if let Some(span) = span {
            trace.finish_span(span, colt_counter_delta(before, after, tuples));
        }
        result
    }

    pub(crate) fn fill_batch_traced(
        &self,
        cursor: &mut TupleCursor,
        batch_size: usize,
        trace: &mut QueryTrace,
        label: impl Into<String>,
    ) -> TupleBatch {
        let before = self.counters();
        let span = trace.start_span(TracePhase::ColtIter, label);
        let batch = self.fill_batch(cursor, batch_size);
        let after = self.counters();
        if let Some(span) = span {
            let mut counters = colt_counter_delta(before, after, batch.len());
            counters.batches_yielded = u64::from(!batch.is_empty());
            trace.finish_span(span, counters);
        }
        batch
    }

    pub(crate) fn get_traced(
        &self,
        tuple: EncodedTupleRef<'_>,
        trace: &mut QueryTrace,
        label: impl Into<String>,
    ) -> Option<ColtSource> {
        let force_span = self
            .is_vector()
            .then(|| {
                crate::query_trace_span!(
                    trace,
                    TracePhase::ColtForce,
                    "force before get relation={:?}",
                    self.atom()
                )
            })
            .flatten();
        let before_force = self.counters();
        self.force();
        let after_force = self.counters();
        if let Some(span) = force_span {
            trace.finish_span(span, colt_counter_delta(before_force, after_force, 0));
        }

        let before_get = self.counters();
        let span = trace.start_span(TracePhase::ColtGet, label);
        let state = self.state_mut();
        state.counters.get_calls += 1;
        let child = map_for_source(state, self)
            .and_then(|map| state.arena.lookup_map(map, KeyRef::new(tuple.bytes())));
        if child.is_none() {
            state.counters.misses += 1;
        }
        let output = child.map(|node| ColtSource {
            state: self.state,
            node,
        });
        let after_get = self.counters();
        if let Some(span) = span {
            trace.finish_span(span, colt_counter_delta(before_get, after_get, 0));
        }
        output
    }

    pub(crate) fn is_vector(&self) -> bool {
        let state = self.state();
        state
            .arena
            .node_data(self.node)
            .is_some_and(|data| !matches!(data, NodeData::Map(_)))
    }

    pub(crate) fn offset_len(&self) -> usize {
        let state = self.state();
        state
            .arena
            .node_data(self.node)
            .map_or(0, |data| state.arena.item_count(data))
    }

    pub(crate) fn has_child_level(&self) -> bool {
        let state = self.state();
        state.nodes[self.node.0 as usize].level + 1 < state.schemas.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.offset_len() == 0
    }

    fn force(&self) {
        let state = self.state_mut();
        let Some(data) = state.arena.node_data(self.node) else {
            return;
        };
        if matches!(data, NodeData::Map(_)) {
            return;
        }
        let node_index = self.node.0 as usize;
        let offset_count = state.arena.item_count(data);
        let level = state.nodes[node_index].level;
        if level >= state.schemas.len() {
            let empty = state.arena.child_offsets_data(&[]);
            state.arena.set_node_data(self.node, empty);
            return;
        }
        let child_level = level + 1;
        let map = state.arena.add_map_table(offset_count, offset_count);
        let mut key_tuple = InlineTuple::default();
        let first_child = state.nodes.len();
        let mut child_counts = Vec::new();
        for position in 0..offset_count as u32 {
            let Some(offset) = state.arena.offset_at_position(data, position) else {
                continue;
            };
            state.counters.offsets_scanned += 1;
            if state.schemas[level]
                .inline_tuple_from_base_offset(&state.base, offset as usize, &mut key_tuple)
                .is_err()
            {
                continue;
            }
            let key = KeyRef::new(key_tuple.as_ref().bytes());
            if let Some(child) = state.arena.lookup_map(map, key) {
                let index = child.0 as usize - first_child;
                child_counts[index] += 1;
            } else {
                state.counters.nodes_created += 1;
                let child = state.arena.add_singleton_node(offset);
                debug_assert_eq!(child.0 as usize, state.nodes.len());
                state.nodes.push(ColtNode { level: child_level });
                child_counts.push(1u32);
                state.arena.insert_map_entry(map, key, child);
            }
        }
        for (index, count) in child_counts.iter().copied().enumerate() {
            if count > 1 {
                let range = state.arena.reserve_offsets(count);
                state.arena.set_node_data(
                    ColtNodeId((first_child + index) as u32),
                    NodeData::Offsets(range),
                );
            }
        }
        let mut child_positions = vec![0u32; child_counts.len()];
        for position in 0..offset_count as u32 {
            let Some(offset) = state.arena.offset_at_position(data, position) else {
                continue;
            };
            if state.schemas[level]
                .inline_tuple_from_base_offset(&state.base, offset as usize, &mut key_tuple)
                .is_err()
            {
                continue;
            }
            let key = KeyRef::new(key_tuple.as_ref().bytes());
            let Some(child) = state.arena.lookup_map(map, key) else {
                continue;
            };
            let index = child.0 as usize - first_child;
            if child_counts[index] > 1
                && let Some(NodeData::Offsets(range)) = state.arena.node_data(child)
            {
                state
                    .arena
                    .set_offset(range, child_positions[index], offset);
                child_positions[index] += 1;
            }
        }
        state.counters.map_entries_built += state.arena.map_entry_count(map);
        state.counters.nodes_forced += 1;
        state.counters.hash_maps_built += 1;
        state.arena.set_node_data(self.node, NodeData::Map(map));
    }

    pub(super) fn state(&self) -> &ColtState {
        // SAFETY: `ColtSource` handles are created only by `ColtSourceOwner`,
        // which owns the boxed state for the whole query/test execution. The
        // engine is single-threaded and never uses a handle after its owner is
        // dropped.
        unsafe { &*self.state.as_ref().get() }
    }

    #[allow(clippy::mut_from_ref)]
    pub(super) fn state_mut(&self) -> &mut ColtState {
        // SAFETY: execution mutates COLT state through short-lived operations on
        // one thread. Handles are compact aliases into the query-local owner;
        // callers do not retain references returned from this method across a
        // recursive call or user callback.
        unsafe { &mut *self.state.as_ref().get() }
    }
}

impl ColtSourceOwner {
    pub(crate) fn new() -> Self {
        Self::with_capacity(1)
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            states: Vec::with_capacity(capacity),
        }
    }

    pub(crate) fn add_filtered_traced_labeled(
        &mut self,
        atom: AtomOccurrenceId,
        base: RelationBaseImageRef,
        schemas: Vec<TupleSchema>,
        filters: Vec<SourceFilter>,
        trace_label: String,
        trace: &mut QueryTrace,
    ) -> ColtSource {
        self.add_filtered_with_trace(atom, base, schemas, filters, Some(trace), trace_label)
    }

    pub(crate) fn add_filtered_with_trace(
        &mut self,
        atom: AtomOccurrenceId,
        base: RelationBaseImageRef,
        schemas: Vec<TupleSchema>,
        filters: Vec<SourceFilter>,
        mut trace: Option<&mut QueryTrace>,
        trace_label: String,
    ) -> ColtSource {
        let span = trace.as_deref_mut().and_then(|trace| {
            crate::query_trace_span!(
                trace,
                TracePhase::ColtBuild,
                "relation={} atom={:?} filters={}",
                base.name,
                atom,
                trace_label
            )
        });
        let counters = ColtCounters {
            nodes_created: 1,
            ..ColtCounters::default()
        };
        let vars_by_level = vars_by_level(&schemas);
        let source_filter_rows_tested = base.row_handles.len() as u64;
        let mut arena = ColtArena::new();
        let data = if filters.is_empty() {
            NodeData::Range {
                start: 0,
                len: base.row_handles.len() as u32,
            }
        } else {
            arena.filtered_offsets_data(base.row_handles.len(), |offset| {
                filters
                    .iter()
                    .all(|filter| source_filter_matches(&base, offset, filter))
            })
        };
        let root = arena.add_node_data(data);
        debug_assert_eq!(root.0, 0);
        let state = ColtState {
            arena,
            atom,
            base,
            schemas,
            vars_by_level,
            nodes: vec![ColtNode { level: 0 }],
            counters,
        };
        self.states.push(UnsafeCell::new(state));
        let state_index = self.states.len() - 1;
        let state_ptr = NonNull::from(&self.states[state_index]);
        let source = ColtSource {
            state: state_ptr,
            node: root,
        };
        if let (Some(trace), Some(span)) = (trace, span) {
            trace.finish_span(
                span,
                TraceCounters {
                    source_filter_rows_tested,
                    source_filter_survivors: source.offset_len() as u64,
                    colt_nodes_created: 1,
                    ..TraceCounters::default()
                },
            );
        }
        source
    }
}

impl std::ops::Deref for OwnedColtSource {
    type Target = ColtSource;

    fn deref(&self) -> &Self::Target {
        &self.source
    }
}

fn map_for_source(state: &ColtState, source: &ColtSource) -> Option<ColtMapId> {
    state.arena.map_for_node(source.node)
}

fn vars_by_level(schemas: &[TupleSchema]) -> Vec<Vec<usize>> {
    let mut vars = Vec::with_capacity(schemas.len());
    for schema in schemas {
        let mut level = Vec::with_capacity(schema.fields.len());
        for field in &schema.fields {
            level.push(field.variable);
        }
        vars.push(level);
    }
    vars
}

fn colt_counter_delta(before: ColtCounters, after: ColtCounters, tuples: usize) -> TraceCounters {
    TraceCounters {
        colt_nodes_created: after.nodes_created.saturating_sub(before.nodes_created) as u64,
        colt_nodes_forced: after.nodes_forced.saturating_sub(before.nodes_forced) as u64,
        colt_offsets_scanned: after.offsets_scanned.saturating_sub(before.offsets_scanned) as u64,
        colt_map_entries_built: after
            .map_entries_built
            .saturating_sub(before.map_entries_built) as u64,
        tuples_yielded: tuples as u64,
        probe_calls: after.get_calls.saturating_sub(before.get_calls) as u64,
        probe_misses: after.misses.saturating_sub(before.misses) as u64,
        ..TraceCounters::default()
    }
}

#[cfg(test)]
#[path = "colt_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "colt_alloc_benchmarks.rs"]
mod allocation_benchmarks;
