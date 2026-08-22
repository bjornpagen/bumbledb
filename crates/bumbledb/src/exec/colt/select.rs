use super::{CHUNK_LEN, Chunk, Colt, Cursor, NodeRef, NodeState, PoolMark, Positions, hash_words};

impl Colt {
    pub fn select(&mut self, keys: &[Vec<u64>]) -> Option<Cursor> {
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
            cursor = if matches!(self.selection_kinds[level], super::SelectionKind::Set) {
                self.select_union(cursor, level, words)?
            } else {
                debug_assert_eq!(words.len(), self.arity_at(level), "one key per level");
                self.probe_child_at(cursor, level, words, hash_words(words))?
            };
        }
        self.start = super::Start::Selected(cursor);
        Some(cursor)
    }

    /// invariant `union_positions` reads.
    fn select_union(&mut self, cursor: Cursor, level: usize, words: &[u64]) -> Option<Cursor> {
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
        for key in words.chunks_exact(arity) {
            if let Some(child) = self.probe_child_at(cursor, level, key, hash_words(key)) {
                hits.push(child);
            }
        }
        let union = self.union_of(&hits);
        self.select_hits = hits;
        union
    }

    fn union_of(&mut self, hits: &[Cursor]) -> Option<Cursor> {
        let mut positions = std::mem::take(&mut self.select_positions);
        positions.clear();
        for hit in hits {
            self.union_positions(*hit, |position| positions.push(position));
        }
        let cursor = match positions.as_slice() {
            [] => None,
            [only] => Some(Cursor::Row(*only)),
            all => {
                if self.union_mark.is_none() {
                    self.union_mark = Some(self.pool_mark());
                }

                // build verifies it outright before concatenating.
                debug_assert!(
                    {
                        let mut seen = std::collections::BTreeSet::new();
                        all.iter().all(|position| seen.insert(*position))
                    },
                    "positions under distinct keys are disjoint by construction"
                );
                let first = u32::try_from(self.chunks.len()).expect("chunk count fits u32");
                for (idx, segment) in all.chunks(CHUNK_LEN).enumerate() {
                    let start =
                        u32::try_from(self.chunk_positions.len()).expect("position slab fits u32");
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
                Some(Cursor::Node(node))
            }
        };
        self.select_positions = positions;
        cursor
    }

    /// row or an unforced chunk list by the `select_union` invariant.
    fn union_positions(&self, hit: Cursor, mut f: impl FnMut(u32)) {
        match hit {
            Cursor::Row(position) => f(position),
            Cursor::Node(node) => match self.nodes[node.0 as usize] {
                NodeState::Unforced(Positions::Chunks { first, .. }) => {
                    let mut chunk = first;
                    while chunk != u32::MAX {
                        let c = &self.chunks[chunk as usize];
                        for &position in
                            &self.chunk_positions[c.start as usize..][..usize::from(c.len)]
                        {
                            f(position);
                        }
                        chunk = c.next;
                    }
                }
                NodeState::Unforced(Positions::Root) | NodeState::Forced { .. } => {
                    unreachable!("set-level children are unforced chunk lists or pinned rows")
                }
            },
        }
    }

    fn pool_mark(&self) -> PoolMark {
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

    fn truncate_to(&mut self, mark: PoolMark) {
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
