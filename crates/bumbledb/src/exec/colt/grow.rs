use super::{Colt, Map, ctrl_tag, hash_words, reserve_pool, zero_byte_mask};
use crate::work::WorkError;

// Copy leftward in bounded pieces so cancellation also interrupts relocation.
// A refusal leaves this unpublished tail disposable, not readable as a map.
pub(super) fn compact_tail<T: Copy>(
    pool: &mut Vec<T>,
    start: usize,
    from: usize,
    checkpoint: &mut impl FnMut() -> Result<(), WorkError>,
) -> Result<(), WorkError> {
    assert!(start <= from, "construction compacts leftward");
    let count = pool.len() - from;
    let chunk = (64 * 1024 / std::mem::size_of::<T>().max(1)).max(1);
    for offset in (0..count).step_by(chunk) {
        checkpoint()?;
        let end = offset + chunk.min(count - offset);
        pool.copy_within(from + offset..from + end, start + offset);
    }
    checkpoint()?;
    pool.truncate(start + count);
    Ok(())
}

impl Colt {
    /// Only the local, unpublished map in `force_fill` owns these arena tails.
    /// Reclaim after each growth, before another reservation can count dead
    /// tables. On any refusal `force_unforced` rolls back the whole construction;
    /// older published maps lie strictly before these starts and never move.
    pub(super) fn grow_construction_map(&mut self, m: &mut Map) -> Result<(), WorkError> {
        let old = *m;
        assert_eq!(
            self.ctrl.len(),
            old.ctrl_start + old.nbuckets * 8,
            "construction owns control tail"
        );
        assert_eq!(
            self.buckets.len(),
            old.bucket_start + old.nbuckets * old.stride(),
            "construction owns bucket tail"
        );
        assert_eq!(
            self.dense.len(),
            old.dense_start + old.len as usize,
            "construction owns dense tail"
        );
        self.grow_map(m)?;
        let mut poll = || {
            self.work
                .as_ref()
                .map_or(Ok(()), crate::work::WorkContext::checkpoint)
        };
        compact_tail(&mut self.ctrl, old.ctrl_start, m.ctrl_start, &mut poll)?;
        compact_tail(
            &mut self.buckets,
            old.bucket_start,
            m.bucket_start,
            &mut poll,
        )?;
        compact_tail(&mut self.dense, old.dense_start, m.dense_start, &mut poll)?;
        m.ctrl_start = old.ctrl_start;
        m.bucket_start = old.bucket_start;
        m.dense_start = old.dense_start;
        Ok(())
    }

    pub(super) fn grow_map(&mut self, m: &mut Map) -> Result<(), WorkError> {
        let arity = m.arity;
        let stride = m.stride();
        let new_nbuckets = m.nbuckets * 2;
        let ctrl_needed = self.ctrl.len() + new_nbuckets * 8;
        let bucket_needed = self.buckets.len() + new_nbuckets * stride;
        let dense_needed = self.dense.len() + usize::try_from(m.len).expect("64-bit usize");
        reserve_pool(ctrl_needed, &mut self.ctrl, self.work.as_ref())?;
        reserve_pool(bucket_needed, &mut self.buckets, self.work.as_ref())?;
        reserve_pool(dense_needed, &mut self.dense, self.work.as_ref())?;
        let ctrl_start = self.ctrl.len();
        let bucket_start = self.buckets.len();
        let dense_start = self.dense.len();
        self.ctrl.resize(ctrl_needed, 0);
        self.buckets.resize(bucket_needed, 0);
        let nbm = new_nbuckets - 1;

        let mut key = std::mem::take(&mut self.scratch);
        let rehashed = self.rehash_into(m, arity, stride, ctrl_start, bucket_start, nbm, &mut key);
        self.scratch = key;
        rehashed?;
        m.nbuckets = new_nbuckets;
        m.ctrl_start = ctrl_start;
        m.bucket_start = bucket_start;
        m.dense_start = dense_start;
        Ok(())
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Separate borrowed arenas and execution limits remain explicit on this internal path"
    )]
    fn rehash_into(
        &mut self,
        m: &Map,
        arity: usize,
        stride: usize,
        ctrl_start: usize,
        bucket_start: usize,
        nbm: usize,
        key: &mut Vec<u64>,
    ) -> Result<(), WorkError> {
        reserve_pool(arity, key, self.work.as_ref())?;
        for i in 0..usize::try_from(m.len).expect("64-bit usize") {
            if i % super::force::FORCE_BATCH == 0 {
                self.poll_force_batch((m.len as usize - i).min(super::force::FORCE_BATCH))?;
            }
            let old_idx = usize::try_from(self.dense[m.dense_start + i]).expect("64-bit usize");
            key.clear();
            for word in 0..arity {
                key.push(self.buckets[m.key_word_at(old_idx, word)]);
            }
            let hash = hash_words(key);
            let mut b = usize::try_from(hash).expect("64-bit usize") & nbm;
            let idx = loop {
                let (groups, _) = self.ctrl.as_chunks::<8>();
                let cw = u64::from_le_bytes(groups[ctrl_start / 8 + b]);
                let empties = zero_byte_mask(cw);
                if empties != 0 {
                    break b * 8 + ((empties.trailing_zeros() as usize) >> 3);
                }
                b = (b + 1) & nbm;
            };
            self.ctrl[ctrl_start + idx] = ctrl_tag(hash);
            let new_base = bucket_start + (idx >> 3) * stride;
            for (word, w) in key.iter().enumerate() {
                self.buckets[new_base + word * 8 + (idx & 7)] = *w;
            }
            self.buckets[new_base + 8 * arity + (idx & 7)] = self.buckets[m.child_at(old_idx)];
            self.dense
                .push(u32::try_from(idx).expect("slot index fits u32"));
        }
        Ok(())
    }
}
