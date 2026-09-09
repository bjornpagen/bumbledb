#[cfg(test)]
use std::mem::MaybeUninit;

use super::WordMap;
#[cfg(test)]
use super::{HINT_CAP, LOAD_DEN};

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

    /// Explicit geometry for collision and generation tests. Production maps
    /// start empty and grow from actual insertions, not planner estimates.
    /// # Panics
    /// If the hinted backing's key-word count cannot be represented.
    #[must_use]
    #[cfg(test)]
    pub fn with_capacity_hint(arity: usize, hint: usize) -> Self {
        let mut map = Self::new(arity);
        let capacity = (hint.clamp(2, HINT_CAP) * LOAD_DEN).next_power_of_two();
        map.allocate(capacity);
        map
    }

    #[cfg(test)]
    fn allocate(&mut self, capacity: usize) {
        debug_assert!(capacity.is_power_of_two() && capacity >= super::WINDOW);
        let words = capacity
            .checked_mul(self.arity)
            .expect("WordMap key capacity overflow");
        self.ctrl = vec![0; capacity + super::WINDOW - 1];
        self.keys = vec![0; words];
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
        if idx < super::WINDOW - 1 {
            let capacity = self.capacity();
            self.ctrl[capacity + idx] = value;
        }
    }
}
