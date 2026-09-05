use super::{
    Colt, Map, NodeRef, NodeState, Positions, Slot, ctrl_tag, hash_core, hash_words, pack_child,
    reserve_pool,
};
use crate::image::view::View;
use crate::work::WorkError;

pub(super) const FORCE_BATCH: usize = 256;

pub(super) fn force_nbuckets(count: usize) -> usize {
    let guess = (count / 8).max(16).min(count.max(1) * 2);
    (guess * 5 / 16).max(1).next_power_of_two()
}

impl Colt {
    /// Force an unforced node into a map. Admission refusal is returned
    /// before any caller can index the map pool.
    pub(crate) fn force(&mut self, node: NodeRef, level: usize) -> Result<u32, WorkError> {
        if let NodeState::Forced { map } = self.nodes[node.0 as usize] {
            return Ok(map);
        }
        let mark = self.pool_mark();
        match self.force_fresh(node, level) {
            Ok(map) => Ok(map),
            Err(error) => {
                self.truncate_to(mark);
                Err(error)
            }
        }
    }

    fn force_fresh(&mut self, node: NodeRef, level: usize) -> Result<u32, WorkError> {
        let arity = self.arity_at(level);
        let count = match self.nodes[node.0 as usize] {
            NodeState::Unforced(Positions::Root) => self.view.len() as u64,
            NodeState::Unforced(Positions::Chunks { count, .. }) => u64::from(count),
            NodeState::Forced { .. } => unreachable!("checked above"),
        };

        let count_usize = usize::try_from(count).expect("64-bit usize");
        let nbuckets = force_nbuckets(count_usize);
        let ctrl_needed = self.ctrl.len() + nbuckets * 8;
        let bucket_needed = self.buckets.len() + nbuckets * (8 * arity + 8);
        reserve_pool(
            ctrl_needed,
            &mut self.ctrl,
            self.work.as_ref(),
            &mut self.charges,
        )?;
        reserve_pool(
            bucket_needed,
            &mut self.buckets,
            self.work.as_ref(),
            &mut self.charges,
        )?;
        let ctrl_start = self.ctrl.len();
        let bucket_start = self.buckets.len();
        let dense_start = self.dense.len();
        self.ctrl.resize(ctrl_needed, 0);
        self.buckets.resize(bucket_needed, 0);
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
        let filled = self.force_fill(node, level, &mut m, &mut keys, &mut positions);
        self.stage_keys = keys;
        self.stage_positions = positions;
        filled?;

        reserve_pool(
            self.maps.len() + 1,
            &mut self.maps,
            self.work.as_ref(),
            &mut self.charges,
        )?;
        let map_idx = u32::try_from(self.maps.len()).expect("map count fits u32");
        crate::obs::event(
            crate::obs::names::COLT_FORCE,
            crate::obs::TraceArgs::Pair(count, u64::from(m.len)),
        );
        self.maps.push(m);
        self.nodes[node.0 as usize] = NodeState::Forced { map: map_idx };
        Ok(map_idx)
    }

    fn force_fill(
        &mut self,
        node: NodeRef,
        level: usize,
        m: &mut Map,
        keys: &mut Vec<u64>,
        positions: &mut Vec<u32>,
    ) -> Result<(), WorkError> {
        let arity = m.arity;
        let stage_needed = FORCE_BATCH * arity;
        reserve_pool(stage_needed, keys, self.work.as_ref(), &mut self.charges)?;
        keys.resize(stage_needed, 0);

        match self.nodes[node.0 as usize] {
            NodeState::Unforced(Positions::Root) => {
                let n = self.view.len();
                let mut base = 0usize;
                if n == 0 {
                    self.poll_force_batch(0)?;
                }
                while base < n {
                    let take = FORCE_BATCH.min(n - base);
                    self.poll_force_batch(take)?;
                    positions.clear();
                    reserve_pool(take, positions, self.work.as_ref(), &mut self.charges)?;
                    positions
                        .extend((base..base + take).map(|idx| self.bound_view().position_at(idx)));
                    self.force_run(m, level, positions, keys)?;
                    base += take;
                }
            }
            NodeState::Unforced(Positions::Chunks { first, .. }) => {
                let mut chunk = first;
                if chunk == u32::MAX {
                    self.poll_force_batch(0)?;
                }
                while chunk != u32::MAX {
                    let c = self.chunks[chunk as usize];
                    let take = usize::from(c.len);
                    self.poll_force_batch(take)?;
                    positions.clear();
                    reserve_pool(take, positions, self.work.as_ref(), &mut self.charges)?;
                    positions.extend_from_slice(&self.chunk_positions[c.start as usize..][..take]);
                    self.force_run(m, level, positions, keys)?;
                    chunk = c.next;
                }
            }
            NodeState::Forced { .. } => unreachable!("checked above"),
        }
        Ok(())
    }

    /// occurrence pays its root build once, BEFORE its same-shaped
    ///
    /// # Errors
    /// Returns the work refusal that stopped force or growth. Callers must
    /// propagate it before reading maps or chunks.
    pub fn force_root(&mut self) -> Result<(), WorkError> {
        if self.schema_columns.is_empty() {
            return Ok(());
        }
        self.force(NodeRef(0), 0).map(|_| ())
    }

    #[must_use]
    pub fn same_shape(&self, other: &Colt) -> bool {
        self.selection_kinds == other.selection_kinds && self.schema_columns == other.schema_columns
    }

    /// tokens minted against the previous binding stay refused.
    ///
    /// # Errors
    /// Refuses when destination pool capacity would grow past the bound ledger.
    pub fn clone_bound_from(
        &mut self,
        other: &Colt,
        mut buffer: Vec<u32>,
    ) -> Result<View, WorkError> {
        debug_assert!(
            self.same_shape(other),
            "the occurrence dedup clones between identical trie shapes only"
        );
        let survivor_needed = match &other.view {
            View::Bound(crate::image::view::BoundView::Survivors { positions, .. }) => {
                positions.len()
            }
            View::Unbound | View::Bound(crate::image::view::BoundView::All(_)) => 0,
        };
        reserve_pool(
            other.nodes.len(),
            &mut self.nodes,
            self.work.as_ref(),
            &mut self.charges,
        )?;
        reserve_pool(
            other.chunks.len(),
            &mut self.chunks,
            self.work.as_ref(),
            &mut self.charges,
        )?;
        reserve_pool(
            other.chunk_positions.len(),
            &mut self.chunk_positions,
            self.work.as_ref(),
            &mut self.charges,
        )?;
        reserve_pool(
            other.maps.len(),
            &mut self.maps,
            self.work.as_ref(),
            &mut self.charges,
        )?;
        reserve_pool(
            other.ctrl.len(),
            &mut self.ctrl,
            self.work.as_ref(),
            &mut self.charges,
        )?;
        reserve_pool(
            other.buckets.len(),
            &mut self.buckets,
            self.work.as_ref(),
            &mut self.charges,
        )?;
        reserve_pool(
            other.dense.len(),
            &mut self.dense,
            self.work.as_ref(),
            &mut self.charges,
        )?;
        reserve_pool(
            survivor_needed,
            &mut buffer,
            self.work.as_ref(),
            &mut self.charges,
        )?;
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
        Ok(old)
    }

    fn force_run(
        &mut self,
        m: &mut Map,
        level: usize,
        positions: &[u32],
        keys: &mut [u64],
    ) -> Result<(), WorkError> {
        self.gather_keys(level, positions, keys, 0);
        match m.arity {
            1 => self.ingest_run::<1>(m, positions, keys),
            2 => self.ingest_run::<2>(m, positions, keys),
            3 => self.ingest_run::<3>(m, positions, keys),
            4 => self.ingest_run::<4>(m, positions, keys),
            _ => self.ingest_run_general(m, positions, keys),
        }
    }

    fn ingest_run<const A: usize>(
        &mut self,
        m: &mut Map,
        positions: &[u32],
        keys: &[u64],
    ) -> Result<(), WorkError> {
        for (k, &position) in positions.iter().enumerate() {
            let key = &keys[k * A..k * A + A];
            let hash = hash_core::<A>(key);
            self.ingest_one(m, key, hash, position)?;
        }
        Ok(())
    }

    fn ingest_run_general(
        &mut self,
        m: &mut Map,
        positions: &[u32],
        keys: &[u64],
    ) -> Result<(), WorkError> {
        let arity = m.arity;
        for (k, &position) in positions.iter().enumerate() {
            let key = &keys[k * arity..k * arity + arity];
            let hash = hash_words(key);
            self.ingest_one(m, key, hash, position)?;
        }
        Ok(())
    }

    pub(super) fn ingest_one(
        &mut self,
        m: &mut Map,
        key: &[u64],
        hash: u64,
        position: u32,
    ) -> Result<(), WorkError> {
        if (usize::try_from(m.len).expect("64-bit usize") + 1) * 5 > m.nbuckets * 16 {
            self.grow_map(m)?;
        }
        let (found, idx) = self.probe_hashed(m, key, hash);
        if found {
            self.append_child(m.child_at(idx), position)?;
        } else {
            reserve_pool(
                self.dense.len() + 1,
                &mut self.dense,
                self.work.as_ref(),
                &mut self.charges,
            )?;
            self.ctrl[m.ctrl_start + idx] = ctrl_tag(hash);
            for (i, w) in key.iter().enumerate() {
                self.buckets[m.key_word_at(idx, i)] = *w;
            }
            self.buckets[m.child_at(idx)] = pack_child(Slot::Single(position));
            self.dense
                .push(u32::try_from(idx).expect("slot index fits u32"));
            m.len += 1;
        }
        Ok(())
    }
}
