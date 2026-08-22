use super::{
    Colt, Map, NodeRef, NodeState, Positions, Slot, ctrl_tag, hash_core, hash_words, pack_child,
};
use crate::image::view::View;

const FORCE_BATCH: usize = 256;

impl Colt {
    pub(super) fn force(&mut self, node: NodeRef, level: usize) -> u32 {
        if let NodeState::Forced { map } = self.nodes[node.0 as usize] {
            return map;
        }
        let arity = self.arity_at(level);
        let count = match self.nodes[node.0 as usize] {
            NodeState::Unforced(Positions::Root) => self.view.len() as u64,
            NodeState::Unforced(Positions::Chunks { count, .. }) => u64::from(count),
            NodeState::Forced { .. } => unreachable!("checked above"),
        };

        // before the pass, so start from the same deterministic guess as

        // buckets for ≤ 0.4 load (the measured occupancy-invariant band):

        let count_usize = usize::try_from(count).expect("64-bit usize");
        let guess = (count_usize / 8).max(16).min(count_usize.max(1) * 2);
        let nbuckets = (guess * 5 / 16).max(1).next_power_of_two();
        let map_idx = u32::try_from(self.maps.len()).expect("map count fits u32");
        let ctrl_start = self.ctrl.len();
        let bucket_start = self.buckets.len();
        let dense_start = self.dense.len();
        self.ctrl.resize(ctrl_start + nbuckets * 8, 0);
        self.buckets
            .resize(bucket_start + nbuckets * (8 * arity + 8), 0);
        let mut m = Map {
            arity,
            nbuckets,
            len: 0,
            ctrl_start,
            bucket_start,
            dense_start,
        };

        let mut keys = std::mem::take(&mut self.stage_keys);
        let mut positions = std::mem::take(&mut self.stage_positions);
        keys.resize(FORCE_BATCH * arity, 0);

        match self.nodes[node.0 as usize] {
            NodeState::Unforced(Positions::Root) => {
                let n = self.view.len();
                let mut base = 0usize;
                while base < n {
                    let take = FORCE_BATCH.min(n - base);
                    positions.clear();
                    positions
                        .extend((base..base + take).map(|idx| self.bound_view().position_at(idx)));
                    self.force_run(&mut m, level, &positions, &mut keys);
                    base += take;
                }
            }
            NodeState::Unforced(Positions::Chunks { first, .. }) => {
                let mut chunk = first;
                while chunk != u32::MAX {
                    let c = self.chunks[chunk as usize];
                    positions.clear();
                    positions.extend_from_slice(
                        &self.chunk_positions[c.start as usize..][..usize::from(c.len)],
                    );
                    self.force_run(&mut m, level, &positions, &mut keys);
                    chunk = c.next;
                }
            }
            NodeState::Forced { .. } => unreachable!("checked above"),
        }
        self.stage_keys = keys;
        self.stage_positions = positions;

        crate::obs::event(
            crate::obs::names::COLT_FORCE,
            crate::obs::TraceArgs::Pair(count, u64::from(m.len)),
        );
        self.maps.push(m);
        self.nodes[node.0 as usize] = NodeState::Forced { map: map_idx };
        map_idx
    }

    /// occurrence pays its root build once, BEFORE its same-shaped
    pub fn force_root(&mut self) {
        if self.schema_columns.is_empty() {
            return;
        }
        self.force(NodeRef(0), 0);
    }

    #[must_use]
    pub fn same_shape(&self, other: &Colt) -> bool {
        self.selection_kinds == other.selection_kinds && self.schema_columns == other.schema_columns
    }

    /// tokens minted against the previous binding stay refused.
    pub fn clone_bound_from(&mut self, other: &Colt, buffer: Vec<u32>) -> View {
        debug_assert!(
            self.same_shape(other),
            "the occurrence dedup clones between identical trie shapes only"
        );
        let old = std::mem::replace(&mut self.view, other.view.clone_in(buffer));
        self.nodes.clone_from(&other.nodes);
        self.chunks.clone_from(&other.chunks);
        self.chunk_positions.clone_from(&other.chunk_positions);
        self.maps.clone_from(&other.maps);
        self.ctrl.clone_from(&other.ctrl);
        self.buckets.clone_from(&other.buckets);
        self.dense.clone_from(&other.dense);
        self.union_mark = other.union_mark;
        self.start = other.start;
        self.epoch = (self.epoch + 1) % 128;
        old
    }

    fn force_run(&mut self, m: &mut Map, level: usize, positions: &[u32], keys: &mut [u64]) {
        self.gather_keys(level, positions, keys, 0);
        match m.arity {
            1 => self.ingest_run::<1>(m, positions, keys),
            2 => self.ingest_run::<2>(m, positions, keys),
            3 => self.ingest_run::<3>(m, positions, keys),
            4 => self.ingest_run::<4>(m, positions, keys),
            _ => self.ingest_run_general(m, positions, keys),
        }
    }

    fn ingest_run<const A: usize>(&mut self, m: &mut Map, positions: &[u32], keys: &[u64]) {
        for (k, &position) in positions.iter().enumerate() {
            let key = &keys[k * A..k * A + A];
            let hash = hash_core::<A>(key);
            self.ingest_one(m, key, hash, position);
        }
    }

    fn ingest_run_general(&mut self, m: &mut Map, positions: &[u32], keys: &[u64]) {
        let arity = m.arity;
        for (k, &position) in positions.iter().enumerate() {
            let key = &keys[k * arity..k * arity + arity];
            let hash = hash_words(key);
            self.ingest_one(m, key, hash, position);
        }
    }

    #[inline(always)]
    fn ingest_one(&mut self, m: &mut Map, key: &[u64], hash: u64, position: u32) {
        // Growth is checked before the probe, so a position that merely

        if (usize::try_from(m.len).expect("64-bit usize") + 1) * 5 > m.nbuckets * 16 {
            self.grow_map(m);
        }
        let (found, idx) = self.probe_hashed(m, key, hash);
        if found {
            self.append_child(m.child_at(idx), position);
        } else {
            self.ctrl[m.ctrl_start + idx] = ctrl_tag(hash);
            for (i, w) in key.iter().enumerate() {
                self.buckets[m.key_word_at(idx, i)] = *w;
            }
            self.buckets[m.child_at(idx)] = pack_child(Slot::Single(position));
            self.dense
                .push(u32::try_from(idx).expect("slot index fits u32"));
            m.len += 1;
        }
    }
}
