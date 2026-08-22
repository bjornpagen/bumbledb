use super::{Answers, ResolveMemo};

use crate::error::{Error, Result};
use crate::obs;
use crate::storage::catalog::CatalogRead;

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

    /// or copied out of the persistent arena — the LMDB descent and the
    pub(super) fn resolve<C: CatalogRead>(
        &mut self,
        catalog: &C,
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
                let stored = catalog.dict_resolve(crate::encoding::InternId::from_raw(word))?;
                let raw = stored.as_ref();
                obs::event(
                    obs::names::DICT_RESOLVE,
                    obs::TraceArgs::Pair(word, raw.len() as u64),
                );
                let text = std::str::from_utf8(raw).map_err(|_| {
                    Error::Corruption(crate::error::CorruptionError::NonUtf8Intern(word))
                })?;
                let start = self.arena.len();
                self.arena.push_str(text);

                let range = (
                    u32::try_from(start).map_err(|_| Error::ResultBytesOverflow)?,
                    u32::try_from(raw.len()).map_err(|_| Error::ResultBytesOverflow)?,
                );
                let (slot, _) = self.arena_ranges.get_or_insert_with(&key, || range);
                *slot = range;
                (start, raw.len())
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
