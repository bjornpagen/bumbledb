use super::{Answers, ResolveMemo};

use crate::error::{Error, Result};
use crate::image::intern::InternerHandle;
use crate::obs;

impl ResolveMemo {
    pub(super) fn new() -> Self {
        Self {
            ranges: crate::exec::wordmap::WordMap::new(1),
            last: None,
            arena_ranges: crate::exec::wordmap::WordMap::new(1),
            arena: String::new(),
        }
    }

    pub(super) fn clear(&mut self) {
        self.ranges.clear();
        self.last = None;
    }

    /// or copied out of the persistent arena — the interner read and the
    pub(super) fn resolve(
        &mut self,
        interner: &InternerHandle<'_>,
        word: u64,
        buffer: &mut Answers,
    ) -> Result<(usize, usize)> {
        if let Some((last_word, range)) = self.last
            && last_word == word
        {
            return Ok(range);
        }
        let key = [word];
        if let (range, false) = self.ranges.get_or_insert_with(&key, || (0, 0)) {
            let range = (range.0 as usize, range.1 as usize);
            self.last = Some((word, range));
            return Ok(range);
        }

        let (arena_start, len) =
            if let (range, false) = self.arena_ranges.get_or_insert_with(&key, || (0, 0)) {
                (range.0 as usize, range.1 as usize)
            } else {
                // Projected text words are interner tokens minted by the
                // images the answers came from; a token this interner never
                // minted is dangling — corruption-grade, never a silent
                // empty string.
                let start = self.arena.len();
                let len = interner
                    .with_text(word, |text| {
                        self.arena.push_str(text);
                        text.len()
                    })
                    .ok_or(Error::Corruption(
                        crate::error::CorruptionError::DanglingInternId(
                            crate::encoding::InternId::from_raw(word),
                        ),
                    ))?;
                obs::event(
                    obs::names::DICT_RESOLVE,
                    obs::TraceArgs::Pair(word, len as u64),
                );

                let range = (
                    u32::try_from(start).map_err(|_| Error::ResultBytesOverflow)?,
                    u32::try_from(len).map_err(|_| Error::ResultBytesOverflow)?,
                );
                let (slot, _) = self.arena_ranges.get_or_insert_with(&key, || range);
                *slot = range;
                (start, len)
            };
        let start = buffer.text.len();
        buffer
            .text
            .push_str(&self.arena[arena_start..arena_start + len]);

        let range = (
            u32::try_from(start).map_err(|_| Error::ResultBytesOverflow)?,
            u32::try_from(len).map_err(|_| Error::ResultBytesOverflow)?,
        );
        let (slot, _) = self.ranges.get_or_insert_with(&key, || range);
        *slot = range;
        self.last = Some((word, (start, len)));
        Ok((start, len))
    }
}
