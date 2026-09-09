use super::{WINDOW, WordMap, ctrl_tag, hash_core, hash_words};

// Check representability before allocating or replacing any backing array.
// Preserve the existing slot-cardinality and key-width representation bounds.
// The next growth must refuse before replacing any index backing.
pub(super) fn growth_layout(capacity: usize, arity: usize) -> (usize, usize) {
    let capacity = capacity
        .checked_mul(2)
        .expect("WordMap capacity overflow")
        .max(WINDOW);
    u32::try_from(capacity - 1).expect("WordMap capacity exceeds dense slot indices");
    let words = capacity
        .checked_mul(arity)
        .expect("WordMap key capacity overflow");
    (capacity, words)
}

impl<V: Copy> WordMap<V> {
    pub(super) fn grow(&mut self) {
        let (new_capacity, _) = growth_layout(self.capacity(), self.arity);

        // Keep all old arrays intact until replacement allocations succeed.
        // Payload stays packed and does not move or grow during rehash.
        let slots = vec![0; new_capacity];
        let ctrl = vec![0; new_capacity + WINDOW - 1];
        let stamps = vec![0; new_capacity];
        self.slots = slots;
        self.ctrl = ctrl;
        self.stamps = stamps;
        self.stale = 0;

        match self.arity {
            0 => self.rehash_core::<0>(),
            1 => self.rehash_core::<1>(),
            2 => self.rehash_core::<2>(),
            3 => self.rehash_core::<3>(),
            4 => self.rehash_core::<4>(),
            5 => self.rehash_core::<5>(),
            6 => self.rehash_core::<6>(),
            7 => self.rehash_core::<7>(),
            8 => self.rehash_core::<8>(),
            _ => self.rehash_dyn(),
        }
    }

    fn rehash_core<const K: usize>(&mut self) {
        for row in 0..self.len() {
            let key = &self.keys[row * K..row * K + K];
            let hash = hash_core::<K>(key);
            let (found, slot) = self.probe_core::<K>(key, hash);
            debug_assert!(!found, "rehashed keys are distinct");
            self.slots[slot] = u32::try_from(row).expect("row ordinal fits u32");
            self.set_ctrl(slot, ctrl_tag(hash));
        }
    }

    fn rehash_dyn(&mut self) {
        for row in 0..self.len() {
            let key = &self.keys[row * self.arity..(row + 1) * self.arity];
            let hash = hash_words(key);
            let (found, slot) = self.probe(key, hash);
            debug_assert!(!found, "rehashed keys are distinct");
            self.slots[slot] = u32::try_from(row).expect("row ordinal fits u32");
            self.set_ctrl(slot, ctrl_tag(hash));
        }
    }
}
