#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct ColtSourceId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct ColtNodeId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct ColtMapId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct OffsetRange {
    pub(super) start: u32,
    pub(super) len: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NodeData {
    Range { start: u32, len: u32 },
    Singleton { offset: u32 },
    Offsets(OffsetRange),
    Map(ColtMapId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ColtNodeRecord {
    pub(super) data: NodeData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ColtMapRecord {
    pub(super) entries: OffsetRange,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ColtArena {
    nodes: Vec<ColtNodeRecord>,
    maps: Vec<ColtMapRecord>,
    offsets: Vec<u32>,
}

impl ColtArena {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn add_range_node(&mut self, start: u32, len: u32) -> ColtNodeId {
        self.push_node(NodeData::Range { start, len })
    }

    pub(super) fn add_singleton_node(&mut self, offset: u32) -> ColtNodeId {
        self.push_node(NodeData::Singleton { offset })
    }

    pub(super) fn add_offsets_node(&mut self, offsets: &[u32]) -> ColtNodeId {
        let range = self.append_offsets(offsets);
        self.push_node(NodeData::Offsets(range))
    }

    pub(super) fn add_map_placeholder_node(&mut self) -> ColtNodeId {
        let map = self.add_map(OffsetRange { start: 0, len: 0 });
        self.push_node(NodeData::Map(map))
    }

    pub(super) fn node(&self, id: ColtNodeId) -> Option<&ColtNodeRecord> {
        self.nodes.get(id.0 as usize)
    }

    pub(super) fn append_offsets(&mut self, offsets: &[u32]) -> OffsetRange {
        let start = self.offsets.len() as u32;
        self.offsets.extend_from_slice(offsets);
        OffsetRange {
            start,
            len: offsets.len() as u32,
        }
    }

    pub(super) fn offsets(&self, range: OffsetRange) -> &[u32] {
        let start = range.start as usize;
        let end = start + range.len as usize;
        &self.offsets[start..end]
    }

    fn push_node(&mut self, data: NodeData) -> ColtNodeId {
        let id = ColtNodeId(self.nodes.len() as u32);
        self.nodes.push(ColtNodeRecord { data });
        id
    }

    fn add_map(&mut self, entries: OffsetRange) -> ColtMapId {
        let id = ColtMapId(self.maps.len() as u32);
        self.maps.push(ColtMapRecord { entries });
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_creates_all_node_data_variants() {
        let mut arena = ColtArena::new();

        let range = arena.add_range_node(0, 10);
        let singleton = arena.add_singleton_node(7);
        let offsets = arena.add_offsets_node(&[2, 4, 8]);
        let map = arena.add_map_placeholder_node();

        assert_eq!(
            arena.node(range).map(|node| node.data),
            Some(NodeData::Range { start: 0, len: 10 })
        );
        assert_eq!(
            arena.node(singleton).map(|node| node.data),
            Some(NodeData::Singleton { offset: 7 })
        );
        assert_eq!(
            arena.node(offsets).map(|node| node.data),
            Some(NodeData::Offsets(OffsetRange { start: 0, len: 3 }))
        );
        assert_eq!(
            arena.node(map).map(|node| node.data),
            Some(NodeData::Map(ColtMapId(0)))
        );
    }

    #[test]
    fn arena_node_ids_remain_stable_after_insertions() {
        let mut arena = ColtArena::new();
        let first = arena.add_singleton_node(1);
        let second = arena.add_range_node(0, 2);

        for offset in 10..100 {
            let _ = arena.add_singleton_node(offset);
        }

        assert_eq!(
            arena.node(first).map(|node| node.data),
            Some(NodeData::Singleton { offset: 1 })
        );
        assert_eq!(
            arena.node(second).map(|node| node.data),
            Some(NodeData::Range { start: 0, len: 2 })
        );
    }

    #[test]
    fn arena_offset_ranges_read_back_exact_offsets() {
        let mut arena = ColtArena::new();

        let first = arena.append_offsets(&[1, 3, 5]);
        let second = arena.append_offsets(&[8, 13]);

        assert_eq!(arena.offsets(first), &[1, 3, 5]);
        assert_eq!(arena.offsets(second), &[8, 13]);
    }

    #[test]
    fn arena_empty_ranges_and_singletons_are_distinct() {
        let mut arena = ColtArena::new();
        let empty = arena.add_offsets_node(&[]);
        let singleton = arena.add_singleton_node(0);

        assert_eq!(
            arena.node(empty).map(|node| node.data),
            Some(NodeData::Offsets(OffsetRange { start: 0, len: 0 }))
        );
        assert_eq!(
            arena.node(singleton).map(|node| node.data),
            Some(NodeData::Singleton { offset: 0 })
        );
        assert_ne!(
            arena.node(empty).map(|node| node.data),
            arena.node(singleton).map(|node| node.data)
        );
    }
}
