use std::mem::MaybeUninit;

use super::{HINT_CAP, LOAD_DEN, WINDOW, WordMap};

impl<V: Copy> WordMap<V> {
    #[must_use]
    pub fn new(arity: usize) -> Self {
        Self {
            arity,
            ctrl: Vec::new(),
            keys: Vec::new(),
            values: Vec::new(),
            stamps: Vec::new(),
            generation: 0,
            stale: 0,
            dense: Vec::new(),
            len: 0,
        }
    }

    #[must_use]
    pub fn with_capacity_hint(arity: usize, hint: usize) -> Self {
        let mut map = Self::new(arity);
        let capacity = (hint.clamp(2, HINT_CAP) * LOAD_DEN).next_power_of_two();
        map.allocate(capacity);
        map
    }

    fn allocate(&mut self, capacity: usize) {
        debug_assert!(capacity.is_power_of_two() && capacity >= WINDOW);
        self.ctrl = vec![0; capacity + WINDOW - 1];
        self.keys = vec![0; capacity * self.arity];
        self.values = std::iter::repeat_with(MaybeUninit::uninit)
            .take(capacity)
            .collect();
        self.stamps = vec![0; capacity];
    }

    #[inline(always)]
    pub(super) fn capacity(&self) -> usize {
        self.values.len()
    }

    #[inline(always)]
    pub(super) fn set_ctrl(&mut self, idx: usize, value: u8) {
        self.ctrl[idx] = value;
        self.stamps[idx] = self.generation;
        if idx < WINDOW - 1 {
            let capacity = self.capacity();
            self.ctrl[capacity + idx] = value;
        }
    }
}
