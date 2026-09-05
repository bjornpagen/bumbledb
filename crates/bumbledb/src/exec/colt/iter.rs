use super::{
    BatchToken, BoundView, Colt, Cursor, DENSE_TOKEN_TAG, NodeRef, NodeState, Positions,
    STALE_EPOCH, STALE_TOKEN, Slot, TOKEN_EPOCH_MASK, TOKEN_PAYLOAD_MASK, View, unpack_child,
};

impl Colt {
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
        self.iter_batch_at(
            cursor,
            self.join_index(level),
            token,
            keys_out,
            children_out,
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

    fn iter_batch_at(
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

        assert!(keys_out.len() >= max * arity && children_out.len() >= max);
        match cursor {
            Cursor::Row(position) => {
                let payload = self.token_payload(token);

                if payload > 0 || max == 0 {
                    return Ok((0, token));
                }
                for (i, col) in self.schema_columns[level].iter().enumerate() {
                    keys_out[i] = self.word_at(*col, position);
                }
                children_out[0] = Cursor::Row(position);
                Ok((1, BatchToken(1 | self.epoch_bits())))
            }
            Cursor::Node(node) => {
                let is_suffix = level + 1 == self.schema_columns.len();
                match self.nodes[node.0 as usize] {
                    NodeState::Unforced(_) if is_suffix => {
                        Ok(self.iter_positions(node, level, token, keys_out, children_out, max))
                    }
                    NodeState::Unforced(_) => {
                        let map = self.force(node, level)?;
                        Ok(self.iter_map(map, level, token, keys_out, children_out, max))
                    }
                    NodeState::Forced { map } => {
                        Ok(self.iter_map(map, level, token, keys_out, children_out, max))
                    }
                }
            }
        }
    }

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
                        self.gather_segment(level, segment, keys_out, children_out, 0);
                    }

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

                debug_assert_eq!(packed & !TOKEN_PAYLOAD_MASK, 0);
                (yielded, BatchToken(packed | epoch_bits))
            }
        }
    }

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

        // iteration before this node was forced — reinterpreting it as a

        assert!(
            payload == 0 || payload & DENSE_TOKEN_TAG != 0,
            "{STALE_TOKEN}"
        );
        let start = usize::try_from(payload & !DENSE_TOKEN_TAG).expect("64-bit usize");
        let len = usize::try_from(m.len).expect("64-bit usize");
        let take = max.min(len.saturating_sub(start));

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
