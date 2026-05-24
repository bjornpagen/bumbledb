use std::ops::ControlFlow;

use crate::colt::arena::NodeData;
use crate::colt::key::{KeyRef, KeyScratch};
use crate::colt::{ColtSource, OwnedColtSource};
use crate::query::model::AtomOccurrenceId;
use crate::tuple::{
    EncodedTupleRef, GhtSource, InlineTuple, KeyCountEstimate, TupleBatch, TupleCursor,
};

impl GhtSource for ColtSource {
    type Child<'a> = ColtSource;

    fn atom(&self) -> Option<AtomOccurrenceId> {
        Some(self.state().atom)
    }

    fn vars(&self) -> &[usize] {
        let state = self.state();
        let level = state.nodes[self.node.0 as usize].level;
        state.vars_by_level.get(level).map_or(&[], Vec::as_slice)
    }

    fn try_for_each_tuple<E, F>(&self, mut f: F) -> std::result::Result<(), E>
    where
        F: FnMut(EncodedTupleRef<'_>) -> std::result::Result<ControlFlow<()>, E>,
    {
        self.state_mut().counters.iter_calls += 1;
        if self.try_for_each_vector_tuple(&mut f)? {
            return Ok(());
        }
        self.force();
        let mut scratch = KeyScratch::new();
        let mut position = 0usize;
        loop {
            let copied = {
                let state = self.state();
                let Some(map) = state.arena.map_for_node(self.node) else {
                    return Ok(());
                };
                if let Some(key) = state.arena.map_key_at(map, position) {
                    let _ = scratch.set(key.bytes());
                    true
                } else {
                    false
                }
            };
            if !copied {
                break;
            }
            if f(EncodedTupleRef::new(scratch.bytes()))?.is_break() {
                break;
            }
            position += 1;
        }
        Ok(())
    }

    fn fill_batch(&self, cursor: &mut TupleCursor, batch_size: usize) -> TupleBatch {
        let batch_size = batch_size.max(1);
        self.state_mut().counters.iter_calls += 1;
        if let Some(batch) = self.fill_vector_batch(cursor, batch_size) {
            return batch;
        }
        self.force();
        let state = self.state();
        if let Some(map) = state.arena.map_for_node(self.node) {
            let total = state.arena.map_entry_count(map);
            let mut batch = TupleBatch::new();
            while cursor.position < total && batch.len() < batch_size {
                if let Some(key) = state.arena.map_key_at(map, cursor.position) {
                    let _ = batch.push(key.bytes());
                }
                cursor.position += 1;
            }
            batch.exhausted = cursor.position >= total;
            return batch;
        }
        TupleBatch::exhausted()
    }

    fn get(&self, tuple: EncodedTupleRef<'_>) -> Option<Self::Child<'_>> {
        self.force();
        let state = self.state_mut();
        state.counters.get_calls += 1;
        let child = state
            .arena
            .map_for_node(self.node)
            .and_then(|map| state.arena.lookup_map(map, KeyRef::new(tuple.bytes())));
        if child.is_none() {
            state.counters.misses += 1;
        }
        child.map(|node| ColtSource {
            state: self.state,
            node,
        })
    }

    fn key_count(&self) -> KeyCountEstimate {
        let state = self.state();
        match state.arena.node_data(self.node) {
            Some(NodeData::Map(map)) => KeyCountEstimate::Exact(state.arena.map_entry_count(map)),
            Some(data) => KeyCountEstimate::Estimate(state.arena.item_count(data)),
            None => KeyCountEstimate::Estimate(0),
        }
    }
}

impl GhtSource for OwnedColtSource {
    type Child<'a> = ColtSource;

    fn atom(&self) -> Option<AtomOccurrenceId> {
        self.source.atom()
    }

    fn vars(&self) -> &[usize] {
        self.source.vars()
    }

    fn try_for_each_tuple<E, F>(&self, f: F) -> std::result::Result<(), E>
    where
        F: FnMut(EncodedTupleRef<'_>) -> std::result::Result<ControlFlow<()>, E>,
    {
        self.source.try_for_each_tuple(f)
    }

    fn fill_batch(&self, cursor: &mut TupleCursor, batch_size: usize) -> TupleBatch {
        self.source.fill_batch(cursor, batch_size)
    }

    fn get(&self, tuple: EncodedTupleRef<'_>) -> Option<Self::Child<'_>> {
        self.source.get(tuple)
    }

    fn key_count(&self) -> KeyCountEstimate {
        self.source.key_count()
    }
}

impl ColtSource {
    fn try_for_each_vector_tuple<E, F>(&self, f: &mut F) -> std::result::Result<bool, E>
    where
        F: FnMut(EncodedTupleRef<'_>) -> std::result::Result<ControlFlow<()>, E>,
    {
        let state = self.state();
        let node = &state.nodes[self.node.0 as usize];
        if node.level + 1 != state.schemas.len() {
            return Ok(false);
        };
        let schema = &state.schemas[node.level];
        let base = &state.base;
        let data = state.arena.node_data(self.node);
        let mut tuple = InlineTuple::default();
        match data {
            Some(NodeData::Range { start, len }) => {
                for offset in start..start + len {
                    if schema
                        .inline_tuple_from_base_offset(base, offset as usize, &mut tuple)
                        .is_ok()
                        && f(tuple.as_ref())?.is_break()
                    {
                        break;
                    }
                }
                Ok(true)
            }
            Some(NodeData::Offsets(range)) => {
                for offset in state.arena.offsets(range) {
                    if schema
                        .inline_tuple_from_base_offset(base, *offset as usize, &mut tuple)
                        .is_ok()
                        && f(tuple.as_ref())?.is_break()
                    {
                        break;
                    }
                }
                Ok(true)
            }
            Some(NodeData::Singleton { offset }) => {
                if schema
                    .inline_tuple_from_base_offset(base, offset as usize, &mut tuple)
                    .is_ok()
                {
                    let _ = f(tuple.as_ref())?;
                }
                Ok(true)
            }
            Some(NodeData::Map(_)) | None => Ok(false),
        }
    }

    fn fill_vector_batch(&self, cursor: &mut TupleCursor, batch_size: usize) -> Option<TupleBatch> {
        let state = self.state();
        let level = state.nodes[self.node.0 as usize].level;
        if level + 1 != state.schemas.len() {
            return None;
        }
        let schema = &state.schemas[level];
        match state.arena.node_data(self.node) {
            Some(NodeData::Range { start, len }) => {
                let mut batch = TupleBatch::new();
                let len = len as usize;
                while cursor.position < len && batch.len() < batch_size {
                    let offset = start as usize + cursor.position;
                    cursor.position += 1;
                    let _ = batch.push_from_base(schema, &state.base, offset);
                }
                batch.exhausted = cursor.position >= len;
                Some(batch)
            }
            Some(NodeData::Offsets(range)) => {
                let offsets = state.arena.offsets(range);
                let mut batch = TupleBatch::new();
                while cursor.position < offsets.len() && batch.len() < batch_size {
                    let offset = offsets[cursor.position] as usize;
                    cursor.position += 1;
                    let _ = batch.push_from_base(schema, &state.base, offset);
                }
                batch.exhausted = cursor.position >= offsets.len();
                Some(batch)
            }
            Some(NodeData::Singleton { offset }) => {
                if cursor.position > 0 {
                    return Some(TupleBatch::exhausted());
                }
                let mut batch = TupleBatch::new();
                let _ = batch.push_from_base(schema, &state.base, offset as usize);
                cursor.position = 1;
                batch.exhausted = true;
                Some(batch)
            }
            Some(NodeData::Map(_)) | None => None,
        }
    }
}
