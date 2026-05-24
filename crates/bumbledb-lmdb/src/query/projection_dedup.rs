use std::hash::{Hash, Hasher};

use bumbledb_core::query_ir::TypedFindTerm;

use crate::Result;
use crate::colt::KeyOwned;
use crate::query::model::NormalizedQuery;
use crate::query::sink::Binding;

const PROJECTION_EMPTY: u32 = u32::MAX;

#[derive(Clone, Debug)]
pub(super) struct ProjectionScratch {
    inline: [u8; 128],
    heap: Vec<u8>,
    len: usize,
}

impl Default for ProjectionScratch {
    fn default() -> Self {
        Self {
            inline: [0; 128],
            heap: Vec::new(),
            len: 0,
        }
    }
}

impl ProjectionScratch {
    pub(super) fn encoded_projection<'scratch>(
        &'scratch mut self,
        query: &NormalizedQuery,
        binding: &Binding,
    ) -> Result<Option<&'scratch [u8]>> {
        self.clear();
        for term in &query.find {
            match term {
                TypedFindTerm::Variable { variable } => {
                    let Some(bytes) = binding.value(*variable) else {
                        return Ok(None);
                    };
                    self.extend_from_slice(bytes);
                }
            }
        }
        Ok(Some(if self.heap.is_empty() {
            &self.inline[..self.len]
        } else {
            &self.heap
        }))
    }

    fn clear(&mut self) {
        self.len = 0;
        self.heap.clear();
    }

    fn extend_from_slice(&mut self, bytes: &[u8]) {
        let next_len = self.len + bytes.len();
        if self.heap.is_empty() && next_len <= self.inline.len() {
            self.inline[self.len..next_len].copy_from_slice(bytes);
            self.len = next_len;
            return;
        }
        if self.heap.is_empty() {
            self.heap.extend_from_slice(&self.inline[..self.len]);
        }
        self.heap.extend_from_slice(bytes);
        self.len = next_len;
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct ProjectionDedup {
    buckets: Vec<u32>,
    entries: Vec<ProjectionEntry>,
}

#[derive(Clone, Debug)]
struct ProjectionEntry {
    hash: u64,
    key: KeyOwned,
    next: u32,
}

impl ProjectionDedup {
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn contains(&self, key: &[u8]) -> bool {
        if self.buckets.is_empty() {
            return false;
        }
        let hash = hash_projection(key);
        let mut entry = self.buckets[bucket_index(hash, self.buckets.len())];
        while entry != PROJECTION_EMPTY {
            let candidate = &self.entries[entry as usize];
            if candidate.hash == hash && candidate.key.bytes() == key {
                return true;
            }
            entry = candidate.next;
        }
        false
    }

    pub(super) fn insert(&mut self, key: &[u8]) -> bool {
        self.ensure_capacity();
        let hash = hash_projection(key);
        let bucket = bucket_index(hash, self.buckets.len());
        let mut entry = self.buckets[bucket];
        while entry != PROJECTION_EMPTY {
            let candidate = &self.entries[entry as usize];
            if candidate.hash == hash && candidate.key.bytes() == key {
                return false;
            }
            entry = candidate.next;
        }
        let index = self.entries.len() as u32;
        self.entries.push(ProjectionEntry {
            hash,
            key: KeyOwned::from_slice(key),
            next: self.buckets[bucket],
        });
        self.buckets[bucket] = index;
        true
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &[u8]> {
        self.entries.iter().map(|entry| entry.key.bytes())
    }

    fn ensure_capacity(&mut self) {
        if self.buckets.is_empty() {
            self.buckets.resize(16, PROJECTION_EMPTY);
            return;
        }
        if self.entries.len() * 4 < self.buckets.len() * 3 {
            return;
        }
        let mut buckets = vec![PROJECTION_EMPTY; self.buckets.len() * 2];
        for (index, entry) in self.entries.iter_mut().enumerate() {
            let bucket = bucket_index(entry.hash, buckets.len());
            entry.next = buckets[bucket];
            buckets[bucket] = index as u32;
        }
        self.buckets = buckets;
    }
}

fn hash_projection(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn bucket_index(hash: u64, buckets: usize) -> usize {
    hash as usize & (buckets - 1)
}
