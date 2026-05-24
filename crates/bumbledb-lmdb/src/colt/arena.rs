#![allow(dead_code)]

use std::hash::{Hash, Hasher};

use super::key::{KeyOwned, KeyRef};

const MAP_EMPTY: u32 = u32::MAX;

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
    pub(super) table: MapTable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct MapTable {
    pub(super) buckets: OffsetRange,
    pub(super) entries: OffsetRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MapEntry {
    pub(super) hash: u64,
    pub(super) key: KeyOwned,
    pub(super) child: ColtNodeId,
    pub(super) next: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ColtArena {
    nodes: Vec<ColtNodeRecord>,
    maps: Vec<ColtMapRecord>,
    offsets: Vec<u32>,
    map_buckets: Vec<u32>,
    map_entries: Vec<MapEntry>,
}

pub(super) enum OffsetIter<'arena> {
    Range { next: u32, end: u32 },
    Singleton(Option<u32>),
    Slice(std::iter::Copied<std::slice::Iter<'arena, u32>>),
    Empty,
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

    pub(super) fn add_full_source_node(&mut self, len: u32) -> ColtNodeId {
        self.push_node(NodeData::Range { start: 0, len })
    }

    pub(super) fn child_offsets_data(&mut self, offsets: &[u32]) -> NodeData {
        match offsets {
            [] => NodeData::Offsets(OffsetRange { start: 0, len: 0 }),
            [offset] => NodeData::Singleton { offset: *offset },
            offsets => NodeData::Offsets(self.append_offsets(offsets)),
        }
    }

    pub(super) fn add_map_placeholder_node(&mut self) -> ColtNodeId {
        let map = self.add_map_table(0, 0);
        self.push_node(NodeData::Map(map))
    }

    pub(super) fn add_map_table(&mut self, bucket_hint: usize, entry_hint: usize) -> ColtMapId {
        let bucket_len = bucket_hint.max(1).next_power_of_two();
        let bucket_start = self.map_buckets.len() as u32;
        self.map_buckets
            .resize(self.map_buckets.len() + bucket_len, MAP_EMPTY);
        let entry_start = self.map_entries.len() as u32;
        self.map_entries.reserve(entry_hint);
        let id = ColtMapId(self.maps.len() as u32);
        self.maps.push(ColtMapRecord {
            table: MapTable {
                buckets: OffsetRange {
                    start: bucket_start,
                    len: bucket_len as u32,
                },
                entries: OffsetRange {
                    start: entry_start,
                    len: 0,
                },
            },
        });
        id
    }

    pub(super) fn insert_map_entry(
        &mut self,
        map: ColtMapId,
        key: KeyRef<'_>,
        child: ColtNodeId,
    ) -> ColtNodeId {
        if let Some(existing) = self.lookup_map(map, key) {
            return existing;
        }
        let hash = hash_key(key.bytes());
        let bucket = self.bucket_index(map, hash);
        let next = self.map_buckets[bucket];
        let entry_index = self.map_entries.len() as u32;
        self.map_entries.push(MapEntry {
            hash,
            key: KeyOwned::from_slice(key.bytes()),
            child,
            next,
        });
        self.map_buckets[bucket] = entry_index;
        self.maps[map.0 as usize].table.entries.len += 1;
        child
    }

    pub(super) fn lookup_map(&self, map: ColtMapId, key: KeyRef<'_>) -> Option<ColtNodeId> {
        let hash = hash_key(key.bytes());
        let mut entry = self.map_buckets[self.bucket_index(map, hash)];
        while entry != MAP_EMPTY {
            let candidate = &self.map_entries[entry as usize];
            if candidate.hash == hash && candidate.key.bytes() == key.bytes() {
                return Some(candidate.child);
            }
            entry = candidate.next;
        }
        None
    }

    pub(super) fn map_entry_count(&self, map: ColtMapId) -> usize {
        self.maps[map.0 as usize].table.entries.len as usize
    }

    pub(super) fn force_node_with_key_fn(
        &mut self,
        node: ColtNodeId,
        mut key_for_offset: impl FnMut(u32, &mut Vec<u8>),
    ) -> ColtMapId {
        if let NodeData::Map(map) = self.nodes[node.0 as usize].data {
            return map;
        }
        let data = self.nodes[node.0 as usize].data;
        let count = self.offset_count(data);
        let map = self.add_map_table(count, count);
        let mut key = Vec::new();
        self.for_each_offset(data, |arena, offset| {
            key_for_offset(offset, &mut key);
            let key_ref = KeyRef::new(&key);
            let child = match arena.lookup_map(map, key_ref) {
                Some(child) => {
                    arena.push_offset_to_child(child, offset);
                    child
                }
                None => arena.add_singleton_node(offset),
            };
            arena.insert_map_entry(map, key_ref, child);
        });
        self.nodes[node.0 as usize].data = NodeData::Map(map);
        map
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

    pub(super) fn iter_offsets(&self, data: NodeData) -> OffsetIter<'_> {
        match data {
            NodeData::Range { start, len } => OffsetIter::Range {
                next: start,
                end: start + len,
            },
            NodeData::Singleton { offset } => OffsetIter::Singleton(Some(offset)),
            NodeData::Offsets(range) => OffsetIter::Slice(self.offsets(range).iter().copied()),
            NodeData::Map(_) => OffsetIter::Empty,
        }
    }

    pub(super) fn offset_pool_len(&self) -> usize {
        self.offsets.len()
    }

    fn push_node(&mut self, data: NodeData) -> ColtNodeId {
        let id = ColtNodeId(self.nodes.len() as u32);
        self.nodes.push(ColtNodeRecord { data });
        id
    }

    fn bucket_index(&self, map: ColtMapId, hash: u64) -> usize {
        let buckets = self.maps[map.0 as usize].table.buckets;
        buckets.start as usize + (hash as usize & (buckets.len as usize - 1))
    }

    fn offset_count(&self, data: NodeData) -> usize {
        match data {
            NodeData::Range { len, .. } => len as usize,
            NodeData::Singleton { .. } => 1,
            NodeData::Offsets(range) => range.len as usize,
            NodeData::Map(map) => self.map_entry_count(map),
        }
    }

    fn for_each_offset(&mut self, data: NodeData, mut f: impl FnMut(&mut Self, u32)) {
        match data {
            NodeData::Range { start, len } => {
                for offset in start..start + len {
                    f(self, offset);
                }
            }
            NodeData::Singleton { offset } => f(self, offset),
            NodeData::Offsets(range) => {
                for index in range.start..range.start + range.len {
                    f(self, self.offsets[index as usize]);
                }
            }
            NodeData::Map(_) => {}
        }
    }

    fn push_offset_to_child(&mut self, child: ColtNodeId, offset: u32) {
        let data = self.nodes[child.0 as usize].data;
        self.nodes[child.0 as usize].data = match data {
            NodeData::Singleton { offset: first } if first == offset => data,
            NodeData::Singleton { offset: first } => {
                let range = self.append_offsets(&[first, offset]);
                NodeData::Offsets(range)
            }
            NodeData::Offsets(mut range)
                if range.start + range.len == self.offsets.len() as u32 =>
            {
                self.offsets.push(offset);
                range.len += 1;
                NodeData::Offsets(range)
            }
            NodeData::Offsets(range) => {
                let mut offsets = self.offsets(range).to_vec();
                offsets.push(offset);
                NodeData::Offsets(self.append_offsets(&offsets))
            }
            _ => data,
        };
    }
}

fn hash_key(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

impl Iterator for OffsetIter<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            OffsetIter::Range { next, end } => {
                if *next >= *end {
                    None
                } else {
                    let output = *next;
                    *next += 1;
                    Some(output)
                }
            }
            OffsetIter::Singleton(offset) => offset.take(),
            OffsetIter::Slice(iter) => iter.next(),
            OffsetIter::Empty => None,
        }
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
#[path = "arena_tests.rs"]
mod tests;
