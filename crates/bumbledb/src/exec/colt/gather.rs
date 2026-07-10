use super::{unpack_child, Colt, Cursor, NodeState, Positions, Slot, SuffixRun, View};
use crate::image::ColumnView;

impl Colt {
    /// The membership probe's position scan (docs/architecture/
    /// 40-execution.md, § access paths — the point-membership scan):
    /// whether ANY position under `cursor` satisfies every check, each
    /// check the half-open rule `start <= point AND point < end` over
    /// the (start column, end column, point word) triple. Early-exit
    /// scalar by doctrine (irregular control flow, not a reduction) and
    /// `&self` — the scan never forces. Positions typically number the
    /// per-key fanout of a fully-descended cursor; the forced arm exists
    /// for zero-arity gate occurrences whose root a sibling probe forced.
    #[must_use]
    #[allow(clippy::inline_always)]
    // the per-element probe class carries no calls (`scripts/
    // check-asm.sh`); this wrapper only builds the check closure — the
    // recursive walk below is the deliberately-outlined body, and its
    // name is outside the gated class by construction
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

    /// [`Colt::any_position_matches`]'s walk: pinned rows check directly;
    /// unforced nodes walk the view or their chunk chain; a forced node
    /// recurses through its map's children.
    fn any_position(&self, cursor: Cursor, check: &impl Fn(u32) -> bool) -> bool {
        let node = match cursor {
            Cursor::Row(position) => return check(position),
            Cursor::Node(node) => node,
        };
        match self.nodes[node.0 as usize] {
            NodeState::Unforced(Positions::Root) => {
                (0..self.view.len()).any(|idx| check(self.view.position_at(idx)))
            }
            NodeState::Unforced(Positions::Chunks { first, .. }) => {
                let mut chunk = first;
                while chunk != u32::MAX {
                    let c = &self.chunks[chunk as usize];
                    if c.positions[..usize::from(c.len)]
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

    /// Gathers one pinned row's key words at a join level into `out`
    /// (the pinned-leaf elision: the executor skips the
    /// batch machinery for `Cursor::Row` leaves and reads the row
    /// directly).
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: `out` shorter than the
    /// level's arity.
    pub fn gather_row(&self, level: usize, position: u32, out: &mut [u64]) {
        let level = self.selection_levels + level;
        for (i, col) in self.schema_columns[level].iter().enumerate() {
            out[i] = match self.view.image().column(*col) {
                ColumnView::Words(words) => words[position as usize],
                ColumnView::Bytes(bytes) => u64::from(bytes[position as usize]),
            };
        }
    }

    /// The column view backing one key word of a join level — the
    /// scan-fold pushdown reads columns directly instead of copying key
    /// batches.
    #[must_use]
    pub fn suffix_column(&self, level: usize, word: usize) -> ColumnView<'_> {
        self.view
            .image()
            .column(self.schema_columns[self.selection_levels + level][word])
    }

    /// Whether a cursor is an unforced node at a suffix — the scan-fold
    /// pushdown's cheap pre-check, so a fallback to
    /// the batch path never has to unwind a half-opened scan.
    #[must_use]
    pub fn suffix_scannable(&self, cursor: Cursor) -> bool {
        matches!(
            cursor,
            Cursor::Node(node)
                if matches!(self.nodes[node.0 as usize], NodeState::Unforced(_))
        )
    }

    /// Drives `f` over every position run under an **unforced** node at
    /// the given join level (the scan-fold pushdown's position source):
    /// the all-rows root yields one `Identity` run, survivor roots and
    /// chunk chains yield position slices. Returns `false` — with `f`
    /// never called — when the cursor is a pinned row or a forced node
    /// (the caller falls back to the batch path).
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
                    View::Survivors { positions, .. } => f(SuffixRun::Positions(positions)),
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
                    f(SuffixRun::Positions(&c.positions[..usize::from(c.len)]));
                    chunk = c.next;
                }
                true
            }
        }
    }

    /// Column-hoisted gather of one position segment into
    /// `keys_out[out_base..]` + pinned-row children (the unchecked-gather interior).
    #[allow(unsafe_code)]
    pub(super) fn gather_segment(
        &self,
        level: usize,
        segment: &[u32],
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        out_base: usize,
    ) {
        let arity = self.arity_at(level);
        for (i, col) in self.schema_columns[level].iter().enumerate() {
            match self.view.image().column(*col) {
                ColumnView::Words(words) => {
                    debug_assert!(segment.iter().all(|&p| (p as usize) < words.len()));
                    for (k, &position) in segment.iter().enumerate() {
                        // SAFETY: positions index the image the view was
                        // built over — debug-asserted per segment above.
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
        for (k, &position) in segment.iter().enumerate() {
            children_out[out_base + k] = Cursor::Row(position);
        }
    }

    /// The all-rows-view gather: positions are `start..start + take`, so
    /// word columns copy contiguously — no position loads at all.
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
            match self.view.image().column(*col) {
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
