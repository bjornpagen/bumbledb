use super::{hash_words, Colt, Cursor, NodeRef};

impl Colt {
    /// Probes the selection levels with this execution's resolved words,
    /// in level order, forcing lazily exactly like join-level probes.
    /// The amortization contract: forcing selection level 0 walks the
    /// view once per generation; every subsequent param value is O(1)
    /// probes. `Some` sits at the first join level; `None` = no fact
    /// matches — the occurrence, and with it the whole conjunctive
    /// query, is empty on this snapshot.
    pub fn select(&mut self, keys: &[u64]) -> Option<Cursor> {
        debug_assert_eq!(
            keys.len(),
            self.selection_levels,
            "one resolved word per selection level"
        );
        let mut cursor = Self::root();
        for (level, key) in keys.iter().enumerate() {
            let key = std::slice::from_ref(key);
            cursor = self.probe_child_at(cursor, level, key, hash_words(key))?;
        }
        self.start = cursor;
        self.selected = true;
        Some(cursor)
    }

    /// The executor's per-execution start cursor: the root, or the
    /// post-selection cursor once [`Colt::select`] ran this execution.
    ///
    /// # Panics
    ///
    /// A release assert: starting a selection-bearing colt before
    /// `select()` would silently drop its selections — wrong results.
    /// Once per occurrence per execution; noise against the join.
    #[must_use]
    pub fn start(&self) -> Cursor {
        assert!(self.selected, "select() runs before the join");
        self.start
    }

    /// The root cursor (level 0).
    #[must_use]
    pub fn root() -> Cursor {
        Cursor::Node(NodeRef(0))
    }
}
