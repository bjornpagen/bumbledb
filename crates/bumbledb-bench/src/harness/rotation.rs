use super::Rotation;

impl<T> Rotation<T> {
    /// # Panics
    ///
    /// On an empty set vector (even param-less families carry one empty
    /// set).
    #[must_use]
    pub fn new(sets: Vec<T>) -> Self {
        assert!(!sets.is_empty(), "a rotation needs at least one set");
        Self { sets, cursor: 0 }
    }

    /// The next param set, wrapping around.
    pub fn next_set(&mut self) -> &T {
        let index = self.next_index();
        &self.sets[index]
    }

    /// The next slot index, wrapping around — for callers indexing
    /// parallel vectors (the `SQLite` twins pair a prepared family with
    /// its draw), so ours/theirs rotation phase is one definition, not
    /// a hand-copied modulus.
    pub fn next_index(&mut self) -> usize {
        let index = self.cursor;
        self.cursor = (self.cursor + 1) % self.sets.len();
        index
    }
}
