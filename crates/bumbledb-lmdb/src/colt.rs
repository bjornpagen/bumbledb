#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use crate::base_image::RelationBaseImage;
use crate::query::free_join::ValidatedFjPlan;
use crate::query::model::{AtomOccurrenceId, NormalizedQuery};
use crate::tuple::{EncodedTuple, GhtSource, KeyCountEstimate, TupleField, TupleSchema};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceFilterOp {
    Eq,
    NotEq,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourceFilter {
    Compare {
        field_id: usize,
        op: SourceFilterOp,
        value: Vec<u8>,
    },
    False,
}

impl SourceFilter {
    pub(crate) fn field_id(&self) -> Option<usize> {
        match self {
            SourceFilter::Compare { field_id, .. } => Some(*field_id),
            SourceFilter::False => None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ColtSource {
    node: Rc<RefCell<ColtNode>>,
    vars: Vec<usize>,
}

struct ColtNode {
    atom: AtomOccurrenceId,
    base: Arc<RelationBaseImage>,
    schemas: Vec<TupleSchema>,
    vars: Vec<usize>,
    data: ColtData,
    counters: Rc<RefCell<ColtCounters>>,
}

enum ColtData {
    Offsets(Vec<usize>),
    Map(BTreeMap<EncodedTuple, Rc<RefCell<ColtNode>>>),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ColtCounters {
    pub(crate) nodes_created: usize,
    pub(crate) nodes_forced: usize,
    pub(crate) offsets_scanned: usize,
    pub(crate) hash_maps_built: usize,
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
        let counters = Rc::new(RefCell::new(ColtCounters {
            nodes_created: 1,
            ..ColtCounters::default()
        }));
        let vars = schemas.first().map_or_else(Vec::new, TupleSchema::vars);
        let offsets = (0..base.row_handles.len())
            .filter(|offset| {
                filters
                    .iter()
                    .all(|filter| source_filter_matches(&base, *offset, filter))
            })
            .collect();
        Self {
            vars: vars.clone(),
            node: Rc::new(RefCell::new(ColtNode {
                atom,
                base,
                schemas,
                vars,
                data: ColtData::Offsets(offsets),
                counters,
            })),
        }
    }

    pub(crate) fn counters(&self) -> ColtCounters {
        self.node.borrow().counters.borrow().clone()
    }

    pub(crate) fn is_vector(&self) -> bool {
        matches!(self.node.borrow().data, ColtData::Offsets(_))
    }

    pub(crate) fn offset_len(&self) -> usize {
        match &self.node.borrow().data {
            ColtData::Offsets(offsets) => offsets.len(),
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
        let ColtData::Offsets(offsets) =
            std::mem::replace(&mut node.data, ColtData::Offsets(Vec::new()))
        else {
            return;
        };
        let Some(schema) = node.schemas.first().cloned() else {
            node.data = ColtData::Offsets(offsets);
            return;
        };
        let child_schemas = node.schemas.iter().skip(1).cloned().collect::<Vec<_>>();
        let child_vars = child_schemas
            .first()
            .map_or_else(Vec::new, TupleSchema::vars);
        let mut grouped: BTreeMap<EncodedTuple, Vec<usize>> = BTreeMap::new();
        for offset in offsets {
            node.counters.borrow_mut().offsets_scanned += 1;
            if let Ok(tuple) = schema.tuple_from_base_offset(&node.base, offset) {
                grouped.entry(tuple).or_default().push(offset);
            }
        }
        let mut map = BTreeMap::new();
        for (tuple, offsets) in grouped {
            node.counters.borrow_mut().nodes_created += 1;
            map.insert(
                tuple,
                Rc::new(RefCell::new(ColtNode {
                    atom: node.atom,
                    base: Arc::clone(&node.base),
                    schemas: child_schemas.clone(),
                    vars: child_vars.clone(),
                    data: ColtData::Offsets(offsets),
                    counters: Rc::clone(&node.counters),
                })),
            );
        }
        node.counters.borrow_mut().nodes_forced += 1;
        node.counters.borrow_mut().hash_maps_built += 1;
        node.data = ColtData::Map(map);
    }
}

fn source_filter_matches(base: &RelationBaseImage, offset: usize, filter: &SourceFilter) -> bool {
    match filter {
        SourceFilter::False => false,
        SourceFilter::Compare {
            field_id,
            op,
            value,
        } => base
            .columns
            .get(field_id)
            .and_then(|column| column.values.get(offset))
            .is_some_and(|candidate| compare_encoded(candidate, *op, value)),
    }
}

fn compare_encoded(candidate: &[u8], op: SourceFilterOp, value: &[u8]) -> bool {
    match op {
        SourceFilterOp::Eq => candidate == value,
        SourceFilterOp::NotEq => candidate != value,
        SourceFilterOp::Lt => candidate < value,
        SourceFilterOp::Lte => candidate <= value,
        SourceFilterOp::Gt => candidate > value,
        SourceFilterOp::Gte => candidate >= value,
    }
}

impl GhtSource for ColtSource {
    type Child<'a> = ColtSource;

    fn atom(&self) -> Option<AtomOccurrenceId> {
        Some(self.node.borrow().atom)
    }

    fn vars(&self) -> &[usize] {
        &self.vars
    }

    fn iter(&self) -> Vec<EncodedTuple> {
        self.node.borrow().counters.borrow_mut().iter_calls += 1;
        if let ColtData::Offsets(offsets) = &self.node.borrow().data
            && self.node.borrow().schemas.len() == 1
        {
            let node = self.node.borrow();
            let schema = &node.schemas[0];
            return offsets
                .iter()
                .filter_map(|offset| schema.tuple_from_base_offset(&node.base, *offset).ok())
                .collect();
        }
        self.force();
        match &self.node.borrow().data {
            ColtData::Map(map) => map.keys().cloned().collect(),
            ColtData::Offsets(_) => Vec::new(),
        }
    }

    fn get(&self, tuple: &EncodedTuple) -> Option<Self::Child<'_>> {
        self.node.borrow().counters.borrow_mut().get_calls += 1;
        self.force();
        let node = self.node.borrow();
        let ColtData::Map(map) = &node.data else {
            return None;
        };
        let child = map.get(tuple).cloned();
        if child.is_none() {
            node.counters.borrow_mut().misses += 1;
        }
        child.map(|node| {
            let vars = node.borrow().vars.clone();
            ColtSource { node, vars }
        })
    }

    fn key_count(&self) -> KeyCountEstimate {
        match &self.node.borrow().data {
            ColtData::Map(map) => KeyCountEstimate::Exact(map.len()),
            ColtData::Offsets(offsets) => KeyCountEstimate::Estimate(offsets.len()),
        }
    }
}

pub(crate) fn tuple_schemas_for_atom(
    query: &NormalizedQuery,
    plan: &ValidatedFjPlan,
    atom: AtomOccurrenceId,
) -> Vec<TupleSchema> {
    let occurrence = &query.atoms[atom.0];
    let mut schemas = Vec::new();
    for node in &plan.nodes {
        for subatom in &node.subatoms {
            if subatom.atom == atom {
                let fields = subatom
                    .vars
                    .iter()
                    .zip(&subatom.field_ids)
                    .filter_map(|(variable, field_id)| {
                        let Ok(field) = TupleField::new(
                            *variable,
                            Some(*field_id),
                            occurrence.fields[*field_id].value_type.encoded_width(),
                        ) else {
                            return None;
                        };
                        Some(field)
                    })
                    .collect();
                schemas.push(TupleSchema::new(fields));
            }
        }
    }
    schemas
}

#[cfg(test)]
#[path = "colt_tests.rs"]
mod tests;
