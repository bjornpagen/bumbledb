use std::mem::MaybeUninit;

use super::{WordMap, HINT_CAP, LOAD_DEN, WINDOW};

impl<V: Copy> WordMap<V> {
    /// An empty map for keys of `arity` words (zero arity is legal: every
    /// key is the empty tuple — the global-aggregate group).
    #[must_use]
    pub fn new(arity: usize) -> Self {
        Self {
            arity,
            ctrl: Vec::new(),
            keys: Vec::new(),
            values: Vec::new(),
            dense: Vec::new(),
            len: 0,
        }
    }

    /// An empty map presized for ~`hint` entries (docs/perf/ PRD 06): one
    /// allocation up front instead of a rehash ladder inside the first
    /// measured execution. The map still grows if the hint was short.
    /// Sizing covers the hint at the shipped max load (docs/silicon/03).
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
    }

    /// The slot capacity (`values.len()`; ctrl carries the mirror tail).
    #[inline(always)]
    pub(super) fn capacity(&self) -> usize {
        self.values.len()
    }

    /// Writes one ctrl byte, mirroring the head bytes into the tail so
    /// window loads never wrap.
    #[inline(always)]
    pub(super) fn set_ctrl(&mut self, idx: usize, value: u8) {
        self.ctrl[idx] = value;
        if idx < WINDOW - 1 {
            let capacity = self.capacity();
            self.ctrl[capacity + idx] = value;
        }
    }
}
