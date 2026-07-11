use super::{hash_words, Chunk, Colt, Cursor, NodeRef, NodeState, PoolMark, Positions, CHUNK_LEN};

impl Colt {
    /// Probes the selection levels with this execution's resolved words,
    /// in level order, forcing lazily exactly like join-level probes.
    /// `keys[level]` holds the level's key words: one word for a scalar
    /// constant, the encoded pair for an interval constant, and the
    /// sorted, deduplicated element words for a set-bound level. The
    /// amortization contract: forcing a selection level walks its node
    /// once per generation; every subsequent constant is O(1) probes —
    /// a set-bound level pays **k probes** (one per element) and unions
    /// the survivor position lists, never re-executing anything per
    /// element (docs/architecture/40-execution.md, § selection levels).
    /// `Some` sits at the first join level; `None` = no fact matches —
    /// the occurrence, and with it the whole conjunctive query, is empty
    /// on this snapshot.
    pub fn select(&mut self, keys: &[Vec<u64>]) -> Option<Cursor> {
        debug_assert_eq!(
            keys.len(),
            self.selection_levels,
            "one resolved key per selection level"
        );
        // Last execution's union subtrie — the position copies and every
        // map the join forced beneath them — is dead now: truncating back
        // to the mark keeps the pools at a fixpoint across set rebinds
        // (capacity retained; see [`PoolMark`]).
        if let Some(mark) = self.union_mark.take() {
            self.truncate_to(mark);
        }
        let mut cursor = Self::root();
        for (level, words) in keys.iter().enumerate() {
            cursor = if self.set_levels[level] {
                self.select_union(cursor, level, words)?
            } else {
                debug_assert_eq!(words.len(), self.arity_at(level), "one key per level");
                self.probe_child_at(cursor, level, words, hash_words(words))?
            };
        }
        self.start = cursor;
        self.selected = true;
        Some(cursor)
    }

    /// One set-bound selection level (docs/architecture/40-execution.md,
    /// § selection levels — param sets ride the selection machinery): k
    /// level-probes, one per element, and the survivor position lists
    /// union by concatenation. Distinct trie keys hold disjoint position
    /// lists by construction of the force pass, and bind sorts and
    /// dedups the element words, so the concatenation IS the union —
    /// asserted, never deduplicated. Per-key children are never returned
    /// (even a single hit is copied), so set-level children stay
    /// unforced chunk lists for the trie's whole lifetime — the
    /// invariant `union_positions` reads.
    fn select_union(&mut self, cursor: Cursor, level: usize, words: &[u64]) -> Option<Cursor> {
        // One key per element: a scalar element is one word, a bytes<N>
        // element its ⌈N/8⌉-word span — the level's arity names the width.
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

    /// Unions the hit children's position lists into one cursor. A
    /// single surviving position pins a row (no node allocated);
    /// anything larger copies into a fresh chunked node — appended past
    /// the union watermark, reclaimed at the next `select`. Gathering
    /// goes through the pooled position scratch (capacity retained —
    /// the concatenation IS the union, see `select_union`).
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
                // Disjointness is structural (distinct keys); the debug
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
                    let mut chunk = Chunk {
                        positions: [0; CHUNK_LEN],
                        len: u8::try_from(segment.len()).expect("CHUNK_LEN fits u8"),
                        next: u32::MAX,
                    };
                    chunk.positions[..segment.len()].copy_from_slice(segment);
                    if idx > 0 {
                        let previous = self.chunks.len() - 1;
                        self.chunks[previous].next =
                            u32::try_from(self.chunks.len()).expect("fits u32");
                    }
                    self.chunks.push(chunk);
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

    /// Drives `f` over a set-level hit's positions. The hit is a pinned
    /// row or an unforced chunk list by the `select_union` invariant.
    fn union_positions(&self, hit: Cursor, mut f: impl FnMut(u32)) {
        match hit {
            Cursor::Row(position) => f(position),
            Cursor::Node(node) => match self.nodes[node.0 as usize] {
                NodeState::Unforced(Positions::Chunks { first, .. }) => {
                    let mut chunk = first;
                    while chunk != u32::MAX {
                        let c = &self.chunks[chunk as usize];
                        for i in 0..usize::from(c.len) {
                            f(c.positions[i]);
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

    /// The current pool high-water ([`PoolMark`]).
    fn pool_mark(&self) -> PoolMark {
        PoolMark {
            nodes: self.nodes.len(),
            chunks: self.chunks.len(),
            maps: self.maps.len(),
            ctrl: self.ctrl.len(),
            buckets: self.buckets.len(),
            dense: self.dense.len(),
        }
    }

    /// Truncates every pool back to a mark. Sound because nothing at or
    /// below the mark references anything past it: post-mark entries are
    /// exactly the union node, its chunk copies, and structures forced
    /// beneath it during one execution's join — all reachable only
    /// through the post-selection start cursor this `select` replaces.
    fn truncate_to(&mut self, mark: PoolMark) {
        self.nodes.truncate(mark.nodes);
        self.chunks.truncate(mark.chunks);
        self.maps.truncate(mark.maps);
        self.ctrl.truncate(mark.ctrl);
        self.buckets.truncate(mark.buckets);
        self.dense.truncate(mark.dense);
    }

    /// The executor's per-execution start cursor: the root, or the
    /// post-selection cursor once [`Colt::select`] ran this execution.
    ///
    /// # Panics
    ///
    /// A release assert: starting a selection-bearing colt before
    /// `select()` would silently drop its selections — wrong results.
    /// Once per occurrence per execution; noise against the join.
    #[must_use]
    pub fn start(&self) -> Cursor {
        assert!(self.selected, "select() runs before the join");
        self.start
    }

    /// The root cursor (level 0).
    #[must_use]
    pub fn root() -> Cursor {
        Cursor::Node(NodeRef(0))
    }
}
