use super::{Answers, ResolveMemo};

use crate::error::{CorruptionError, Error, Result};
use crate::image::intern::InternerHandle;
use crate::image::NonresidentTextStore;
use crate::obs;

impl ResolveMemo {
    pub(super) fn new() -> Self {
        Self {
            ranges: crate::exec::wordmap::WordMap::new(1),
            last: None,
            scratch_epoch: None,
        }
    }

    pub(super) fn clear(&mut self) {
        self.ranges.clear();
        self.last = None;
    }

    /// Drop per-finalize mappings that named scratch tokens. Called
    /// before the store that minted them disappears.
    pub(super) fn forget_scratch(&mut self) {
        self.ranges.clear();
        self.last = None;
        self.scratch_epoch = None;
    }

    #[cfg(test)]
    pub(super) fn uncharged_copy_bytes(&self) -> usize {
        0
    }

    /// Resolve one token into this finalize's answer heap. Intern text
    /// is read from the generation owner; scratch text from the live
    /// store. No persistent uncharged dictionary is retained.
    pub(super) fn resolve(
        &mut self,
        interner: &InternerHandle<'_>,
        store: Option<&mut NonresidentTextStore>,
        word: u64,
        buffer: &mut Answers,
    ) -> Result<(usize, usize)> {
        if crate::image::is_scratch_token(word) {
            let Some(live) = store.as_deref() else {
                return Err(Error::Corruption(CorruptionError::DanglingInternId(
                    crate::encoding::InternId::from_raw(word),
                )));
            };
            match self.scratch_epoch {
                Some(stamp) => {
                    let eq = interner
                        .generation()
                        .text_eq(Some(live))
                        .with_memo_stamp(stamp);
                    if !eq.accepts_stamp(stamp) {
                        self.forget_scratch();
                        self.scratch_epoch = Some(live.epoch());
                    }
                }
                None => self.scratch_epoch = Some(live.epoch()),
            }
            if !live.live(word) {
                return Err(Error::Corruption(CorruptionError::DanglingInternId(
                    crate::encoding::InternId::from_raw(word),
                )));
            }
        }
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

        let start = buffer.text.len();
        let Some(len) = super::text::resolve_tagged(interner, store, word, |text| {
            buffer.text.push_str(text);
        })? else {
            return Err(Error::Corruption(CorruptionError::DanglingInternId(
                crate::encoding::InternId::from_raw(word),
            )));
        };
        obs::event(
            obs::names::DICT_RESOLVE,
            obs::TraceArgs::Pair(word, len as u64),
        );

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
