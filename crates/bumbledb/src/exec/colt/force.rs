use super::{Colt, Map, NodeRef, NodeState, Positions, Slot, ctrl_tag, hash_words, pack_child};

impl Colt {
    /// Single-pass force: iterate the node's positions once, decoding key
    /// words and appending each position to its key's chunked child list.
    /// Returns the map index (idempotent).
    pub(super) fn force(&mut self, node: NodeRef, level: usize) -> u32 {
        if let NodeState::Forced { map } = self.nodes[node.0 as usize] {
            return map;
        }
        let arity = self.arity_at(level);
        let count = match self.nodes[node.0 as usize] {
            NodeState::Unforced(Positions::Root) => self.view.len() as u64,
            NodeState::Unforced(Positions::Chunks { count, .. }) => u64::from(count),
            NodeState::Forced { .. } => unreachable!("checked above"),
        };
        // Initial sizing: distinct keys are unknown
        // before the pass, so start from the same deterministic guess as
        // the linear layout — `clamp(count/8, 16, 2*count)` — and size
        // buckets for ≤ 0.4 load (the measured occupancy-invariant band):
        // `nbuckets = next_pow2(guess * 5 / 16)` (5/16 = 1/(8·0.4)),
        // rehash-doubling in bucket units when the guess was short.
        let count_usize = usize::try_from(count).expect("64-bit usize");
        let guess = (count_usize / 8).max(16).min(count_usize.max(1) * 2);
        let nbuckets = (guess * 5 / 16).max(1).next_power_of_two();
        let map_idx = u32::try_from(self.maps.len()).expect("map count fits u32");
        let ctrl_start = self.ctrl.len();
        let bucket_start = self.buckets.len();
        let dense_start = self.dense.len();
        self.ctrl.resize(ctrl_start + nbuckets * 8, 0);
        self.buckets
            .resize(bucket_start + nbuckets * (8 * arity + 8), 0);
        let mut m = Map {
            arity,
            nbuckets,
            len: 0,
            ctrl_start,
            bucket_start,
            dense_start,
        };

        // Single pass, O(1) advance per position: the root walks the view
        // by index (O(1) each); a chunked list walks its chain directly —
        // never `nth_position`'s from-the-head re-walk, which made forcing
        // a k-position child O(k²/64).
        match self.nodes[node.0 as usize] {
            NodeState::Unforced(Positions::Root) => {
                for idx in 0..self.view.len() {
                    let position = self.view.position_at(idx);
                    self.force_ingest(&mut m, level, position);
                }
            }
            NodeState::Unforced(Positions::Chunks { first, .. }) => {
                let mut chunk = first;
                while chunk != u32::MAX {
                    let c = self.chunks[chunk as usize];
                    for i in 0..usize::from(c.len) {
                        self.force_ingest(&mut m, level, c.positions[i]);
                    }
                    chunk = c.next;
                }
            }
            NodeState::Forced { .. } => unreachable!("checked above"),
        }

        crate::obs::event(
            crate::obs::names::COLT_FORCE,
            crate::obs::Category::Execute,
            count,
            u64::from(m.len),
        );
        self.maps.push(m);
        self.nodes[node.0 as usize] = NodeState::Forced { map: map_idx };
        map_idx
    }

    /// One position of a [`Colt::force`] pass: decode its key words, probe,
    /// and land it (new slot or appended child), rehash-doubling first
    /// when the next insert would cross the 0.4 max load —
    /// `(len + 1) * 5 > nbuckets * 16`, i.e. `len + 1 > 0.4 · 8 · nbuckets`.
    fn force_ingest(&mut self, m: &mut Map, level: usize, position: u32) {
        // Growth is checked before the probe, so a position that merely
        // appends to an existing key can still trigger a double — an
        // over-size by at most one doubling step, closed by audit as
        // no-action: checking after the probe would probe the old table
        // and insert into the new one. The trigger is the 0.4 max load
        // (measured): `(len + 1) > 0.4 · 8 · nbuckets`.
        if (usize::try_from(m.len).expect("64-bit usize") + 1) * 5 > m.nbuckets * 16 {
            self.grow_map(m);
        }
        self.scratch.clear();
        for col in &self.schema_columns[level] {
            let w = self.word_at(*col, position);
            self.scratch.push(w);
        }
        let key = std::mem::take(&mut self.scratch);
        let hash = hash_words(&key);
        let (found, idx) = self.probe_hashed(m, &key, hash);
        if found {
            self.append_child(m.child_at(idx), position);
        } else {
            self.ctrl[m.ctrl_start + idx] = ctrl_tag(hash);
            for (i, w) in key.iter().enumerate() {
                self.buckets[m.key_word_at(idx, i)] = *w;
            }
            self.buckets[m.child_at(idx)] = pack_child(Slot::Single(position));
            self.dense
                .push(u32::try_from(idx).expect("slot index fits u32"));
            m.len += 1;
        }
        self.scratch = key;
    }
}
