use super::{Colt, Map, ctrl_tag, hash_words, zero_byte_mask};

impl Colt {

    pub(super) fn grow_map(&mut self, m: &mut Map) {
        let arity = m.arity;
        let stride = m.stride();
        let new_nbuckets = m.nbuckets * 2;
        let ctrl_start = self.ctrl.len();
        let bucket_start = self.buckets.len();
        let dense_start = self.dense.len();
        self.ctrl.resize(ctrl_start + new_nbuckets * 8, 0);
        self.buckets.resize(bucket_start + new_nbuckets * stride, 0);
        let nbm = new_nbuckets - 1;

        // the scratch only after its growth check).
        let mut key = std::mem::take(&mut self.scratch);
        for i in 0..usize::try_from(m.len).expect("64-bit usize") {
            let old_idx = usize::try_from(self.dense[m.dense_start + i]).expect("64-bit usize");
            key.clear();
            for word in 0..arity {
                key.push(self.buckets[m.key_word_at(old_idx, word)]);
            }
            let hash = hash_words(&key);
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
        self.scratch = key;
        m.nbuckets = new_nbuckets;
        m.ctrl_start = ctrl_start;
        m.bucket_start = bucket_start;
        m.dense_start = dense_start;
    }
}
