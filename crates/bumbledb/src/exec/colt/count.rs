use super::{Colt, Cursor, KeyCount, NodeState, Positions};
use crate::image::ColumnView;

impl Colt {
    /// Key arity at a *join* level (public APIs speak join levels; the
    /// selection prefix is internal). Production callers derive arity
    /// from the plan; this is the test-facing accessor.
    #[cfg(test)]
    #[must_use]
    pub fn arity(&self, level: usize) -> usize {
        self.arity_at(self.selection_levels + level)
    }

    /// Key arity at an internal (selection-inclusive) level.
    pub(super) fn arity_at(&self, level: usize) -> usize {
        self.schema_columns[level].len()
    }

    /// A forced node's map capacity (`None` when unforced) — the test
    /// observability for the sizing formula (docs/architecture/40-execution.md).
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

    /// Total pool footprint — the test observability for laziness
    /// (allocations only ever grow this).
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

    /// The chunk pool's live byte footprint — metadata frames plus the
    /// position slab — the geometry pin's observability (finding 094).
    #[cfg(test)]
    #[must_use]
    pub fn chunk_footprint_bytes(&self) -> usize {
        self.chunks.len() * std::mem::size_of::<super::Chunk>() + self.chunk_positions.len() * 4
    }

    /// Overrides the first-chunk capacity — the geometry pin's A/B
    /// knob: 64 emulates the retired fixed-frame geometry inside the
    /// same slab layout, so the pin isolates the geometry itself.
    #[cfg(test)]
    pub fn set_first_chunk_cap(&mut self, cap: u8) {
        assert!(cap >= 2, "the second position allocates the first chunk");
        self.first_chunk_cap = cap;
    }

    /// Bytes a probe of this trie's forced maps can touch — the
    /// residency proxy for the prefetch tier decision (measured):
    /// software prefetch pays only when the probed structure misses L2
    /// (+7–12% pure loss when it is resident), and the LIVE forced
    /// footprint is a better tier signal than any prepare-time estimate.
    #[must_use]
    pub fn probe_footprint_bytes(&self) -> usize {
        self.ctrl.len() + self.buckets.len() * 8 + self.dense.len() * 4
    }

    /// The labeled key count at a cursor (never forces).
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

    /// Decodes the key word of one column at one position (1-byte columns
    /// widen to u64 — binding slots are words everywhere).
    #[inline(always)]
    pub(super) fn word_at(&self, column: usize, position: u32) -> u64 {
        match self.view.image().column(column) {
            ColumnView::Words(words) => words[position as usize],
            ColumnView::Bytes(bytes) => u64::from(bytes[position as usize]),
        }
    }

    /// Whether the position's key words at `level` equal `key`.
    #[inline(always)]
    pub(super) fn position_matches(&self, level: usize, position: u32, key: &[u64]) -> bool {
        // The zip truncates to the shorter side — correct only when the
        // arities agree, so the invariant is asserted where the
        // truncation lives.
        debug_assert_eq!(key.len(), self.schema_columns[level].len());
        self.schema_columns[level]
            .iter()
            .zip(key)
            .all(|(col, expected)| self.word_at(*col, position) == *expected)
    }
}
