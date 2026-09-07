use super::{
    BatchToken, BoundView, Colt, Cursor, DENSE_TOKEN_TAG, Map, NodeRef, NodeState, Positions,
    STALE_EPOCH, STALE_TOKEN, TOKEN_EPOCH_MASK, TOKEN_PAYLOAD_MASK, View, unpack_child,
};

impl Colt {
    /// Force cover tuples before enumeration, even at a terminal level.
    /// Raw terminal positions need not be distinct after hidden fields
    /// were projected away. Pinned rows already contain one tuple.
    pub(crate) fn force_distinct_iteration(
        &mut self,
        cursor: Cursor,
        level: usize,
    ) -> Result<(), crate::work::WorkError> {
        if let Cursor::Node(node) = cursor {
            self.force(node, self.join_index(level))?;
        }
        Ok(())
    }

    /// # Panics
    /// Only on programmer-invariant violations: undersized caller buffers.
    /// # Errors
    /// Returns the force/growth refusal. Do not treat `Ok((0, token))` as
    /// admission failure.
    pub fn iter_batch(
        &mut self,
        cursor: Cursor,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> Result<(usize, BatchToken), crate::work::WorkError> {
        self.iter_batch_at::<true>(
            cursor,
            self.join_index(level),
            token,
            keys_out,
            children_out,
            max,
        )
    }

    /// The same key stream and resume tokens as `iter_batch`, without loading
    /// or materializing child cursors. Leaf sinks consume keys alone unless
    /// an interval-membership probe needs the underlying positions.
    pub(crate) fn iter_keys_batch(
        &mut self,
        cursor: Cursor,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        max: usize,
    ) -> Result<(usize, BatchToken), crate::work::WorkError> {
        self.iter_batch_at::<false>(
            cursor,
            self.join_index(level),
            token,
            keys_out,
            &mut [],
            max,
        )
    }

    /// [`Colt::reset`] is refused loudly on every arm.
    fn epoch_bits(&self) -> u64 {
        u64::from(self.epoch) << 56
    }

    fn token_payload(&self, token: BatchToken) -> u64 {
        assert!(
            token.0 == 0 || token.0 & TOKEN_EPOCH_MASK == self.epoch_bits(),
            "{STALE_EPOCH}"
        );
        token.0 & !TOKEN_EPOCH_MASK
    }

    fn iter_batch_at<const CHILDREN: bool>(
        &mut self,
        cursor: Cursor,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> Result<(usize, BatchToken), crate::work::WorkError> {
        let arity = self.arity_at(level);
        // Caller-buffer contract — a plan-shape invariant, never data:

        let key_words = max.checked_mul(arity).expect("iteration key buffer extent");
        assert!(keys_out.len() >= key_words);
        assert!(!CHILDREN || children_out.len() >= max);
        match cursor {
            Cursor::Row(position) => {
                let payload = self.token_payload(token);

                if payload > 0 || max == 0 {
                    return Ok((0, token));
                }
                for (i, col) in self.schema_columns[level].iter().enumerate() {
                    keys_out[i] = self.word_at(*col, position);
                }
                if CHILDREN {
                    children_out[0] = Cursor::Row(position);
                }
                Ok((1, BatchToken(1 | self.epoch_bits())))
            }
            Cursor::Node(node) => {
                let is_suffix = level + 1 == self.schema_columns.len();
                let map = match self.nodes[node.0 as usize] {
                    NodeState::Unforced(_) if is_suffix => {
                        return Ok(self.iter_positions::<CHILDREN>(
                            node,
                            level,
                            token,
                            keys_out,
                            children_out,
                            max,
                        ));
                    }
                    NodeState::Unforced(_) => self.force(node, level)?,
                    NodeState::Forced { map } => map,
                };
                Ok(self.iter_map::<CHILDREN>(map, level, token, keys_out, children_out, max))
            }
        }
    }

    fn iter_positions<const CHILDREN: bool>(
        &mut self,
        node: NodeRef,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> (usize, BatchToken) {
        let payload = self.token_payload(token);

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
                    View::Bound(BoundView::Survivors { positions, .. }) => {
                        let segment = &positions[index..index + take];
                        self.gather_segment::<CHILDREN>(level, segment, keys_out, children_out, 0);
                    }

                    _ => {
                        self.gather_identity::<CHILDREN>(
                            level,
                            index,
                            take,
                            keys_out,
                            children_out,
                        );
                    }
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

                    if c.next != u32::MAX {
                        crate::exec::kernel::prefetch_read(&raw const self.chunks[c.next as usize]);
                    }
                    let take = (len - offset).min(max - yielded);
                    let segment = &self.chunk_positions[c.start as usize + offset..][..take];
                    self.gather_segment::<CHILDREN>(
                        level,
                        segment,
                        keys_out,
                        children_out,
                        yielded,
                    );
                    yielded += take;
                    offset += take;
                }
                let packed = (u64::from(chunk) + 2) << 32 | offset as u64;

                debug_assert_eq!(packed & !TOKEN_PAYLOAD_MASK, 0);
                (yielded, BatchToken(packed | epoch_bits))
            }
        }
    }

    fn iter_map<const CHILDREN: bool>(
        &self,
        map: u32,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> (usize, BatchToken) {
        let m = &self.maps[map as usize];
        let arity = self.arity_at(level);
        debug_assert_eq!(arity, m.arity);
        let payload = self.token_payload(token);

        // A positions token cannot be reinterpreted as dense-map iteration.
        assert!(
            payload == 0 || payload & DENSE_TOKEN_TAG != 0,
            "{STALE_TOKEN}"
        );
        let start = usize::try_from(payload & !DENSE_TOKEN_TAG).expect("64-bit usize");
        let len = usize::try_from(m.len).expect("64-bit usize");
        let take = max.min(len.saturating_sub(start));

        if take > 0 {
            let keys = &mut keys_out[..take * arity];
            // Match construction/probing's fixed-width kernels. Dispatch once
            // per batch; bucket pitch and the per-key copy are then constants.
            match arity {
                1 => self.copy_map_batch::<1, CHILDREN>(m, start, take, keys, children_out),
                2 => self.copy_map_batch::<2, CHILDREN>(m, start, take, keys, children_out),
                3 => self.copy_map_batch::<3, CHILDREN>(m, start, take, keys, children_out),
                4 => self.copy_map_batch::<4, CHILDREN>(m, start, take, keys, children_out),
                _ => self.copy_map_batch::<0, CHILDREN>(m, start, take, keys, children_out),
            }
        }
        (
            take,
            BatchToken((start + take) as u64 | DENSE_TOKEN_TAG | self.epoch_bits()),
        )
    }

    /// One checked view of this map's pools, shared by fixed and dynamic
    /// widths. `A == 0` retains general keys, including actual zero-width
    /// keys. Row count stays explicit even without keys or child output.
    fn copy_map_batch<const A: usize, const CHILDREN: bool>(
        &self,
        map: &Map,
        start: usize,
        take: usize,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
    ) {
        debug_assert!(A == 0 || A == map.arity);
        let arity = if A == 0 { map.arity } else { A };
        let stride = 8 * (arity + 1);
        let len = usize::try_from(map.len).expect("64-bit usize");
        let dense = &self.dense[map.dense_start..][..len];
        let slots = &dense[start..][..take];
        let buckets = &self.buckets[map.bucket_start..][..map.nbuckets * stride];
        for (k, &slot) in slots.iter().enumerate() {
            let dense_idx = start + k;
            if dense_idx + 8 < len {
                let ahead = usize::try_from(dense[dense_idx + 8]).expect("64-bit usize");
                crate::exec::kernel::prefetch_read(&raw const buckets[(ahead >> 3) * stride]);
            }
            let slot = usize::try_from(slot).expect("64-bit usize");
            let base = (slot >> 3) * stride;
            let lane = slot & 7;
            for word in 0..arity {
                keys_out[k * arity + word] = buckets[base + word * 8 + lane];
            }
            if CHILDREN {
                children_out[k] = unpack_child(buckets[base + 8 * arity + lane]);
            }
        }
    }
}
