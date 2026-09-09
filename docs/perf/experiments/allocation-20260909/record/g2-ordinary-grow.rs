use super::{Colt, Map, ctrl_tag, hash_words, reserve_pool, zero_byte_mask};
use crate::work::{WorkContext, WorkError};

const GROW_BYTES: usize = 64 * 1024;

fn checkpoint(work: Option<&WorkContext>) -> Result<(), WorkError> {
    #[cfg(test)]
    super::tests::staged_growth::checkpoint()?;
    work.map_or(Ok(()), WorkContext::checkpoint)
}

fn pool_end<T>(start: usize, count: usize) -> Result<usize, WorkError> {
    start
        .checked_add(count)
        .filter(|&end| end <= isize::MAX as usize / std::mem::size_of::<T>())
        .ok_or(WorkError::Allocation)
}

// Validate every size before reserving or touching construction storage.
fn growth_layout(m: &Map) -> Result<(Map, usize, usize, usize), WorkError> {
    let words = m.arity.checked_add(1).ok_or(WorkError::Allocation)?;
    let nbuckets = m.nbuckets.checked_mul(2).ok_or(WorkError::Allocation)?;
    let slots = nbuckets.checked_mul(8).ok_or(WorkError::Allocation)?;
    u32::try_from(slots.checked_sub(1).ok_or(WorkError::Allocation)?)
        .map_err(|_| WorkError::Allocation)?;
    let bucket_words = slots.checked_mul(words).ok_or(WorkError::Allocation)?;
    let staged_words = (m.len as usize)
        .checked_mul(words)
        .ok_or(WorkError::Allocation)?;
    let ctrl_end = pool_end::<u8>(m.ctrl_start, slots)?;
    let bucket_end = pool_end::<u64>(m.bucket_start, bucket_words)?;
    pool_end::<u32>(m.dense_start, m.len as usize)?;
    pool_end::<u64>(0, staged_words)?;
    Ok((Map { nbuckets, ..*m }, ctrl_end, bucket_end, staged_words))
}

fn extend_tail<T: Copy>(
    pool: &mut Vec<T>,
    end: usize,
    value: T,
    work: Option<&WorkContext>,
) -> Result<(), WorkError> {
    let batch = GROW_BYTES / std::mem::size_of::<T>();
    while pool.len() < end {
        checkpoint(work)?;
        pool.resize(pool.len() + (end - pool.len()).min(batch), value);
    }
    Ok(())
}

impl Colt {
    /// Grow an unpublished map that owns all three arena tails. Any error
    /// requires `force_unforced` to discard the entire unfinished construction;
    /// already-published maps precede these tails and are never rewritten.
    pub(super) fn grow_map(&mut self, m: &mut Map) -> Result<(), WorkError> {
        let mut entries = std::mem::take(&mut self.scratch);
        entries.clear();
        let result = self.grow_staged(m, &mut entries);
        entries.clear();
        self.scratch = entries;
        result
    }

    fn grow_staged(&mut self, m: &mut Map, entries: &mut Vec<u64>) -> Result<(), WorkError> {
        let (grown, ctrl_end, bucket_end, staged_words) = growth_layout(m)?;
        assert_eq!(self.ctrl.len(), m.ctrl_start + m.nbuckets * 8);
        assert_eq!(self.buckets.len(), m.bucket_start + m.nbuckets * m.stride());
        assert_eq!(self.dense.len(), m.dense_start + m.len as usize);
        let work = self.work.as_ref();
        checkpoint(work)?;
        reserve_pool(staged_words, entries, work)?;
        for (i, &old_idx) in self.dense[m.dense_start..].iter().enumerate() {
            if i % super::force::FORCE_BATCH == 0 {
                checkpoint(work)?;
            }
            let old_idx = old_idx as usize;
            entries.extend((0..m.arity).map(|word| self.buckets[m.key_word_at(old_idx, word)]));
            entries.push(self.buckets[m.child_at(old_idx)]);
        }
        checkpoint(work)?;
        reserve_pool(ctrl_end, &mut self.ctrl, work)?;
        reserve_pool(bucket_end, &mut self.buckets, work)?;
        checkpoint(work)?;

        // Both reservations precede destructive writes. New bucket words must
        // be initialized, but empty controls gate the old payload: don't clear it.
        extend_tail(&mut self.buckets, bucket_end, 0, work)?;
        for chunk in self.ctrl[m.ctrl_start..].chunks_mut(GROW_BYTES) {
            checkpoint(work)?;
            chunk.fill(0);
        }
        extend_tail(&mut self.ctrl, ctrl_end, 0, work)?;

        let nbm = grown.nbuckets - 1;
        for (i, entry) in entries.chunks_exact(m.arity + 1).enumerate() {
            if i % super::force::FORCE_BATCH == 0 {
                checkpoint(work)?;
            }
            let (key, child) = entry.split_at(m.arity);
            let hash = hash_words(key);
            let mut b = usize::try_from(hash).expect("64-bit usize") & nbm;
            let idx = loop {
                let (groups, _) = self.ctrl.as_chunks::<8>();
                let cw = u64::from_le_bytes(groups[grown.ctrl_start / 8 + b]);
                let empties = zero_byte_mask(cw);
                if empties != 0 {
                    break b * 8 + ((empties.trailing_zeros() as usize) >> 3);
                }
                b = (b + 1) & nbm;
            };
            self.ctrl[grown.ctrl_start + idx] = ctrl_tag(hash);
            for (word, &value) in key.iter().enumerate() {
                self.buckets[grown.key_word_at(idx, word)] = value;
            }
            self.buckets[grown.child_at(idx)] = child[0];
            self.dense[grown.dense_start + i] = u32::try_from(idx).expect("checked slot capacity");
        }
        checkpoint(work)?;
        *m = grown;
        Ok(())
    }
}
