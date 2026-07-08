use bumbledb::Value;

use super::Rotation;

impl Rotation {
    /// # Panics
    ///
    /// On an empty set vector (even param-less families carry one empty
    /// set).
    #[must_use]
    pub fn new(sets: Vec<Vec<Value>>) -> Self {
        assert!(!sets.is_empty(), "a rotation needs at least one set");
        Self { sets, cursor: 0 }
    }

    /// The next param set, wrapping around.
    pub fn next_set(&mut self) -> &[Value] {
        let set = &self.sets[self.cursor];
        self.cursor = (self.cursor + 1) % self.sets.len();
        set
    }
}
