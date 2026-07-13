use super::{ResolveMemo, ResultBuffer};

use crate::error::{Error, Result};
use crate::obs;
use crate::storage::dict;
use crate::storage::env::ReadTxn;

impl ResolveMemo {
    pub(super) fn new() -> Self {
        Self {
            ranges: crate::exec::wordmap::WordMap::new(1),
            last: None,
        }
    }

    pub(super) fn clear(&mut self) {
        self.ranges.clear();
        self.last = None;
    }

    /// The byte range for one string intern word: memoized, or resolved
    /// through the dictionary (emitting `dict_resolve`), UTF-8-checked,
    /// and appended to the buffer once. Strings are the only interned
    /// type, so the key is the bare word — the tag byte died with
    /// variable bytes (docs/architecture/50-storage.md).
    pub(super) fn resolve(
        &mut self,
        txn: &ReadTxn<'_>,
        word: u64,
        buffer: &mut ResultBuffer,
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
        let raw = dict::resolve(txn, word)?;
        obs::event(
            obs::names::DICT_RESOLVE,
            obs::Category::Storage,
            word,
            raw.len() as u64,
        );
        std::str::from_utf8(raw)
            .map_err(|_| Error::Corruption(crate::error::CorruptionError::NonUtf8Intern(word)))?;
        let start = buffer.bytes.len();
        buffer.bytes.extend_from_slice(raw);
        // The byte heap's offsets are u32: a >4 GiB distinct-payload
        // result is absurd under the scale axiom but valid input — a
        // typed error, not a panic (finalize already threads Result).
        let range = (
            u32::try_from(start).map_err(|_| Error::ResultBytesOverflow)?,
            u32::try_from(raw.len()).map_err(|_| Error::ResultBytesOverflow)?,
        );
        let (slot, _) = self.ranges.get_or_insert_with(&key, || range);
        *slot = range;
        self.last = Some((word, (start, raw.len())));
        Ok((start, raw.len()))
    }
}
