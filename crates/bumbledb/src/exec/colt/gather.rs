use super::{Colt, Cursor, NodeState, SuffixRun, Positions, View};
use crate::image::ColumnView;

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
    /// Gathers one pinned row's key words at a join level into `out`
    /// (docs/perf/ PRD 05's pinned-leaf elision: the executor skips the
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
    /// batches (docs/perf/ PRD 05).
    #[must_use]
    pub fn suffix_column(&self, level: usize, word: usize) -> ColumnView<'_> {
        self.view
            .image()
            .column(self.schema_columns[self.selection_levels + level][word])
    }

    /// Whether a cursor is an unforced node at a suffix — the scan-fold
    /// pushdown's cheap pre-check (docs/perf/ PRD 05), so a fallback to
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
    /// `keys_out[out_base..]` + pinned-row children (PRD 04's interior).
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
