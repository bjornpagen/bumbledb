#![allow(dead_code)]

use std::collections::HashMap;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::sync::Mutex;

use crate::base_image::RelationBaseImage;
use crate::colt_filter::source_filter_matches;
pub(crate) use crate::colt_filter::{SourceFilter, SourceFilterOp};
pub(crate) use crate::colt_schema::tuple_schemas_for_atom;
use crate::query::model::AtomOccurrenceId;
use crate::query::trace::{QueryTrace, TraceCounters, TracePhase};
use crate::tuple::{EncodedTupleRef, GhtSource, TupleBatch, TupleCursor, TupleSchema};

#[path = "colt/arena.rs"]
mod arena;
#[path = "colt/ght.rs"]
mod ght;
#[path = "colt/key.rs"]
mod key;

use key::KeyOwned;

#[derive(Clone)]
pub(crate) struct ColtSource {
    state: Arc<Mutex<ColtState>>,
    node: usize,
    vars: Arc<[usize]>,
}

pub(super) struct ColtState {
    nodes: Vec<ColtNode>,
    counters: ColtCounters,
}

pub(super) struct ColtNode {
    atom: AtomOccurrenceId,
    base: Arc<RelationBaseImage>,
    schemas: Arc<[TupleSchema]>,
    vars: Arc<[usize]>,
    data: ColtData,
}

#[derive(Clone)]
pub(super) enum ColtData {
    Range(usize),
    Offsets(Vec<u32>),
    Offset(usize),
    Map(HashMap<KeyOwned, usize>),
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
    pub(crate) fn new(
        atom: AtomOccurrenceId,
        base: Arc<RelationBaseImage>,
        schemas: Vec<TupleSchema>,
    ) -> Self {
        Self::new_filtered(atom, base, schemas, Vec::new())
    }

    pub(crate) fn new_filtered(
        atom: AtomOccurrenceId,
        base: Arc<RelationBaseImage>,
        schemas: Vec<TupleSchema>,
        filters: Vec<SourceFilter>,
    ) -> Self {
        Self::new_filtered_with_trace(atom, base, schemas, filters, None)
    }

    pub(crate) fn new_filtered_traced(
        atom: AtomOccurrenceId,
        base: Arc<RelationBaseImage>,
        schemas: Vec<TupleSchema>,
        filters: Vec<SourceFilter>,
        trace: &mut QueryTrace,
    ) -> Self {
        Self::new_filtered_with_trace(atom, base, schemas, filters, Some(trace))
    }

    fn new_filtered_with_trace(
        atom: AtomOccurrenceId,
        base: Arc<RelationBaseImage>,
        schemas: Vec<TupleSchema>,
        filters: Vec<SourceFilter>,
        mut trace: Option<&mut QueryTrace>,
    ) -> Self {
        let span = trace.as_deref_mut().and_then(|trace| {
            trace.start_span(
                TracePhase::ColtBuild,
                format!("relation={} atom={:?}", base.name, atom),
            )
        });
        let counters = ColtCounters {
            nodes_created: 1,
            ..ColtCounters::default()
        };
        let schemas: Arc<[TupleSchema]> = schemas.into();
        let vars: Arc<[usize]> = schemas
            .first()
            .map_or_else(Vec::new, TupleSchema::vars)
            .into();
        let source_filter_rows_tested = base.row_handles.len() as u64;
        let data = if filters.is_empty() {
            ColtData::Range(base.row_handles.len())
        } else {
            ColtData::Offsets(
                (0..base.row_handles.len())
                    .filter(|offset| {
                        filters
                            .iter()
                            .all(|filter| source_filter_matches(&base, *offset, filter))
                    })
                    .map(|offset| offset as u32)
                    .collect(),
            )
        };
        let state = Arc::new(Mutex::new(ColtState {
            nodes: vec![ColtNode {
                atom,
                base,
                schemas,
                vars: Arc::clone(&vars),
                data,
            }],
            counters,
        }));
        let source = Self {
            state,
            node: 0,
            vars,
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

    pub(crate) fn counters(&self) -> ColtCounters {
        self.state
            .lock()
            .map_or_else(|_| ColtCounters::default(), |state| state.counters.clone())
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
            let mut counters = colt_counter_delta(before, after, batch.tuples.len());
            counters.batches_yielded = u64::from(!batch.tuples.is_empty());
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
                trace.start_span(
                    TracePhase::ColtForce,
                    format!("force before get relation={:?}", self.atom()),
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
        let mut state = self.lock_state();
        state.counters.get_calls += 1;
        let ColtData::Map(map) = &state.nodes[self.node].data else {
            if let Some(span) = span {
                trace.finish_span(span, TraceCounters::default());
            }
            return None;
        };
        let child = map.get(tuple.bytes()).cloned();
        if child.is_none() {
            state.counters.misses += 1;
        }
        let output = child.map(|node| ColtSource {
            vars: Arc::clone(&state.nodes[node].vars),
            state: Arc::clone(&self.state),
            node,
        });
        drop(state);
        let after_get = self.counters();
        if let Some(span) = span {
            trace.finish_span(span, colt_counter_delta(before_get, after_get, 0));
        }
        output
    }

    pub(crate) fn is_vector(&self) -> bool {
        let state = self.lock_state();
        matches!(
            state.nodes[self.node].data,
            ColtData::Range(_) | ColtData::Offsets(_) | ColtData::Offset(_)
        )
    }

    pub(crate) fn offset_len(&self) -> usize {
        let state = self.lock_state();
        match &state.nodes[self.node].data {
            ColtData::Range(len) => *len,
            ColtData::Offsets(offsets) => offsets.len(),
            ColtData::Offset(_) => 1,
            ColtData::Map(map) => map.len(),
        }
    }

    pub(crate) fn has_child_level(&self) -> bool {
        self.lock_state().nodes[self.node].schemas.len() > 1
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.offset_len() == 0
    }

    fn force(&self) {
        if !self.is_vector() {
            return;
        }
        let mut state = self.lock_state();
        if !matches!(
            state.nodes[self.node].data,
            ColtData::Range(_) | ColtData::Offsets(_) | ColtData::Offset(_)
        ) {
            return;
        }
        let data = std::mem::replace(
            &mut state.nodes[self.node].data,
            ColtData::Offsets(Vec::new()),
        );
        let offset_count = match &data {
            ColtData::Range(len) => *len,
            ColtData::Offsets(offsets) => offsets.len(),
            ColtData::Offset(_) => 1,
            ColtData::Map(_) => 0,
        };
        let offsets: Box<dyn Iterator<Item = usize>> = match data {
            ColtData::Range(len) => Box::new(0..len),
            ColtData::Offsets(offsets) => {
                Box::new(offsets.into_iter().map(|offset| offset as usize))
            }
            ColtData::Offset(offset) => Box::new(std::iter::once(offset)),
            ColtData::Map(map) => {
                state.nodes[self.node].data = ColtData::Map(map);
                return;
            }
        };
        let Some(schema) = state.nodes[self.node].schemas.first().cloned() else {
            state.nodes[self.node].data = ColtData::Offsets(Vec::new());
            return;
        };
        let child_schemas: Arc<[TupleSchema]> = state.nodes[self.node]
            .schemas
            .iter()
            .skip(1)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        let child_vars: Arc<[usize]> = child_schemas
            .first()
            .map_or_else(Vec::new, TupleSchema::vars)
            .into();
        let base = Arc::clone(&state.nodes[self.node].base);
        let atom = state.nodes[self.node].atom;
        let mut map: HashMap<KeyOwned, usize> = HashMap::with_capacity(offset_count);
        let mut key_bytes = Vec::with_capacity(schema.encoded_width());
        for offset in offsets {
            state.counters.offsets_scanned += 1;
            if schema
                .write_tuple_from_base_offset(&base, offset, &mut key_bytes)
                .is_err()
            {
                continue;
            }
            if let Some(child) = map.get(key_bytes.as_slice()).copied() {
                push_child_offset(&mut state, child, offset);
            } else {
                state.counters.nodes_created += 1;
                let child = state.nodes.len();
                state.nodes.push(ColtNode {
                    atom,
                    base: Arc::clone(&base),
                    schemas: Arc::clone(&child_schemas),
                    vars: Arc::clone(&child_vars),
                    data: ColtData::Offset(offset),
                });
                map.insert(KeyOwned::from_slice(&key_bytes), child);
            }
        }
        state.counters.map_entries_built += map.len();
        state.counters.nodes_forced += 1;
        state.counters.hash_maps_built += 1;
        state.nodes[self.node].data = ColtData::Map(map);
    }

    pub(super) fn lock_state(&self) -> std::sync::MutexGuard<'_, ColtState> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

fn push_child_offset(state: &mut ColtState, child: usize, offset: usize) {
    match &mut state.nodes[child].data {
        ColtData::Offset(first) if *first != offset => {
            state.nodes[child].data = ColtData::Offsets(vec![*first as u32, offset as u32]);
        }
        ColtData::Offsets(child_offsets) => child_offsets.push(offset as u32),
        _ => {}
    }
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
