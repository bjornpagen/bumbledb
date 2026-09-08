use super::{
    CHUNK_LEN, Chunk, Colt, Cursor, NodeRef, NodeState, PoolMark, Positions, hash_words,
    reserve_pool,
};
use crate::work::WorkError;

impl Colt {
    /// # Errors
    /// Returns the force/growth refusal. A selection miss is `Ok(None)`.
    pub fn select(&mut self, keys: &[impl AsRef<[u64]>]) -> Result<Option<Cursor>, WorkError> {
        debug_assert_eq!(
            keys.len(),
            self.selection_depth(),
            "one resolved key per selection level"
        );

        if let Some(mark) = self.union_mark.take() {
            self.truncate_to(mark);
        }
        let mut cursor = Self::root();
        for (level, words) in keys.iter().enumerate() {
            let words = words.as_ref();
            cursor = if matches!(self.selection_kinds[level], super::SelectionKind::Set) {
                match self.select_union(cursor, level, words)? {
                    Some(hit) => hit,
                    None => return Ok(None),
                }
            } else {
                debug_assert_eq!(words.len(), self.arity_at(level), "one key per level");
                match self.probe_child_at(cursor, level, words, hash_words(words))? {
                    Some(hit) => hit,
                    None => return Ok(None),
                }
            };
        }
        self.start = super::Start::Selected(cursor);
        Ok(Some(cursor))
    }

    /// invariant `union_positions` reads.
    fn select_union(
        &mut self,
        cursor: Cursor,
        level: usize,
        words: &[u64],
    ) -> Result<Option<Cursor>, WorkError> {
        let arity = self.arity_at(level);
        debug_assert_eq!(words.len() % arity, 0, "flat element-major rows");
        debug_assert!(
            words
                .chunks_exact(arity)
                .zip(words.chunks_exact(arity).skip(1))
                .all(|(a, b)| a < b),
            "bind sorts and dedups set elements — distinct keys make the \
             survivor lists disjoint by construction"
        );
        debug_assert!(!words.is_empty(), "an empty set short-circuits at resolve");
        let mut hits = std::mem::take(&mut self.select_hits);
        hits.clear();
        let needed_hits = words.len() / arity.max(1);
        let probed = (|| -> Result<Option<Cursor>, WorkError> {
            reserve_pool(needed_hits, &mut hits, self.work.as_ref())?;
            for (index, key) in words.chunks_exact(arity).enumerate() {
                if index % super::force::FORCE_BATCH == 0 {
                    self.poll_force_batch((needed_hits - index).min(super::force::FORCE_BATCH))?;
                }
                if let Some(child) = self.probe_child_at(cursor, level, key, hash_words(key))? {
                    hits.push(child);
                }
            }
            self.union_of(&hits)
        })();
        self.select_hits = hits;
        probed
    }

    fn union_of(&mut self, hits: &[Cursor]) -> Result<Option<Cursor>, WorkError> {
        let mut positions = std::mem::take(&mut self.select_positions);
        positions.clear();
        let built = (|| -> Result<Option<Cursor>, WorkError> {
            let mut counted = 0usize;
            for batch in hits.chunks(super::force::FORCE_BATCH) {
                self.poll_force_batch(batch.len())?;
                for &hit in batch {
                    // Selection children are unforced, so their estimate is
                    // exactly their position count; no position walk is needed.
                    counted += usize::try_from(self.key_count(hit).magnitude())
                        .expect("selection counts index the resident position arena");
                }
            }
            reserve_pool(counted, &mut positions, self.work.as_ref())?;
            let mut pending_work = 0;
            for hit in hits {
                self.union_positions(*hit, &mut positions, &mut pending_work)?;
            }
            self.poll_force_batch(pending_work)?;
            match positions.as_slice() {
                [] => Ok(None),
                [only] => Ok(Some(Cursor::Row(*only))),
                all => {
                    if self.union_mark.is_none() {
                        self.union_mark = Some(self.pool_mark());
                    }

                    debug_assert!(
                        {
                            let mut seen = std::collections::BTreeSet::new();
                            all.iter().all(|position| seen.insert(*position))
                        },
                        "positions under distinct keys are disjoint by construction"
                    );
                    let pos_needed = self.chunk_positions.len() + all.len();
                    let chunk_needed = self.chunks.len() + all.len().div_ceil(CHUNK_LEN);
                    reserve_pool(pos_needed, &mut self.chunk_positions, self.work.as_ref())?;
                    reserve_pool(chunk_needed, &mut self.chunks, self.work.as_ref())?;
                    reserve_pool(self.nodes.len() + 1, &mut self.nodes, self.work.as_ref())?;
                    let first = u32::try_from(self.chunks.len()).expect("chunk count fits u32");
                    for (idx, segment) in all.chunks(CHUNK_LEN).enumerate() {
                        if idx % (super::force::FORCE_BATCH / CHUNK_LEN) == 0 {
                            self.poll_force_batch(
                                (all.len() - idx * CHUNK_LEN).min(super::force::FORCE_BATCH),
                            )?;
                        }
                        let start = u32::try_from(self.chunk_positions.len())
                            .expect("position slab fits u32");
                        self.chunk_positions.extend_from_slice(segment);
                        let len = u8::try_from(segment.len()).expect("CHUNK_LEN fits u8");
                        if idx > 0 {
                            let previous = self.chunks.len() - 1;
                            self.chunks[previous].next =
                                u32::try_from(self.chunks.len()).expect("fits u32");
                        }
                        self.chunks.push(Chunk {
                            start,
                            cap: len,
                            len,
                            next: u32::MAX,
                        });
                    }
                    let last = u32::try_from(self.chunks.len() - 1).expect("fits u32");
                    let node = NodeRef(u32::try_from(self.nodes.len()).expect("fits u32"));
                    self.nodes.push(NodeState::Unforced(Positions::Chunks {
                        first,
                        last,
                        count: u32::try_from(all.len()).expect("positions fit u32"),
                    }));
                    Ok(Some(Cursor::Node(node)))
                }
            }
        })();
        self.select_positions = positions;
        built
    }

    /// row or an unforced chunk list by the `select_union` invariant.
    fn union_positions(
        &self,
        hit: Cursor,
        out: &mut Vec<u32>,
        pending_work: &mut usize,
    ) -> Result<(), WorkError> {
        match hit {
            Cursor::Row(position) => {
                out.push(position);
                *pending_work += 1;
            }
            Cursor::Node(node) => match self.nodes[node.0 as usize] {
                NodeState::Unforced(Positions::Chunks { first, .. }) => {
                    let mut chunk = first;
                    while chunk != u32::MAX {
                        let c = &self.chunks[chunk as usize];
                        out.extend_from_slice(
                            &self.chunk_positions[c.start as usize..][..usize::from(c.len)],
                        );
                        *pending_work += usize::from(c.len);
                        if *pending_work >= super::force::FORCE_BATCH {
                            self.poll_force_batch(*pending_work)?;
                            *pending_work = 0;
                        }
                        chunk = c.next;
                    }
                }
                NodeState::Unforced(Positions::Root) | NodeState::Forced { .. } => {
                    unreachable!("set-level children are unforced chunk lists or pinned rows")
                }
            },
        }
        if *pending_work >= super::force::FORCE_BATCH {
            self.poll_force_batch(*pending_work)?;
            *pending_work = 0;
        }
        Ok(())
    }

    pub(super) fn pool_mark(&self) -> PoolMark {
        PoolMark {
            nodes: self.nodes.len(),
            chunks: self.chunks.len(),
            chunk_positions: self.chunk_positions.len(),
            maps: self.maps.len(),
            ctrl: self.ctrl.len(),
            buckets: self.buckets.len(),
            dense: self.dense.len(),
        }
    }

    pub(super) fn truncate_to(&mut self, mark: PoolMark) {
        self.nodes.truncate(mark.nodes);
        self.chunks.truncate(mark.chunks);
        self.chunk_positions.truncate(mark.chunk_positions);
        self.maps.truncate(mark.maps);
        self.ctrl.truncate(mark.ctrl);
        self.buckets.truncate(mark.buckets);
        self.dense.truncate(mark.dense);
    }

    /// # Panics
    /// `select()` would silently drop its selections — wrong results.
    #[must_use]
    pub fn start(&self) -> Cursor {
        match self.start {
            super::Start::Vacuous(cursor) | super::Start::Selected(cursor) => cursor,
            super::Start::Pending => panic!("select() runs before the join"),
        }
    }

    #[must_use]
    pub fn root() -> Cursor {
        Cursor::Node(NodeRef(0))
    }
}
