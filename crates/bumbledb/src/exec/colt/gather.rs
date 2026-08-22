use super::{BoundView, Colt, Cursor, NodeState, Positions, Slot, SuffixRun, View, unpack_child};
use crate::image::ColumnView;

impl Colt {
    #[must_use]
    #[expect(
        clippy::inline_always,
        reason = "measured kernel inlining is machine-checked and load-bearing"
    )]
    #[inline(always)]
    pub fn any_position_matches(&self, cursor: Cursor, checks: &[(usize, usize, u64)]) -> bool {
        let check = |position: u32| {
            checks.iter().all(|(start_col, end_col, point)| {
                self.word_at(*start_col, position) <= *point
                    && *point < self.word_at(*end_col, position)
            })
        };
        self.any_position(cursor, &check)
    }

    fn any_position(&self, cursor: Cursor, check: &impl Fn(u32) -> bool) -> bool {
        let node = match cursor {
            Cursor::Row(position) => return check(position),
            Cursor::Node(node) => node,
        };
        match self.nodes[node.0 as usize] {
            NodeState::Unforced(Positions::Root) => {
                (0..self.view.len()).any(|idx| check(self.bound_view().position_at(idx)))
            }
            NodeState::Unforced(Positions::Chunks { first, .. }) => {
                let mut chunk = first;
                while chunk != u32::MAX {
                    let c = &self.chunks[chunk as usize];
                    if self.chunk_positions[c.start as usize..][..usize::from(c.len)]
                        .iter()
                        .any(|position| check(*position))
                    {
                        return true;
                    }
                    chunk = c.next;
                }
                false
            }
            NodeState::Forced { map } => {
                let m = self.maps[map as usize];
                let len = usize::try_from(m.len).expect("64-bit usize");
                self.dense[m.dense_start..m.dense_start + len]
                    .iter()
                    .any(|slot_idx| {
                        let idx = usize::try_from(*slot_idx).expect("64-bit usize");
                        match unpack_child(self.buckets[m.child_at(idx)]) {
                            Slot::Single(position) => check(position),
                            Slot::Node(child) => self.any_position(Cursor::Node(child), check),
                        }
                    })
            }
        }
    }

    /// (the `gather_keys` invariant), bounds-checked here — this gather
    pub fn gather_interval_pair(
        &self,
        start_col: usize,
        end_col: usize,
        positions: &[u32],
        starts: &mut [u64],
        ends: &mut [u64],
    ) {
        self.gather_column(start_col, positions, starts);
        self.gather_column(end_col, positions, ends);
    }

    fn gather_column(&self, col: usize, positions: &[u32], out: &mut [u64]) {
        match self.bound_view().image().column(col) {
            ColumnView::Words(words) => {
                for (j, &position) in positions.iter().enumerate() {
                    out[j] = words[position as usize];
                }
            }
            ColumnView::Bytes(bytes) => {
                for (j, &position) in positions.iter().enumerate() {
                    out[j] = u64::from(bytes[position as usize]);
                }
            }
        }
    }

    /// # Panics

    /// Only on a programmer-invariant violation: `out` shorter than the
    pub fn gather_row(&self, level: usize, position: u32, out: &mut [u64]) {
        let level = self.join_index(level);
        for (i, col) in self.schema_columns[level].iter().enumerate() {
            out[i] = match self.bound_view().image().column(*col) {
                ColumnView::Words(words) => words[position as usize],
                ColumnView::Bytes(bytes) => u64::from(bytes[position as usize]),
            };
        }
    }

    #[must_use]
    pub fn suffix_column(&self, level: usize, word: usize) -> ColumnView<'_> {
        self.bound_view()
            .image()
            .column(self.schema_columns[self.join_index(level)][word])
    }

    #[must_use]
    pub fn suffix_scannable(&self, cursor: Cursor) -> bool {
        matches!(
            cursor,
            Cursor::Node(node)
                if matches!(self.nodes[node.0 as usize], NodeState::Unforced(_))
        )
    }

    pub fn for_each_suffix_run(&self, cursor: Cursor, mut f: impl FnMut(SuffixRun<'_>)) -> bool {
        let Cursor::Node(node) = cursor else {
            return false;
        };
        match self.nodes[node.0 as usize] {
            NodeState::Forced { .. } => false,
            NodeState::Unforced(Positions::Root) => {
                if self.view.is_empty() {
                    return true;
                }
                match &self.view {
                    View::Bound(BoundView::Survivors { positions, .. }) => {
                        f(SuffixRun::Positions(positions));
                    }
                    _ => f(SuffixRun::Identity {
                        start: 0,
                        len: self.view.len(),
                    }),
                }
                true
            }
            NodeState::Unforced(Positions::Chunks { first, .. }) => {
                let mut chunk = first;
                while chunk != u32::MAX {
                    let c = &self.chunks[chunk as usize];
                    if c.next != u32::MAX {
                        crate::exec::kernel::prefetch_read(&raw const self.chunks[c.next as usize]);
                    }
                    f(SuffixRun::Positions(
                        &self.chunk_positions[c.start as usize..][..usize::from(c.len)],
                    ));
                    chunk = c.next;
                }
                true
            }
        }
    }

    pub(super) fn gather_segment(
        &self,
        level: usize,
        segment: &[u32],
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        out_base: usize,
    ) {
        self.gather_keys(level, segment, keys_out, out_base);
        for (k, &position) in segment.iter().enumerate() {
            children_out[out_base + k] = Cursor::Row(position);
        }
    }

    #[expect(
        unsafe_code,
        reason = "the localized unsafe operation has a documented safety invariant"
    )]
    pub(super) fn gather_keys(
        &self,
        level: usize,
        segment: &[u32],
        keys_out: &mut [u64],
        out_base: usize,
    ) {
        let arity = self.arity_at(level);
        for (i, col) in self.schema_columns[level].iter().enumerate() {
            match self.bound_view().image().column(*col) {
                ColumnView::Words(words) => {
                    debug_assert!(segment.iter().all(|&p| (p as usize) < words.len()));
                    for (k, &position) in segment.iter().enumerate() {
                        // SAFETY: `position < words.len()` rests on a
                        // CROSS-MODULE invariant, not a local check:

                        // its view after construction, so no path can

                        // replay and ASAN lanes' 2026-07-20 hard-delete

                        let word = unsafe { *words.get_unchecked(position as usize) };
                        keys_out[(out_base + k) * arity + i] = word;
                    }
                }
                ColumnView::Bytes(bytes) => {
                    debug_assert!(segment.iter().all(|&p| (p as usize) < bytes.len()));
                    for (k, &position) in segment.iter().enumerate() {
                        // SAFETY: as above.
                        let byte = unsafe { *bytes.get_unchecked(position as usize) };
                        keys_out[(out_base + k) * arity + i] = u64::from(byte);
                    }
                }
            }
        }
    }

    pub(super) fn gather_identity(
        &self,
        level: usize,
        start: usize,
        take: usize,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
    ) {
        let arity = self.arity_at(level);
        for (i, col) in self.schema_columns[level].iter().enumerate() {
            match self.bound_view().image().column(*col) {
                ColumnView::Words(words) => {
                    let src = &words[start..start + take];
                    if arity == 1 {
                        keys_out[..take].copy_from_slice(src);
                    } else {
                        for (k, &word) in src.iter().enumerate() {
                            keys_out[k * arity + i] = word;
                        }
                    }
                }
                ColumnView::Bytes(bytes) => {
                    let src = &bytes[start..start + take];
                    for (k, &byte) in src.iter().enumerate() {
                        keys_out[k * arity + i] = u64::from(byte);
                    }
                }
            }
        }
        for (k, position) in (start..start + take).enumerate() {
            children_out[k] = Cursor::Row(u32::try_from(position).expect("positions fit u32"));
        }
    }
}
