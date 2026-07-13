//! Slot-indexed bindings with an epoch discipline.

use super::Bindings;

impl Bindings {
    #[must_use]
    pub fn new(slot_count: usize) -> Self {
        Self {
            slots: vec![0; slot_count],
            #[cfg(debug_assertions)]
            epochs: vec![0; slot_count],
            #[cfg(debug_assertions)]
            current: 0,
        }
    }

    /// Re-sizes the slot array to one rule's binding layout (the rule
    /// loop shares this scratch across rules — capacity is the
    /// high-water across all of them) and staleness-bumps like
    /// [`Bindings::reset`].
    pub fn resize(&mut self, slot_count: usize) {
        self.slots.clear();
        self.slots.resize(slot_count, 0);
        #[cfg(debug_assertions)]
        {
            self.epochs.clear();
            self.epochs.resize(slot_count, 0);
            self.current += 1;
        }
    }

    /// Starts a fresh execution: every slot becomes stale at once.
    pub fn reset(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.current += 1;
        }
    }

    pub fn set(&mut self, slot: usize, value: u64) {
        self.slots[slot] = value;
        #[cfg(debug_assertions)]
        {
            self.epochs[slot] = self.current;
        }
    }

    /// Loads a complete binding row (the pipelined executor's parent
    /// rows): every slot becomes bound.
    pub fn load_row(&mut self, row: &[u64]) {
        self.slots.copy_from_slice(row);
        #[cfg(debug_assertions)]
        {
            self.current += 1;
            for epoch in &mut self.epochs {
                *epoch = self.current;
            }
        }
    }

    /// Reads a bound slot.
    #[must_use]
    pub fn get(&self, slot: usize) -> u64 {
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            self.epochs[slot], self.current,
            "reads are plan-scoped: slot {slot} must be bound"
        );
        self.slots[slot]
    }

    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }
}
