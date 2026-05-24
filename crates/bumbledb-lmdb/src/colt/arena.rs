#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct ColtSourceId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct ColtNodeId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct ColtMapId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct SchemaVarsId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ArenaSourceHandle {
    pub(super) arena_id: ColtSourceId,
    pub(super) node_id: ColtNodeId,
    pub(super) vars_id: SchemaVarsId,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ArenaSourceUndo {
    atom: usize,
    previous: ArenaSourceHandle,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ArenaSourceStore {
    current: Vec<Option<ArenaSourceHandle>>,
    undo: Vec<ArenaSourceUndo>,
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

impl ArenaSourceHandle {
    pub(super) fn new(arena_id: ColtSourceId, node_id: ColtNodeId, vars_id: SchemaVarsId) -> Self {
        Self {
            arena_id,
            node_id,
            vars_id,
        }
    }
}

impl ArenaSourceStore {
    pub(super) fn with_atom_count(atom_count: usize) -> Self {
        Self {
            current: vec![None; atom_count],
            undo: Vec::new(),
        }
    }

    pub(super) fn set_initial(&mut self, atom: usize, source: ArenaSourceHandle) {
        self.ensure_atom(atom);
        self.current[atom] = Some(source);
    }

    pub(super) fn source_for(&self, atom: usize) -> Option<ArenaSourceHandle> {
        self.current.get(atom).copied().flatten()
    }

    pub(super) fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    pub(super) fn replace_source(&mut self, atom: usize, next: ArenaSourceHandle) -> bool {
        let Some(previous) = self.source_for(atom) else {
            return false;
        };
        self.current[atom] = Some(next);
        self.undo.push(ArenaSourceUndo { atom, previous });
        true
    }

    pub(super) fn restore_to(&mut self, mark: usize) {
        while self.undo.len() > mark {
            let Some(entry) = self.undo.pop() else { break };
            self.current[entry.atom] = Some(entry.previous);
        }
    }

    fn ensure_atom(&mut self, atom: usize) {
        if atom >= self.current.len() {
            self.current.resize(atom + 1, None);
        }
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

    #[test]
    fn arena_source_handle_is_compact_copy_state() {
        assert!(std::mem::size_of::<ArenaSourceHandle>() <= 24);
        assert!(std::mem::size_of::<ArenaSourceUndo>() <= 32);
    }

    #[test]
    fn arena_source_store_replaces_and_restores_compact_handles() {
        let root = ArenaSourceHandle::new(ColtSourceId(0), ColtNodeId(0), SchemaVarsId(0));
        let child = ArenaSourceHandle::new(ColtSourceId(0), ColtNodeId(1), SchemaVarsId(1));
        let mut store = ArenaSourceStore::with_atom_count(1);
        store.set_initial(0, root);
        let mark = store.undo_mark();

        assert_eq!(store.source_for(0), Some(root));
        assert!(store.replace_source(0, child));
        assert_eq!(store.source_for(0), Some(child));

        store.restore_to(mark);
        assert_eq!(store.source_for(0), Some(root));
    }

    #[test]
    fn arena_source_store_missing_atom_replacement_is_rejected() {
        let child = ArenaSourceHandle::new(ColtSourceId(0), ColtNodeId(1), SchemaVarsId(1));
        let mut store = ArenaSourceStore::with_atom_count(1);

        assert!(!store.replace_source(0, child));
        assert_eq!(store.source_for(0), None);
    }
}
