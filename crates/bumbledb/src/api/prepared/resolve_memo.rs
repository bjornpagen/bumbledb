use super::{Answers, ResolveMemo};

use crate::error::{CorruptionError, Error, Result};
use crate::image::intern::InternerHandle;

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

    /// Resolve one token into this finalize's answer heap. The text is
    /// borrowed from the pinned generation and copied once per distinct token.
    #[inline]
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
        self.resolve_checked(interner, word, buffer)
    }

    #[inline(never)]
    fn resolve_checked(
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
        let (slot, inserted) = self.ranges.get_or_insert_with(&key, || (0, 0));
        if !inserted {
            let range = (slot.0 as usize, slot.1 as usize);
            self.last = Some((word, range));
            return Ok(range);
        }

        let start = buffer.text.len();
        // Resolution touches only the resolver and answer heap, never this
        // map. Retain the inserted slot instead of probing (and possibly
        // growing before a duplicate lookup) a second time.
        let resolved = (|| {
            let Some(len) = super::text::resolve_text(interner, word, |text| {
                buffer.text.push_str(text);
            }) else {
                return Err(Error::Corruption(CorruptionError::DanglingInternId(
                    crate::encoding::InternId::from_raw(word),
                )));
            };

            let range = (
                u32::try_from(start).map_err(|_| Error::ResultBytesOverflow)?,
                u32::try_from(len).map_err(|_| Error::ResultBytesOverflow)?,
            );
            Ok((range, len))
        })();
        let (range, len) = match resolved {
            Ok(resolved) => resolved,
            Err(error) => {
                // Failed resolution must not leave (0, 0) as a valid hit.
                // Keep the existing answer-buffer/error semantics intact.
                self.clear();
                return Err(error);
            }
        };
        *slot = range;
        self.last = Some((word, (start, len)));
        Ok((start, len))
    }
}
