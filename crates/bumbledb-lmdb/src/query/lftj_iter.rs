use std::ops::Range;

use super::*;

pub(super) struct LftjAtomPlan<'a> {
    pub(super) variables: Vec<usize>,
    pub(super) source: LftjAtomSource<'a>,
    pub(super) fact_count: usize,
}

pub(super) enum LftjAtomSource<'a> {
    LazyAccess(LazyAccessSlice<'a>),
}

impl<'a> LftjAtomSource<'a> {
    pub(super) fn iter(&'a self) -> LftjTrieIter<'a> {
        match self {
            LftjAtomSource::LazyAccess(slice) => LftjTrieIter::Lazy(slice.iter()),
        }
    }
}

pub(super) enum LftjTrieIter<'a> {
    Lazy(LazyAccessIter<'a>),
}

impl LinearIter for LftjTrieIter<'_> {
    fn key(&self) -> Option<crate::EncodedRef<'_>> {
        match self {
            LftjTrieIter::Lazy(iter) => iter.key(),
        }
    }

    fn next(&mut self) {
        match self {
            LftjTrieIter::Lazy(iter) => iter.next(),
        }
    }

    fn seek(&mut self, target: crate::EncodedRef<'_>) {
        match self {
            LftjTrieIter::Lazy(iter) => iter.seek(target),
        }
    }

    fn at_end(&self) -> bool {
        match self {
            LftjTrieIter::Lazy(iter) => iter.at_end(),
        }
    }
}

impl TrieIter for LftjTrieIter<'_> {
    fn open(&mut self) {
        match self {
            LftjTrieIter::Lazy(iter) => iter.open(),
        }
    }

    fn up(&mut self) {
        match self {
            LftjTrieIter::Lazy(iter) => iter.up(),
        }
    }
}

pub(super) struct LazyAccessSlice<'a> {
    pub(super) index: &'a crate::query_image::RelationIndexImage,
    pub(super) fields: Vec<FieldId>,
    pub(super) filters: Vec<LazyFieldFilter>,
    pub(super) range: Range<usize>,
    pub(super) fact_count: usize,
}

#[derive(Clone)]
pub(super) struct LazyFieldFilter {
    pub(super) field: FieldId,
    pub(super) expected: EncodedOwned,
}

impl<'a> LazyAccessSlice<'a> {
    fn iter(&'a self) -> LazyAccessIter<'a> {
        LazyAccessIter {
            index: self.index,
            fields: &self.fields,
            filters: &self.filters,
            root: self.range.clone(),
            stack: SmallVec::new(),
        }
    }
}

pub(super) struct LazyAccessIter<'a> {
    index: &'a crate::query_image::RelationIndexImage,
    fields: &'a [FieldId],
    filters: &'a [LazyFieldFilter],
    root: Range<usize>,
    stack: SmallVec<[LazyAccessFrame; 4]>,
}

#[derive(Clone, Copy)]
struct LazyAccessFrame {
    depth: usize,
    begin: usize,
    end: usize,
    pos: usize,
}

impl LazyAccessIter<'_> {
    fn current_frame(&self) -> Option<&LazyAccessFrame> {
        self.stack.last()
    }

    fn current_frame_mut(&mut self) -> Option<&mut LazyAccessFrame> {
        self.stack.last_mut()
    }

    fn component_at(&self, position: usize, field: FieldId) -> Option<crate::EncodedRef<'_>> {
        let entry = self.index.entry_at(position)?;
        let bytes = self.index.component_bytes(entry, field)?;
        encoded_ref_for_width(bytes)
    }

    fn entry_matches_filters(&self, position: usize) -> bool {
        let Some(entry) = self.index.entry_at(position) else {
            return false;
        };
        self.filters.iter().all(|filter| {
            self.index
                .component_bytes(entry, filter.field)
                .is_some_and(|bytes| bytes == filter.expected.as_bytes())
        })
    }

    fn group_matches_filters(&self, range: Range<usize>) -> bool {
        self.filters.is_empty() || range.into_iter().any(|pos| self.entry_matches_filters(pos))
    }

    fn advance_to_valid_group(&mut self) {
        loop {
            let Some(frame) = self.current_frame().copied() else {
                return;
            };
            if frame.pos >= frame.end || frame.depth >= self.fields.len() {
                return;
            }
            let range = self.group_bounds(frame);
            if self.group_matches_filters(range.clone()) {
                return;
            }
            if let Some(frame) = self.current_frame_mut() {
                frame.pos = range.end;
            }
        }
    }

    fn group_bounds(&self, frame: LazyAccessFrame) -> Range<usize> {
        if frame.pos >= frame.end {
            return frame.end..frame.end;
        }
        let field = self.fields[frame.depth];
        let Some(key) = self
            .component_at(frame.pos, field)
            .map(EncodedOwned::from_ref)
        else {
            return frame.end..frame.end;
        };
        let mut end = frame.pos + 1;
        while end < frame.end {
            let Some(next) = self.component_at(end, field) else {
                break;
            };
            if compare_encoded_ref_owned(next, &key) != std::cmp::Ordering::Equal {
                break;
            }
            end += 1;
        }
        frame.pos..end
    }

    fn group_start(&self, frame: LazyAccessFrame, position: usize) -> usize {
        if position >= frame.end {
            return frame.end;
        }
        let field = self.fields[frame.depth];
        let Some(key) = self
            .component_at(position, field)
            .map(EncodedOwned::from_ref)
        else {
            return position;
        };
        let mut start = position;
        while start > frame.begin {
            let Some(prev) = self.component_at(start - 1, field) else {
                break;
            };
            if compare_encoded_ref_owned(prev, &key) != std::cmp::Ordering::Equal {
                break;
            }
            start -= 1;
        }
        start
    }
}

impl LinearIter for LazyAccessIter<'_> {
    fn key(&self) -> Option<crate::EncodedRef<'_>> {
        let frame = self.current_frame()?;
        if frame.pos >= frame.end || frame.depth >= self.fields.len() {
            return None;
        }
        self.component_at(frame.pos, self.fields[frame.depth])
    }

    fn next(&mut self) {
        let Some(frame) = self.current_frame().copied() else {
            return;
        };
        let end = self.group_bounds(frame).end;
        if let Some(frame) = self.current_frame_mut() {
            frame.pos = end;
        }
        self.advance_to_valid_group();
    }

    fn seek(&mut self, target: crate::EncodedRef<'_>) {
        let Some(frame) = self.current_frame().copied() else {
            return;
        };
        if frame.depth >= self.fields.len() {
            return;
        }
        let field = self.fields[frame.depth];
        let mut low = frame.pos;
        let mut high = frame.end;
        while low < high {
            let mid = low + (high - low) / 2;
            let Some(value) = self.component_at(mid, field) else {
                high = mid;
                continue;
            };
            if compare_encoded_ref(value, target) == std::cmp::Ordering::Less {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        let pos = self.group_start(frame, low);
        if let Some(frame) = self.current_frame_mut() {
            frame.pos = pos;
        }
        self.advance_to_valid_group();
    }

    fn at_end(&self) -> bool {
        self.current_frame()
            .is_none_or(|frame| frame.pos >= frame.end)
    }
}

impl TrieIter for LazyAccessIter<'_> {
    fn open(&mut self) {
        let depth = self.stack.len();
        if depth >= self.fields.len() {
            self.stack.push(LazyAccessFrame {
                depth,
                begin: 0,
                end: 0,
                pos: 0,
            });
            return;
        }
        let range = if depth == 0 {
            self.root.clone()
        } else if let Some(parent) = self.current_frame().copied() {
            self.group_bounds(parent)
        } else {
            0..0
        };
        self.stack.push(LazyAccessFrame {
            depth,
            begin: range.start,
            end: range.end,
            pos: range.start,
        });
        self.advance_to_valid_group();
    }

    fn up(&mut self) {
        self.stack.pop();
    }
}
