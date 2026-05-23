use super::*;

pub(super) fn encoded_owned_for_width(width: usize, bytes: &[u8]) -> Result<EncodedOwned> {
    match width {
        1 => {
            Ok(EncodedOwned::One(bytes.try_into().map_err(|_| {
                Error::internal("encoded value width mismatch")
            })?))
        }
        8 => {
            Ok(EncodedOwned::Eight(bytes.try_into().map_err(|_| {
                Error::internal("encoded value width mismatch")
            })?))
        }
        16 => {
            Ok(EncodedOwned::Sixteen(bytes.try_into().map_err(|_| {
                Error::internal("encoded value width mismatch")
            })?))
        }
        width => Err(Error::internal(format!(
            "unsupported encoded value width {width}"
        ))),
    }
}

pub(super) fn encoded_ref_for_width(bytes: &[u8]) -> Option<crate::EncodedRef<'_>> {
    match bytes.len() {
        1 => Some(crate::EncodedRef::One(bytes.try_into().ok()?)),
        8 => Some(crate::EncodedRef::Eight(bytes.try_into().ok()?)),
        16 => Some(crate::EncodedRef::Sixteen(bytes.try_into().ok()?)),
        _ => None,
    }
}

pub(super) struct LeapfrogState {
    iter_ids: SmallParticipants,
    p: usize,
    at_end: bool,
}

impl LeapfrogState {
    pub(super) fn new(iter_ids: SmallParticipants) -> Self {
        Self {
            iter_ids,
            p: 0,
            at_end: false,
        }
    }

    pub(super) fn at_end(&self) -> bool {
        self.at_end
    }

    pub(super) fn init(
        &mut self,
        iters: &mut [LftjTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<()> {
        if self.iter_ids.iter().any(|id| iters[*id].at_end()) {
            self.at_end = true;
            return Ok(());
        }
        self.sort_iter_ids(iters, counters)?;
        self.p = 0;
        self.search(iters, counters)
    }

    fn sort_iter_ids(
        &mut self,
        iters: &[LftjTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<()> {
        let mut error = None;
        self.iter_ids.sort_by(|left, right| {
            if error.is_some() {
                return std::cmp::Ordering::Equal;
            }
            let Some(left) = key_ref_opt(&iters[*left], counters) else {
                error = Some(missing_trie_key_error());
                return std::cmp::Ordering::Equal;
            };
            let Some(right) = key_ref_opt(&iters[*right], counters) else {
                error = Some(missing_trie_key_error());
                return std::cmp::Ordering::Equal;
            };
            compare_encoded_ref(left, right)
        });
        if let Some(error) = error {
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn key(
        &self,
        iters: &[LftjTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<EncodedOwned> {
        self.iter_ids
            .first()
            .map(|id| key_owned(&iters[*id], counters))
            .transpose()?
            .ok_or_else(|| Error::internal("leapfrog join has no iterators"))
    }

    pub(super) fn next(
        &mut self,
        iters: &mut [LftjTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<()> {
        if self.at_end {
            return Ok(());
        }
        let id = self.iter_ids[self.p];
        iters[id].next();
        counters.trie_next += 1;
        counters.lftj_next_calls += 1;
        if iters[id].at_end() {
            self.at_end = true;
            return Ok(());
        }
        self.p = (self.p + 1) % self.iter_ids.len();
        self.search(iters, counters)
    }

    fn search(
        &mut self,
        iters: &mut [LftjTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<()> {
        if self.iter_ids.is_empty() || self.at_end {
            return Ok(());
        }
        if self.iter_ids.len() == 1 {
            return Ok(());
        }
        let Some(mut max) = key_owned_opt(
            &iters[self.iter_ids[(self.p + self.iter_ids.len() - 1) % self.iter_ids.len()]],
            counters,
        ) else {
            return Err(missing_trie_key_error());
        };
        loop {
            let id = self.iter_ids[self.p];
            let Some(current) = key_ref_opt(&iters[id], counters) else {
                return Err(missing_trie_key_error());
            };
            if compare_encoded_ref_owned(current, &max) == std::cmp::Ordering::Equal {
                return Ok(());
            }
            iters[id].seek(max.as_ref());
            counters.trie_seek += 1;
            counters.lftj_seek_calls += 1;
            if iters[id].at_end() {
                self.at_end = true;
                return Ok(());
            }
            let Some(next_max) = key_owned_opt(&iters[id], counters) else {
                return Err(missing_trie_key_error());
            };
            max = next_max;
            self.p = (self.p + 1) % self.iter_ids.len();
        }
    }
}

fn key_owned(iter: &LftjTrieIter<'_>, counters: &mut PlanCounters) -> Result<EncodedOwned> {
    key_owned_opt(iter, counters).ok_or_else(missing_trie_key_error)
}

fn key_owned_opt(iter: &LftjTrieIter<'_>, counters: &mut PlanCounters) -> Option<EncodedOwned> {
    key_ref_opt(iter, counters).map(EncodedOwned::from_ref)
}

fn key_ref_opt<'a>(
    iter: &'a LftjTrieIter<'a>,
    counters: &mut PlanCounters,
) -> Option<crate::EncodedRef<'a>> {
    let key = iter.key()?;
    counters.trie_key_reads += 1;
    counters.lftj_key_reads += 1;
    Some(key)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EncodedWidth {
    W1,
    W8,
    W16,
}

fn encoded_width_for_len(len: usize) -> Option<EncodedWidth> {
    match len {
        1 => Some(EncodedWidth::W1),
        8 => Some(EncodedWidth::W8),
        16 => Some(EncodedWidth::W16),
        _ => None,
    }
}

pub(super) fn compare_encoded_ref(
    left: crate::EncodedRef<'_>,
    right: crate::EncodedRef<'_>,
) -> std::cmp::Ordering {
    compare_encoded_bytes(left.as_bytes(), right.as_bytes())
}

pub(super) fn compare_encoded_ref_owned(
    left: crate::EncodedRef<'_>,
    right: &EncodedOwned,
) -> std::cmp::Ordering {
    compare_encoded_bytes(left.as_bytes(), right.as_bytes())
}

pub(in crate::query) fn compare_encoded_bytes(left: &[u8], right: &[u8]) -> std::cmp::Ordering {
    match (encoded_width_for_len(left.len()), left.len() == right.len()) {
        (Some(EncodedWidth::W1), true) => left[0].cmp(&right[0]),
        (Some(EncodedWidth::W8), true) => {
            let mut left_bytes = [0u8; 8];
            let mut right_bytes = [0u8; 8];
            left_bytes.copy_from_slice(left);
            right_bytes.copy_from_slice(right);
            let left = u64::from_be_bytes(left_bytes);
            let right = u64::from_be_bytes(right_bytes);
            left.cmp(&right)
        }
        (Some(EncodedWidth::W16), true) | (None, _) | (_, false) => left.cmp(right),
    }
}

fn missing_trie_key_error() -> Error {
    Error::internal("trie key requested for exhausted iterator")
}
