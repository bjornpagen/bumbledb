use super::{ResolveMemo, ResultBuffer};

use crate::error::{Error, Result};
use crate::obs;
use crate::storage::dict;
use crate::storage::env::ReadTxn;

impl ResolveMemo {
    pub(super) fn new() -> Self {
        Self {
            ranges: crate::exec::wordmap::WordMap::new(2),
            last: None,
        }
    }

    pub(super) fn clear(&mut self) {
        self.ranges.clear();
        self.last = None;
    }

    /// The byte range for one intern word: memoized, or resolved through
    /// the dictionary (emitting `dict_resolve`), UTF-8-checked for
    /// strings, and appended to the buffer once.
    pub(super) fn resolve(
        &mut self,
        txn: &ReadTxn<'_>,
        word: u64,
        tag: u8,
        buffer: &mut ResultBuffer,
        utf8: bool,
    ) -> Result<(usize, usize)> {
        if let Some((last_key, range)) = self.last {
            if last_key == (word, tag) {
                return Ok(range);
            }
        }
        let key = [word, u64::from(tag)];
        if let (range, false) = self.ranges.get_or_insert_with(&key, || (0, 0)) {
            let range = (range.0 as usize, range.1 as usize);
            self.last = Some(((word, tag), range));
            return Ok(range);
        }
        let raw = dict::resolve(txn, word, tag)?;
        obs::event(
            obs::names::DICT_RESOLVE,
            obs::Category::Storage,
            word,
            raw.len() as u64,
        );
        if utf8 {
            std::str::from_utf8(raw).map_err(|_| {
                Error::Corruption(crate::error::CorruptionError::NonUtf8Intern(word))
            })?;
        }
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
        self.last = Some(((word, tag), (start, raw.len())));
        Ok((start, raw.len()))
    }
}
