#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::ControlFlow;
use std::rc::Rc;
use std::sync::Arc;

use crate::base_image::RelationBaseImage;
use crate::colt_filter::source_filter_matches;
pub(crate) use crate::colt_filter::{SourceFilter, SourceFilterOp};
pub(crate) use crate::colt_schema::tuple_schemas_for_atom;
use crate::query::model::AtomOccurrenceId;
use crate::query::trace::{QueryTrace, TraceCounters, TracePhase};
use crate::tuple::{
    EncodedTuple, EncodedTupleRef, GhtSource, KeyCountEstimate, TupleBatch, TupleCursor,
    TupleSchema,
};

#[derive(Clone)]
pub(crate) struct ColtSource {
    node: Rc<RefCell<ColtNode>>,
    vars: Rc<Vec<usize>>,
}

struct ColtNode {
    atom: AtomOccurrenceId,
    base: Arc<RelationBaseImage>,
    schemas: Rc<Vec<TupleSchema>>,
    vars: Rc<Vec<usize>>,
    data: ColtData,
    counters: Rc<RefCell<ColtCounters>>,
}

enum ColtData {
    Offsets(Vec<usize>),
    Offset(usize),
    Map(HashMap<EncodedTuple, Rc<RefCell<ColtNode>>>),
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
        let counters = Rc::new(RefCell::new(ColtCounters {
            nodes_created: 1,
            ..ColtCounters::default()
        }));
        let schemas = Rc::new(schemas);
        let vars = Rc::new(schemas.first().map_or_else(Vec::new, TupleSchema::vars));
        let source_filter_rows_tested = base.row_handles.len() as u64;
        let offsets = (0..base.row_handles.len())
            .filter(|offset| {
                filters
                    .iter()
                    .all(|filter| source_filter_matches(&base, *offset, filter))
            })
            .collect();
        let source = Self {
            vars: Rc::clone(&vars),
            node: Rc::new(RefCell::new(ColtNode {
                atom,
                base,
                schemas,
                vars,
                data: ColtData::Offsets(offsets),
                counters,
            })),
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
        self.node.borrow().counters.borrow().clone()
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
        self.node.borrow().counters.borrow_mut().get_calls += 1;
        let node = self.node.borrow();
        let ColtData::Map(map) = &node.data else {
            if let Some(span) = span {
                trace.finish_span(span, TraceCounters::default());
            }
            return None;
        };
        let child = map.get(tuple.bytes()).cloned();
        if child.is_none() {
            node.counters.borrow_mut().misses += 1;
        }
        drop(node);
        let output = child.map(|node| {
            let vars = Rc::clone(&node.borrow().vars);
            ColtSource { node, vars }
        });
        let after_get = self.counters();
        if let Some(span) = span {
            trace.finish_span(span, colt_counter_delta(before_get, after_get, 0));
        }
        output
    }

    pub(crate) fn is_vector(&self) -> bool {
        matches!(
            self.node.borrow().data,
            ColtData::Offsets(_) | ColtData::Offset(_)
        )
    }

    pub(crate) fn offset_len(&self) -> usize {
        match &self.node.borrow().data {
            ColtData::Offsets(offsets) => offsets.len(),
            ColtData::Offset(_) => 1,
            ColtData::Map(map) => map.len(),
        }
    }

    pub(crate) fn has_child_level(&self) -> bool {
        self.node.borrow().schemas.len() > 1
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.offset_len() == 0
    }

    fn force(&self) {
        if !self.is_vector() {
            return;
        }
        let mut node = self.node.borrow_mut();
        let data = std::mem::replace(&mut node.data, ColtData::Offsets(Vec::new()));
        let offsets = match data {
            ColtData::Offsets(offsets) => offsets,
            ColtData::Offset(offset) => vec![offset],
            ColtData::Map(map) => {
                node.data = ColtData::Map(map);
                return;
            }
        };
        let Some(schema) = node.schemas.first().cloned() else {
            node.data = ColtData::Offsets(offsets);
            return;
        };
        let child_schemas = Rc::new(node.schemas.iter().skip(1).cloned().collect::<Vec<_>>());
        let child_vars = Rc::new(
            child_schemas
                .first()
                .map_or_else(Vec::new, TupleSchema::vars),
        );
        let mut map: HashMap<EncodedTuple, Rc<RefCell<ColtNode>>> =
            HashMap::with_capacity(offsets.len());
        let mut key_bytes = Vec::with_capacity(schema.encoded_width());
        for offset in offsets {
            node.counters.borrow_mut().offsets_scanned += 1;
            if schema
                .write_tuple_from_base_offset(&node.base, offset, &mut key_bytes)
                .is_err()
            {
                continue;
            }
            if let Some(child) = map.get_mut(key_bytes.as_slice()) {
                push_child_offset(child, offset);
            } else {
                node.counters.borrow_mut().nodes_created += 1;
                map.insert(
                    EncodedTuple::from_bytes(key_bytes.clone()),
                    Rc::new(RefCell::new(ColtNode {
                        atom: node.atom,
                        base: Arc::clone(&node.base),
                        schemas: Rc::clone(&child_schemas),
                        vars: Rc::clone(&child_vars),
                        data: ColtData::Offset(offset),
                        counters: Rc::clone(&node.counters),
                    })),
                );
            }
        }
        node.counters.borrow_mut().map_entries_built += map.len();
        node.counters.borrow_mut().nodes_forced += 1;
        node.counters.borrow_mut().hash_maps_built += 1;
        node.data = ColtData::Map(map);
    }
}

fn push_child_offset(child: &Rc<RefCell<ColtNode>>, offset: usize) {
    let mut child = child.borrow_mut();
    match &mut child.data {
        ColtData::Offset(first) if *first != offset => {
            child.data = ColtData::Offsets(vec![*first, offset]);
        }
        ColtData::Offsets(child_offsets) => child_offsets.push(offset),
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

impl GhtSource for ColtSource {
    type Child<'a> = ColtSource;

    fn atom(&self) -> Option<AtomOccurrenceId> {
        Some(self.node.borrow().atom)
    }

    fn vars(&self) -> &[usize] {
        self.vars.as_slice()
    }

    fn try_for_each_tuple<E, F>(&self, mut f: F) -> std::result::Result<(), E>
    where
        F: FnMut(EncodedTupleRef<'_>) -> std::result::Result<ControlFlow<()>, E>,
    {
        self.node.borrow().counters.borrow_mut().iter_calls += 1;
        if let ColtData::Offsets(offsets) = &self.node.borrow().data
            && self.node.borrow().schemas.len() == 1
        {
            let node = self.node.borrow();
            let schema = &node.schemas[0];
            let mut bytes = Vec::with_capacity(schema.encoded_width());
            for offset in offsets {
                if schema
                    .write_tuple_from_base_offset(&node.base, *offset, &mut bytes)
                    .is_ok()
                    && f(EncodedTupleRef::new(&bytes))?.is_break()
                {
                    break;
                }
            }
            return Ok(());
        }
        if let ColtData::Offset(offset) = &self.node.borrow().data
            && self.node.borrow().schemas.len() == 1
        {
            let node = self.node.borrow();
            let schema = &node.schemas[0];
            let mut bytes = Vec::with_capacity(schema.encoded_width());
            if schema
                .write_tuple_from_base_offset(&node.base, *offset, &mut bytes)
                .is_ok()
            {
                let _ = f(EncodedTupleRef::new(&bytes))?;
            }
            return Ok(());
        }
        self.force();
        if let ColtData::Map(map) = &self.node.borrow().data {
            for key in map.keys() {
                if f(key.as_ref())?.is_break() {
                    break;
                }
            }
        }
        Ok(())
    }

    fn fill_batch(&self, cursor: &mut TupleCursor, batch_size: usize) -> TupleBatch {
        let batch_size = batch_size.max(1);
        self.node.borrow().counters.borrow_mut().iter_calls += 1;
        if let ColtData::Offsets(offsets) = &self.node.borrow().data
            && self.node.borrow().schemas.len() == 1
        {
            let node = self.node.borrow();
            let schema = &node.schemas[0];
            let mut tuples = Vec::with_capacity(batch_size.min(offsets.len()));
            while cursor.position < offsets.len() && tuples.len() < batch_size {
                let offset = offsets[cursor.position];
                cursor.position += 1;
                if let Ok(tuple) = schema.tuple_from_base_offset(&node.base, offset) {
                    tuples.push(tuple);
                }
            }
            return TupleBatch {
                tuples,
                exhausted: cursor.position >= offsets.len(),
            };
        }
        if let ColtData::Offset(offset) = &self.node.borrow().data
            && self.node.borrow().schemas.len() == 1
        {
            if cursor.position > 0 {
                return TupleBatch {
                    tuples: Vec::new(),
                    exhausted: true,
                };
            }
            let node = self.node.borrow();
            let schema = &node.schemas[0];
            let mut tuples = Vec::with_capacity(1);
            if let Ok(tuple) = schema.tuple_from_base_offset(&node.base, *offset) {
                tuples.push(tuple);
            }
            cursor.position = 1;
            return TupleBatch {
                tuples,
                exhausted: true,
            };
        }
        self.force();
        if let ColtData::Map(map) = &self.node.borrow().data {
            let mut tuples = Vec::with_capacity(batch_size.min(map.len()));
            for key in map.keys().skip(cursor.position) {
                if tuples.len() >= batch_size {
                    break;
                }
                tuples.push(key.clone());
            }
            cursor.position += tuples.len();
            return TupleBatch {
                tuples,
                exhausted: cursor.position >= map.len(),
            };
        }
        TupleBatch {
            tuples: Vec::new(),
            exhausted: true,
        }
    }

    fn get(&self, tuple: EncodedTupleRef<'_>) -> Option<Self::Child<'_>> {
        self.node.borrow().counters.borrow_mut().get_calls += 1;
        self.force();
        let node = self.node.borrow();
        let ColtData::Map(map) = &node.data else {
            return None;
        };
        let child = map.get(tuple.bytes()).cloned();
        if child.is_none() {
            node.counters.borrow_mut().misses += 1;
        }
        child.map(|node| {
            let vars = Rc::clone(&node.borrow().vars);
            ColtSource { node, vars }
        })
    }

    fn key_count(&self) -> KeyCountEstimate {
        match &self.node.borrow().data {
            ColtData::Map(map) => KeyCountEstimate::Exact(map.len()),
            ColtData::Offsets(offsets) => KeyCountEstimate::Estimate(offsets.len()),
            ColtData::Offset(_) => KeyCountEstimate::Estimate(1),
        }
    }
}

#[cfg(test)]
#[path = "colt_tests.rs"]
mod tests;
