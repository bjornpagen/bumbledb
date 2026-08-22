use super::Rotation;

impl<T> Rotation<T> {
    /// # Panics
    #[must_use]
    pub fn new(sets: Vec<T>) -> Self {
        assert!(!sets.is_empty(), "a rotation needs at least one set");
        Self { sets, cursor: 0 }
    }

    pub fn next_set(&mut self) -> &T {
        let index = self.next_index();
        &self.sets[index]
    }

    pub fn next_index(&mut self) -> usize {
        let index = self.cursor;
        self.cursor = (self.cursor + 1) % self.sets.len();
        index
    }
}
