use super::{Answers, ResolveMemo};

use crate::error::{Error, Result};
use crate::obs;
use crate::storage::dict;
use crate::storage::env::ReadTxn;

impl ResolveMemo {
    pub(super) fn new() -> Self {
        Self {
            ranges: crate::exec::wordmap::WordMap::new(1),
            last: None,
            arena_ranges: crate::exec::wordmap::WordMap::new(1),
            arena: String::new(),
        }
    }

    /// Clears the per-finalize tier only: the ranges index the finalize's
    /// own answer carrier. The arena tier persists — the append-only
    /// dictionary makes every entry in it final.
    pub(super) fn clear(&mut self) {
        self.ranges.clear();
        self.last = None;
    }

    /// The text range for one string intern word: memoized per finalize,
    /// or copied out of the persistent arena — the LMDB descent and the
    /// UTF-8 parse run only on an arena miss, i.e. ONCE per distinct
    /// intern over the prepared query's lifetime (`dict_resolve` fires
    /// exactly there). The parse's proof travels with the arena's type,
    /// so reads never re-validate (parse, don't validate). Strings are
    /// the only interned type, so the key is the bare word — the tag
    /// byte died with variable bytes (docs/architecture/50-storage.md).
    pub(super) fn resolve(
        &mut self,
        txn: &ReadTxn<'_>,
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
        // The persistent tier: an arena hit skips the dictionary whole.
        let (arena_start, len) =
            if let (range, false) = self.arena_ranges.get_or_insert_with(&key, || (0, 0)) {
                (range.0 as usize, range.1 as usize)
            } else {
                let raw = dict::resolve(txn, word)?;
                obs::event(
                    obs::names::DICT_RESOLVE,
                    obs::Category::Storage,
                    word,
                    raw.len() as u64,
                );
                let text = std::str::from_utf8(raw).map_err(|_| {
                    Error::Corruption(crate::error::CorruptionError::NonUtf8Intern(word))
                })?;
                let start = self.arena.len();
                self.arena.push_str(text);
                // The arena's offsets are u32: a >4 GiB distinct-payload
                // high-water (the u32 ceiling — a representation limit,
                // not the map size) is beyond any validated workload but
                // valid input — a typed error, not a panic.
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
        // The byte heap's offsets are u32, as the arena's (above).
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
