use std::mem::MaybeUninit;

use super::{WINDOW, WordMap, ctrl_tag, hash_core, hash_words};

impl<V: Copy> WordMap<V> {
    pub(super) fn grow(&mut self) {
        let new_capacity = (self.capacity() * 2).max(WINDOW);

        crate::obs::event(
            crate::obs::names::WORDMAP_GROW,
            crate::obs::TraceArgs::Pair(new_capacity as u64, self.arity as u64),
        );
        let old_keys = std::mem::replace(&mut self.keys, vec![0; new_capacity * self.arity]);
        let old_values = std::mem::replace(
            &mut self.values,
            std::iter::repeat_with(MaybeUninit::uninit)
                .take(new_capacity)
                .collect(),
        );
        self.ctrl = vec![0; new_capacity + WINDOW - 1];

        self.stamps = vec![0; new_capacity];
        self.stale = 0;

        match self.arity {
            0 => self.rehash_core::<0>(&old_keys, &old_values),
            1 => self.rehash_core::<1>(&old_keys, &old_values),
            2 => self.rehash_core::<2>(&old_keys, &old_values),
            3 => self.rehash_core::<3>(&old_keys, &old_values),
            4 => self.rehash_core::<4>(&old_keys, &old_values),
            5 => self.rehash_core::<5>(&old_keys, &old_values),
            6 => self.rehash_core::<6>(&old_keys, &old_values),
            7 => self.rehash_core::<7>(&old_keys, &old_values),
            8 => self.rehash_core::<8>(&old_keys, &old_values),
            _ => self.rehash_dyn(&old_keys, &old_values),
        }
    }

    fn rehash_core<const K: usize>(&mut self, old_keys: &[u64], old_values: &[MaybeUninit<V>]) {
        for i in 0..self.dense.len() {
            let old_idx = self.dense[i] as usize;
            let key = &old_keys[old_idx * K..old_idx * K + K];
            let hash = hash_core::<K>(key);
            let (found, new_idx) = self.probe_core::<K>(key, hash);
            debug_assert!(!found, "rehashed keys are distinct");
            self.set_ctrl(new_idx, ctrl_tag(hash));
            self.keys[new_idx * K..new_idx * K + K].copy_from_slice(key);
            // SAFETY: old_idx was occupied (dense-listed), so its value

            self.values[new_idx].write(unsafe { old_values[old_idx].assume_init_read() });
            self.dense[i] = u32::try_from(new_idx).expect("slot index fits u32");
        }
    }

    fn rehash_dyn(&mut self, old_keys: &[u64], old_values: &[MaybeUninit<V>]) {
        for i in 0..self.dense.len() {
            let old_idx = self.dense[i] as usize;
            let key_range = old_idx * self.arity..(old_idx + 1) * self.arity;
            let hash = hash_words(&old_keys[key_range.clone()]);
            let (found, new_idx) = self.probe(&old_keys[key_range.clone()], hash);
            debug_assert!(!found, "rehashed keys are distinct");
            self.set_ctrl(new_idx, ctrl_tag(hash));
            self.keys[new_idx * self.arity..(new_idx + 1) * self.arity]
                .copy_from_slice(&old_keys[key_range]);
            // SAFETY: old_idx was occupied (dense-listed), so its value

            self.values[new_idx].write(unsafe { old_values[old_idx].assume_init_read() });
            self.dense[i] = u32::try_from(new_idx).expect("slot index fits u32");
        }
    }
}
