use std::ops::ControlFlow;
use std::rc::Rc;

use crate::colt::{ColtData, ColtSource};
use crate::query::model::AtomOccurrenceId;
use crate::tuple::{EncodedTupleRef, GhtSource, KeyCountEstimate, TupleBatch, TupleCursor};

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
        if self.try_for_each_vector_tuple(&mut f)? {
            return Ok(());
        }
        self.force();
        if let ColtData::Map(map) = &self.node.borrow().data {
            for key in map.keys() {
                if f(EncodedTupleRef::new(key.bytes()))?.is_break() {
                    break;
                }
            }
        }
        Ok(())
    }

    fn fill_batch(&self, cursor: &mut TupleCursor, batch_size: usize) -> TupleBatch {
        let batch_size = batch_size.max(1);
        self.node.borrow().counters.borrow_mut().iter_calls += 1;
        if let Some(batch) = self.fill_vector_batch(cursor, batch_size) {
            return batch;
        }
        self.force();
        if let ColtData::Map(map) = &self.node.borrow().data {
            let mut tuples = Vec::with_capacity(batch_size.min(map.len()));
            for key in map.keys().skip(cursor.position) {
                if tuples.len() >= batch_size {
                    break;
                }
                tuples.push(key.to_encoded_tuple());
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
            ColtData::Range(len) => KeyCountEstimate::Estimate(*len),
            ColtData::Offsets(offsets) => KeyCountEstimate::Estimate(offsets.len()),
            ColtData::Offset(_) => KeyCountEstimate::Estimate(1),
        }
    }
}

impl ColtSource {
    fn try_for_each_vector_tuple<E, F>(&self, f: &mut F) -> std::result::Result<bool, E>
    where
        F: FnMut(EncodedTupleRef<'_>) -> std::result::Result<ControlFlow<()>, E>,
    {
        if self.node.borrow().schemas.len() != 1 {
            return Ok(false);
        }
        let node = self.node.borrow();
        let schema = &node.schemas[0];
        let mut bytes = Vec::with_capacity(schema.encoded_width());
        match &node.data {
            ColtData::Range(len) => {
                for offset in 0..*len {
                    if schema
                        .write_tuple_from_base_offset(&node.base, offset, &mut bytes)
                        .is_ok()
                        && f(EncodedTupleRef::new(&bytes))?.is_break()
                    {
                        break;
                    }
                }
                Ok(true)
            }
            ColtData::Offsets(offsets) => {
                for offset in offsets {
                    if schema
                        .write_tuple_from_base_offset(&node.base, *offset, &mut bytes)
                        .is_ok()
                        && f(EncodedTupleRef::new(&bytes))?.is_break()
                    {
                        break;
                    }
                }
                Ok(true)
            }
            ColtData::Offset(offset) => {
                if schema
                    .write_tuple_from_base_offset(&node.base, *offset, &mut bytes)
                    .is_ok()
                {
                    let _ = f(EncodedTupleRef::new(&bytes))?;
                }
                Ok(true)
            }
            ColtData::Map(_) => Ok(false),
        }
    }

    fn fill_vector_batch(&self, cursor: &mut TupleCursor, batch_size: usize) -> Option<TupleBatch> {
        if self.node.borrow().schemas.len() != 1 {
            return None;
        }
        let node = self.node.borrow();
        let schema = &node.schemas[0];
        match &node.data {
            ColtData::Range(len) => {
                let mut tuples = Vec::with_capacity(batch_size.min(*len));
                while cursor.position < *len && tuples.len() < batch_size {
                    let offset = cursor.position;
                    cursor.position += 1;
                    if let Ok(tuple) = schema.tuple_from_base_offset(&node.base, offset) {
                        tuples.push(tuple);
                    }
                }
                Some(TupleBatch {
                    tuples,
                    exhausted: cursor.position >= *len,
                })
            }
            ColtData::Offsets(offsets) => {
                let mut tuples = Vec::with_capacity(batch_size.min(offsets.len()));
                while cursor.position < offsets.len() && tuples.len() < batch_size {
                    let offset = offsets[cursor.position];
                    cursor.position += 1;
                    if let Ok(tuple) = schema.tuple_from_base_offset(&node.base, offset) {
                        tuples.push(tuple);
                    }
                }
                Some(TupleBatch {
                    tuples,
                    exhausted: cursor.position >= offsets.len(),
                })
            }
            ColtData::Offset(offset) => {
                if cursor.position > 0 {
                    return Some(TupleBatch {
                        tuples: Vec::new(),
                        exhausted: true,
                    });
                }
                let mut tuples = Vec::with_capacity(1);
                if let Ok(tuple) = schema.tuple_from_base_offset(&node.base, *offset) {
                    tuples.push(tuple);
                }
                cursor.position = 1;
                Some(TupleBatch {
                    tuples,
                    exhausted: true,
                })
            }
            ColtData::Map(_) => None,
        }
    }
}
