use super::{
    Colt, Map, NodeRef, NodeState, Positions, Slot, ctrl_tag, hash_core, hash_words, pack_child,
};

/// Positions staged per force run — the build side's batch width,
/// matching the read path's column-hoisted gather discipline.
const FORCE_BATCH: usize = 256;

impl Colt {
    /// Two-phase force: iterate the node's positions once in staged
    /// runs — phase 1 gathers a run's key words column-hoisted
    /// ([`Colt::gather_keys`], the read path's idiom: each column
    /// resolves its view slice once per run, ~1 load per (position,
    /// column) instead of an enum match and two bounds checks each),
    /// phase 2 ingests the staged rows arity-monomorphic (one dispatch
    /// per run; hash and probe under const arity, the `probe_walk`
    /// precedent). The paper names trie building the major bottleneck —
    /// the build side pays the read side's cost class, not its own.
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

        // The staging buffers are pooled scratch (capacity retained
        // across forces — the allocation contract's steady state); the
        // key buffer sizes once per force, every run overwrites its
        // prefix.
        let mut keys = std::mem::take(&mut self.stage_keys);
        let mut positions = std::mem::take(&mut self.stage_positions);
        keys.resize(FORCE_BATCH * arity, 0);

        // Single pass, O(1) advance per position: the root walks the view
        // by index (O(1) each); a chunked list walks its chain directly —
        // never `nth_position`'s from-the-head re-walk, which made forcing
        // a k-position child O(k²/64).
        match self.nodes[node.0 as usize] {
            NodeState::Unforced(Positions::Root) => {
                let n = self.view.len();
                let mut base = 0usize;
                while base < n {
                    let take = FORCE_BATCH.min(n - base);
                    positions.clear();
                    positions.extend((base..base + take).map(|idx| self.view.position_at(idx)));
                    self.force_run(&mut m, level, &positions, &mut keys);
                    base += take;
                }
            }
            NodeState::Unforced(Positions::Chunks { first, .. }) => {
                let mut chunk = first;
                while chunk != u32::MAX {
                    let c = self.chunks[chunk as usize];
                    positions.clear();
                    positions.extend_from_slice(
                        &self.chunk_positions[c.start as usize..][..usize::from(c.len)],
                    );
                    self.force_run(&mut m, level, &positions, &mut keys);
                    chunk = c.next;
                }
            }
            NodeState::Forced { .. } => unreachable!("checked above"),
        }
        self.stage_keys = keys;
        self.stage_positions = positions;

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

    /// One staged run of a [`Colt::force`] pass: gather the run's key
    /// words column-hoisted, then dispatch the ingest loop once on the
    /// level's arity.
    fn force_run(&mut self, m: &mut Map, level: usize, positions: &[u32], keys: &mut Vec<u64>) {
        self.gather_keys(level, positions, keys, 0);
        match m.arity {
            1 => self.ingest_run::<1>(m, positions, keys),
            2 => self.ingest_run::<2>(m, positions, keys),
            3 => self.ingest_run::<3>(m, positions, keys),
            4 => self.ingest_run::<4>(m, positions, keys),
            _ => self.ingest_run_general(m, positions, keys),
        }
    }

    /// The arity-monomorphic ingest: probe each staged row and land it
    /// (new slot or appended child), rehash-doubling first when the next
    /// insert would cross the 0.4 max load —
    /// `(len + 1) * 5 > nbuckets * 16`, i.e. `len + 1 > 0.4 · 8 · nbuckets`.
    /// The hash is `hash_core::<A>` (hash-identical to [`hash_words`] by
    /// the one-fold construction) and the probe's own arity dispatch
    /// constant-folds under the const key width.
    fn ingest_run<const A: usize>(&mut self, m: &mut Map, positions: &[u32], keys: &[u64]) {
        for (k, &position) in positions.iter().enumerate() {
            let key = &keys[k * A..k * A + A];
            let hash = hash_core::<A>(key);
            self.ingest_one(m, key, hash, position);
        }
    }

    /// [`Colt::ingest_run`]'s runtime-arity fallback (arity > 4 —
    /// beyond every bench plan, the `probe_walk_general` twin).
    fn ingest_run_general(&mut self, m: &mut Map, positions: &[u32], keys: &[u64]) {
        let arity = m.arity;
        for (k, &position) in positions.iter().enumerate() {
            let key = &keys[k * arity..k * arity + arity];
            let hash = hash_words(key);
            self.ingest_one(m, key, hash, position);
        }
    }

    /// One staged row: probe and land it.
    #[inline(always)]
    fn ingest_one(&mut self, m: &mut Map, key: &[u64], hash: u64, position: u32) {
        // Growth is checked before the probe, so a position that merely
        // appends to an existing key can still trigger a double — an
        // over-size by at most one doubling step, closed by audit as
        // no-action: checking after the probe would probe the old table
        // and insert into the new one. The trigger is the 0.4 max load
        // (measured): `(len + 1) > 0.4 · 8 · nbuckets`.
        if (usize::try_from(m.len).expect("64-bit usize") + 1) * 5 > m.nbuckets * 16 {
            self.grow_map(m);
        }
        let (found, idx) = self.probe_hashed(m, key, hash);
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
    }
}
