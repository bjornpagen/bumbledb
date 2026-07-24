use super::{
    BatchToken, Colt, Cursor, DENSE_TOKEN_TAG, NodeRef, NodeState, Positions, STALE_EPOCH,
    STALE_TOKEN, Slot, TOKEN_EPOCH_MASK, TOKEN_PAYLOAD_MASK, View, unpack_child,
};

impl Colt {
    /// Copies up to `max` entries into the caller's buffers, returning the
    /// yielded count and the resume token. `keys_out` receives
    /// `yielded * arity(level)` words; `children_out` one cursor per entry.
    ///
    /// An unforced node iterates its positions directly only at the last
    /// level (the suffix rule, paper §4.2); anywhere else it forces first.
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations: undersized caller buffers.
    pub fn iter_batch(
        &mut self,
        cursor: Cursor,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> (usize, BatchToken) {
        self.iter_batch_at(
            cursor,
            self.selection_levels + level,
            token,
            keys_out,
            children_out,
            max,
        )
    }

    /// The current epoch's token field (bits 56–62): minted into every
    /// nonzero token, asserted on presentation — a token crossing a
    /// [`Colt::reset`] is refused loudly on every arm.
    fn epoch_bits(&self) -> u64 {
        u64::from(self.epoch) << 56
    }

    /// The presentation-side epoch check plus payload strip: every
    /// nonzero token must carry the current epoch.
    fn token_payload(&self, token: BatchToken) -> u64 {
        assert!(
            token.0 == 0 || token.0 & TOKEN_EPOCH_MASK == self.epoch_bits(),
            "{STALE_EPOCH}"
        );
        token.0 & !TOKEN_EPOCH_MASK
    }

    /// [`Colt::iter_batch`] over an internal (selection-inclusive) level.
    fn iter_batch_at(
        &mut self,
        cursor: Cursor,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> (usize, BatchToken) {
        let arity = self.arity_at(level);
        // Caller-buffer contract — a plan-shape invariant, never data:
        // the executor sizes its per-node `entry_keys`/`children`
        // scratch to `batch × level arity` at construction
        // (`Executor::with_batch_size`), the one caller class.
        assert!(keys_out.len() >= max * arity && children_out.len() >= max);
        match cursor {
            Cursor::Row(position) => {
                let payload = self.token_payload(token);
                // `max == 0` yields nothing — the same contract every
                // other arm honors (an over-yield here both violated the
                // contract and wrote past a zero-sized buffer).
                if payload > 0 || max == 0 {
                    return (0, token);
                }
                for (i, col) in self.schema_columns[level].iter().enumerate() {
                    keys_out[i] = self.word_at(*col, position);
                }
                children_out[0] = Cursor::Row(position);
                (1, BatchToken(1 | self.epoch_bits()))
            }
            Cursor::Node(node) => {
                let is_suffix = level + 1 == self.schema_columns.len();
                match self.nodes[node.0 as usize] {
                    NodeState::Unforced(_) if is_suffix => {
                        self.iter_positions(node, level, token, keys_out, children_out, max)
                    }
                    NodeState::Unforced(_) => {
                        let map = self.force(node, level);
                        self.iter_map(map, level, token, keys_out, children_out, max)
                    }
                    NodeState::Forced { map } => {
                        self.iter_map(map, level, token, keys_out, children_out, max)
                    }
                }
            }
        }
    }

    /// Suffix iteration: yield each position's key words with a pinned-row
    /// child — no forcing, no allocation.
    ///
    /// The resume token is O(1) to advance: the root token is a plain view
    /// index; a chunked node's token packs `(chunk + 2, offset)` into the
    /// u64 (0 = start, high half 1 = exhausted) so a drain is O(k), never
    /// the O(k²/64) of re-walking the chain per position.
    ///
    /// Gathers are column-hoisted and unchecked: each
    /// key column resolves its slice once per segment, positions are
    /// debug-asserted in-bounds once, and the interior runs bare loads —
    /// ~1 load per (position, column) instead of an enum match and two
    /// bounds checks each.
    fn iter_positions(
        &mut self,
        node: NodeRef,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> (usize, BatchToken) {
        let payload = self.token_payload(token);
        // A dense-tagged token here means the node was un-forced under an
        // outstanding iteration — impossible within a generation (and a
        // pre-reset token already failed the epoch check above).
        assert!(payload & DENSE_TOKEN_TAG == 0, "{STALE_TOKEN}");
        let epoch_bits = self.epoch_bits();
        match self.nodes[node.0 as usize] {
            NodeState::Forced { .. } => unreachable!("caller checked unforced"),
            NodeState::Unforced(Positions::Root) => {
                let index = usize::try_from(payload).expect("64-bit usize");
                let take = max.min(self.view.len().saturating_sub(index));
                if take == 0 {
                    return (0, token);
                }
                match &self.view {
                    View::Survivors { positions, .. } => {
                        let segment = &positions[index..index + take];
                        self.gather_segment(level, segment, keys_out, children_out, 0);
                    }
                    // The all-rows view: positions ARE the indices — the
                    // fully contiguous gather, no position loads at all.
                    _ => self.gather_identity(level, index, take, keys_out, children_out),
                }
                (take, BatchToken((index + take) as u64 | epoch_bits))
            }
            NodeState::Unforced(Positions::Chunks { first, .. }) => {
                const EXHAUSTED: u64 = 1 << 32;
                let (mut chunk, mut offset) = match payload {
                    0 => (first, 0usize),
                    EXHAUSTED => return (0, token),
                    packed => (
                        u32::try_from((packed >> 32) - 2).expect("packed chunk index"),
                        usize::try_from(packed & 0xFFFF_FFFF).expect("64-bit usize"),
                    ),
                };
                let mut yielded = 0;
                loop {
                    if yielded >= max {
                        break;
                    }
                    let c = &self.chunks[chunk as usize];
                    let len = usize::from(c.len);
                    if offset >= len {
                        if c.next == u32::MAX {
                            return (yielded, BatchToken(EXHAUSTED | epoch_bits));
                        }
                        chunk = c.next;
                        offset = 0;
                        continue;
                    }
                    // One chunk ahead: the chain walk is this loop's only
                    // dependent-load sequence.
                    if c.next != u32::MAX {
                        crate::exec::kernel::prefetch_read(&raw const self.chunks[c.next as usize]);
                    }
                    let take = (len - offset).min(max - yielded);
                    let segment = &self.chunk_positions[c.start as usize + offset..][..take];
                    self.gather_segment(level, segment, keys_out, children_out, yielded);
                    yielded += take;
                    offset += take;
                }
                let packed = (u64::from(chunk) + 2) << 32 | offset as u64;
                // The epoch field (bit 56) and dense tag (bit 63) are
                // unreachable below 2²³ chunks — the map's physical row
                // bound (~5×10⁸ at 32 GiB) sits under it, and the u32
                // position space itself wraps first.
                debug_assert_eq!(packed & !TOKEN_PAYLOAD_MASK, 0);
                (yielded, BatchToken(packed | epoch_bits))
            }
        }
    }

    /// Map iteration: yield `(key words, child)` per occupied slot — the
    /// child comes with the key; no re-probe is possible.
    fn iter_map(
        &self,
        map: u32,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> (usize, BatchToken) {
        let m = self.maps[map as usize];
        let arity = self.arity_at(level);
        debug_assert_eq!(arity, m.arity);
        let payload = self.token_payload(token);
        // Walk the dense occupied list — O(keys), never O(capacity)
        // (docs/architecture/40-execution.md). The token is a tagged
        // dense index: an untagged nonzero token was minted by positions
        // iteration before this node was forced — reinterpreting it as a
        // dense index would silently omit entries (the audit's
        // wrong-results scenario). Once per batch: noise.
        assert!(
            payload == 0 || payload & DENSE_TOKEN_TAG != 0,
            "{STALE_TOKEN}"
        );
        let start = usize::try_from(payload & !DENSE_TOKEN_TAG).expect("64-bit usize");
        let len = usize::try_from(m.len).expect("64-bit usize");
        let take = max.min(len.saturating_sub(start));
        // Hoisted slices: the dense walk touches the
        // occupied list, the key slab, and the slot array — resolved once,
        // with the key line prefetched a few entries ahead (insertion
        // order scatters slots across the map).
        let dense = &self.dense[m.dense_start..m.dense_start + len];
        for k in 0..take {
            let dense_idx = start + k;
            if dense_idx + 8 < len {
                let ahead = usize::try_from(dense[dense_idx + 8]).expect("64-bit usize");
                crate::exec::kernel::prefetch_read(&raw const self.buckets[m.bucket_base(ahead)]);
            }
            let slot_idx = usize::try_from(dense[dense_idx]).expect("64-bit usize");
            for word in 0..arity {
                keys_out[k * arity + word] = self.buckets[m.key_word_at(slot_idx, word)];
            }
            children_out[k] = match unpack_child(self.buckets[m.child_at(slot_idx)]) {
                Slot::Single(position) => Cursor::Row(position),
                Slot::Node(child) => Cursor::Node(child),
            };
        }
        (
            take,
            BatchToken((start + take) as u64 | DENSE_TOKEN_TAG | self.epoch_bits()),
        )
    }
}
