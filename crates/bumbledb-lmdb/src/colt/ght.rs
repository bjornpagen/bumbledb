use std::ops::ControlFlow;
use std::sync::Arc;

use crate::colt::{ColtData, ColtSource};
use crate::query::model::AtomOccurrenceId;
use crate::tuple::{EncodedTupleRef, GhtSource, KeyCountEstimate, TupleBatch, TupleCursor};

impl GhtSource for ColtSource {
    type Child<'a> = ColtSource;

    fn atom(&self) -> Option<AtomOccurrenceId> {
        Some(self.lock_state().nodes[self.node].atom)
    }

    fn vars(&self) -> &[usize] {
        &self.vars
    }

    fn try_for_each_tuple<E, F>(&self, mut f: F) -> std::result::Result<(), E>
    where
        F: FnMut(EncodedTupleRef<'_>) -> std::result::Result<ControlFlow<()>, E>,
    {
        self.lock_state().counters.iter_calls += 1;
        if self.try_for_each_vector_tuple(&mut f)? {
            return Ok(());
        }
        self.force();
        let keys = {
            let state = self.lock_state();
            match &state.nodes[self.node].data {
                ColtData::Map(map) => map.keys().cloned().collect::<Vec<_>>(),
                _ => Vec::new(),
            }
        };
        for key in keys {
            if f(EncodedTupleRef::new(key.bytes()))?.is_break() {
                break;
            }
        }
        Ok(())
    }

    fn fill_batch(&self, cursor: &mut TupleCursor, batch_size: usize) -> TupleBatch {
        let batch_size = batch_size.max(1);
        self.lock_state().counters.iter_calls += 1;
        if let Some(batch) = self.fill_vector_batch(cursor, batch_size) {
            return batch;
        }
        self.force();
        let state = self.lock_state();
        if let ColtData::Map(map) = &state.nodes[self.node].data {
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
        self.force();
        let mut state = self.lock_state();
        state.counters.get_calls += 1;
        let ColtData::Map(map) = &state.nodes[self.node].data else {
            return None;
        };
        let child = map.get(tuple.bytes()).cloned();
        if child.is_none() {
            state.counters.misses += 1;
        }
        child.map(|node| ColtSource {
            vars: Arc::clone(&state.nodes[node].vars),
            state: Arc::clone(&self.state),
            node,
        })
    }

    fn key_count(&self) -> KeyCountEstimate {
        let state = self.lock_state();
        match &state.nodes[self.node].data {
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
        let Some((schema, base, data)) = ({
            let state = self.lock_state();
            if state.nodes[self.node].schemas.len() != 1 {
                None
            } else {
                Some((
                    state.nodes[self.node].schemas[0].clone(),
                    Arc::clone(&state.nodes[self.node].base),
                    state.nodes[self.node].data.clone(),
                ))
            }
        }) else {
            return Ok(false);
        };
        let mut bytes = Vec::with_capacity(schema.encoded_width());
        match &data {
            ColtData::Range(len) => {
                for offset in 0..*len {
                    if schema
                        .write_tuple_from_base_offset(&base, offset, &mut bytes)
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
                        .write_tuple_from_base_offset(&base, *offset as usize, &mut bytes)
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
                    .write_tuple_from_base_offset(&base, *offset, &mut bytes)
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
        let state = self.lock_state();
        if state.nodes[self.node].schemas.len() != 1 {
            return None;
        }
        let node = &state.nodes[self.node];
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
                    let offset = offsets[cursor.position] as usize;
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
