use super::{Colt, Cursor, KeyCount, NodeState, Positions};
use crate::image::ColumnView;

impl Colt {
    #[cfg(test)]
    #[must_use]
    pub fn arity(&self, level: usize) -> usize {
        self.arity_at(self.join_index(level))
    }

    pub(super) fn arity_at(&self, level: usize) -> usize {
        self.schema_columns[level].len()
    }

    #[cfg(test)]
    #[must_use]
    pub fn forced_capacity(&self, cursor: Cursor) -> Option<usize> {
        match cursor {
            Cursor::Row(_) => None,
            Cursor::Node(node) => match self.nodes[node.0 as usize] {
                NodeState::Forced { map } => Some(self.maps[map as usize].capacity()),
                NodeState::Unforced(_) => None,
            },
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn watermark(&self) -> usize {
        self.nodes.len()
            + self.chunks.len()
            + self.chunk_positions.len()
            + self.maps.len()
            + self.ctrl.len()
            + self.buckets.len()
            + self.dense.len()
    }

    #[cfg(test)]
    #[must_use]
    pub fn chunk_footprint_bytes(&self) -> usize {
        self.chunks.len() * std::mem::size_of::<super::Chunk>() + self.chunk_positions.len() * 4
    }

    #[cfg(test)]
    pub fn set_first_chunk_cap(&mut self, cap: u8) {
        assert!(cap >= 2, "the second position allocates the first chunk");
        self.first_chunk_cap = cap;
    }

    #[must_use]
    pub fn probe_footprint_bytes(&self) -> usize {
        self.ctrl.len() + self.buckets.len() * 8 + self.dense.len() * 4
    }

    #[must_use]
    pub fn key_count(&self, cursor: Cursor) -> KeyCount {
        match cursor {
            Cursor::Row(_) => KeyCount::Estimate(1),
            Cursor::Node(node) => match self.nodes[node.0 as usize] {
                NodeState::Forced { map } => {
                    KeyCount::Exact(u64::from(self.maps[map as usize].len))
                }
                NodeState::Unforced(Positions::Root) => KeyCount::Estimate(self.view.len() as u64),
                NodeState::Unforced(Positions::Chunks { count, .. }) => {
                    KeyCount::Estimate(u64::from(count))
                }
            },
        }
    }

    #[inline(always)]
    pub(super) fn word_at(&self, column: usize, position: u32) -> u64 {
        match self.bound_view().image().column(column) {
            ColumnView::Words(words) => words[position as usize],
            ColumnView::Bytes(bytes) => u64::from(bytes[position as usize]),
        }
    }

    #[inline(always)]
    pub(super) fn position_matches(&self, level: usize, position: u32, key: &[u64]) -> bool {
        // arities agree, so the invariant is asserted where the

        debug_assert_eq!(key.len(), self.schema_columns[level].len());
        self.schema_columns[level]
            .iter()
            .zip(key)
            .all(|(col, expected)| self.word_at(*col, position) == *expected)
    }
}
