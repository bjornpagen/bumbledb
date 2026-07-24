//! The order-based overlap index (ruled 2026-07-23; finding 012 — the
//! `docs/architecture/40-execution.md` range-accelerator OPEN item,
//! armed "on violation" and tripped at bench scale by `t2_overlap_join`):
//! per key group, the start-sorted position list under an implicit
//! max-end tree, so "every position whose interval pair overlaps
//! `[q_start, q_end)`" enumerates in ~O(log n + out) instead of the
//! group's full n — the `Σ n_k²` all-pairs walk becomes
//! `Σ n_k log n_k + out` across a per-key self-join.
//!
//! The overlap predicate is the half-open shared-point law
//! (`10-data-model.md`): `a ∩ b ≠ ∅ ⇔ a.start < b.end ∧ b.start <
//! a.end` over the order-faithful encoded words — rays are ordinary
//! largest-end words, adjacency (`a.end == b.start`) shares no point
//! and is correctly excluded. The executor keeps the Allen mask as
//! data: enumerated candidates still flow through the uniform classify
//! kernels, so this structure only ever needs to be a *superset* filter
//! for connected masks (mask ⊆ INTERSECTS — the caller's gate).
//!
//! One cache serves one execution (the executor resets it per
//! `execute`): indexes key on the caller's (occurrence, bound prefix)
//! words — the trie path that minted the group's cursor — and build
//! exactly once per group, pooled slabs throughout (capacity retained
//! across executions, the 40-execution allocation contract).

/// One built index's directory row: ranges into the shared slabs.
#[derive(Debug, Clone, Copy)]
struct Dir {
    key_start: u32,
    key_len: u32,
    /// `starts`/`positions` base and entry count.
    base: u32,
    len: u32,
    /// This index's 1-based max-end tree: `2·p` words at `tree_base`;
    /// leaves `tree_base + p + j` hold entry j's end word.
    tree_base: u32,
    /// Padded leaf count (power of two, ≥ 1).
    p: u32,
}

/// One query's tree walk, hoisted to this index's slices.
struct Walk<'a> {
    tree: &'a [u64],
    positions: &'a [u32],
    /// The `start < q_end` prefix bound (exclusive entry index).
    hi: usize,
    q_start: u64,
}

impl Walk<'_> {
    /// Reports node `[lo, hi_node)`: prune right of the prefix bound
    /// or under the max-end bar; leaves report.
    fn report(&self, node: usize, lo: usize, hi_node: usize, out: &mut Vec<u32>) {
        if lo >= self.hi || self.tree[node] <= self.q_start {
            return;
        }
        if hi_node - lo == 1 {
            out.push(self.positions[lo]);
            return;
        }
        let mid = usize::midpoint(lo, hi_node);
        self.report(2 * node, lo, mid, out);
        self.report(2 * node + 1, mid, hi_node, out);
    }
}

/// The per-execution overlap-index cache: an open-addressed directory
/// over pooled index slabs.
#[derive(Default)]
pub(crate) struct OverlapCache {
    /// `table[i]` holds `dir index + 1`, 0 = empty (power-of-two sized,
    /// linear probing). Keys compare in full from the key slab on a
    /// hash hit — aliasing is a probe step, never a wrong answer.
    table: Vec<u32>,
    dirs: Vec<Dir>,
    keys: Vec<u64>,
    /// Start words, ascending per index.
    starts: Vec<u64>,
    /// Image positions, aligned with `starts`.
    positions: Vec<u32>,
    /// Implicit max-end trees. Pad leaves hold 0 — unreachable as an
    /// end word (`end > start ≥ 0` over order-faithful words), so a pad
    /// never satisfies `end > q_start`.
    tree: Vec<u64>,
    /// Build scratch: (start, end, position), start-sorted in place.
    triples: Vec<(u64, u64, u32)>,
}

impl OverlapCache {
    /// Drops every index and key, capacities retained — the caller's
    /// per-execution boundary (group positions and cursors are only
    /// stable within one execution).
    pub(crate) fn reset(&mut self) {
        self.table.iter_mut().for_each(|slot| *slot = 0);
        self.dirs.clear();
        self.keys.clear();
        self.starts.clear();
        self.positions.clear();
        self.tree.clear();
    }

    /// The index for `key`, building it from `feed` on first touch:
    /// `feed` pushes the group's (start word, end word, position)
    /// triples in any order; the build start-sorts them and erects the
    /// max-end tree. Returns the directory index for [`Self::query_into`].
    pub(crate) fn get_or_build(
        &mut self,
        key: &[u64],
        feed: impl FnOnce(&mut Vec<(u64, u64, u32)>),
    ) -> u32 {
        let hash = crate::exec::colt::hash_key(key);
        if let Some(found) = self.lookup(hash, key) {
            return found;
        }
        let mut triples = std::mem::take(&mut self.triples);
        triples.clear();
        feed(&mut triples);
        triples.sort_unstable_by_key(|&(start, _, _)| start);
        let len = triples.len();
        let p = len.next_power_of_two().max(1);
        let dir = Dir {
            key_start: u32::try_from(self.keys.len()).expect("slabs fit u32"),
            key_len: u32::try_from(key.len()).expect("keys are a few words"),
            base: u32::try_from(self.starts.len()).expect("slabs fit u32"),
            len: u32::try_from(len).expect("positions fit u32"),
            tree_base: u32::try_from(self.tree.len()).expect("slabs fit u32"),
            p: u32::try_from(p).expect("positions fit u32"),
        };
        self.keys.extend_from_slice(key);
        self.starts
            .extend(triples.iter().map(|&(start, _, _)| start));
        self.positions
            .extend(triples.iter().map(|&(_, _, position)| position));
        let tree_base = self.tree.len();
        self.tree.resize(tree_base + 2 * p, 0);
        for (j, &(_, end, _)) in triples.iter().enumerate() {
            self.tree[tree_base + p + j] = end;
        }
        for i in (1..p).rev() {
            self.tree[tree_base + i] =
                self.tree[tree_base + 2 * i].max(self.tree[tree_base + 2 * i + 1]);
        }
        self.triples = triples;
        let dir_idx = u32::try_from(self.dirs.len()).expect("dirs fit u32");
        self.dirs.push(dir);
        self.insert(hash, dir_idx);
        dir_idx
    }

    /// Entry count of a built index — the caller's stability tripwire
    /// (a group must not grow between build and query).
    pub(crate) fn len_of(&self, dir: u32) -> usize {
        self.dirs[dir as usize].len as usize
    }

    /// Every position of index `dir` whose interval overlaps
    /// `[q_start, q_end)`, into `out` (cleared; start-ordered): binary
    /// search bounds the `start < q_end` prefix, the max-end tree walk
    /// reports `end > q_start` within it — each visited node either
    /// prunes or has a report in its subtree.
    pub(crate) fn query_into(&self, dir: u32, q_start: u64, q_end: u64, out: &mut Vec<u32>) {
        out.clear();
        let d = self.dirs[dir as usize];
        let base = d.base as usize;
        let starts = &self.starts[base..base + d.len as usize];
        let hi = starts.partition_point(|&start| start < q_end);
        if hi == 0 {
            return;
        }
        let walk = Walk {
            tree: &self.tree[d.tree_base as usize..d.tree_base as usize + 2 * d.p as usize],
            positions: &self.positions[base..base + d.len as usize],
            hi,
            q_start,
        };
        walk.report(1, 0, d.p as usize, out);
    }

    /// Directory probe: full-key equality behind the hash.
    fn lookup(&self, hash: u64, key: &[u64]) -> Option<u32> {
        if self.table.is_empty() {
            return None;
        }
        let mask = self.table.len() - 1;
        let mut idx = usize::try_from(hash).expect("64-bit usize") & mask;
        loop {
            match self.table[idx] {
                0 => return None,
                entry => {
                    let dir = self.dirs[(entry - 1) as usize];
                    let stored =
                        &self.keys[dir.key_start as usize..(dir.key_start + dir.key_len) as usize];
                    if stored == key {
                        return Some(entry - 1);
                    }
                    idx = (idx + 1) & mask;
                }
            }
        }
    }

    /// Inserts a fresh dir, growing at half load (rehash from the key
    /// slab — the table stores no hashes).
    fn insert(&mut self, hash: u64, dir_idx: u32) {
        if (self.dirs.len() + 1) * 2 > self.table.len() {
            let capacity = (self.table.len() * 2).max(64);
            self.table.clear();
            self.table.resize(capacity, 0);
            for existing in 0..dir_idx {
                let dir = self.dirs[existing as usize];
                let stored =
                    &self.keys[dir.key_start as usize..(dir.key_start + dir.key_len) as usize];
                let rehash = crate::exec::colt::hash_key(stored);
                self.place(rehash, existing);
            }
        }
        self.place(hash, dir_idx);
    }

    fn place(&mut self, hash: u64, dir_idx: u32) {
        let mask = self.table.len() - 1;
        let mut idx = usize::try_from(hash).expect("64-bit usize") & mask;
        while self.table[idx] != 0 {
            idx = (idx + 1) & mask;
        }
        self.table[idx] = dir_idx + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::OverlapCache;

    /// A deterministic LCG (the sweep tests' twin) so the property
    /// sweeps are reproducible.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }
    }

    /// Random half-open segments over a small domain — dense enough for
    /// constant adjacency/nesting; every fifth draw a ray.
    fn random_group(rng: &mut Lcg, len: usize) -> Vec<(u64, u64, u32)> {
        (0..len)
            .map(|i| {
                let start = rng.next() % 40;
                let end = if rng.next().is_multiple_of(5) {
                    u64::MAX
                } else {
                    start + 1 + rng.next() % 9
                };
                (start, end, u32::try_from(i).expect("small") * 3)
            })
            .collect()
    }

    fn naive(group: &[(u64, u64, u32)], q_start: u64, q_end: u64) -> Vec<u32> {
        let mut hits: Vec<u32> = group
            .iter()
            .filter(|&&(start, end, _)| start < q_end && q_start < end)
            .map(|&(_, _, position)| position)
            .collect();
        hits.sort_unstable();
        hits
    }

    #[test]
    fn queries_match_the_naive_filter_across_random_groups() {
        let mut rng = Lcg(0x0BEE);
        let mut cache = OverlapCache::default();
        let mut out = Vec::new();
        for round in 0..200u64 {
            let len = (rng.next() % 70) as usize;
            let group = random_group(&mut rng, len);
            let dir = cache.get_or_build(&[round], |triples| {
                triples.extend_from_slice(&group);
            });
            assert_eq!(cache.len_of(dir), group.len());
            for _ in 0..20 {
                let q_start = rng.next() % 45;
                let q_end = if rng.next().is_multiple_of(4) {
                    u64::MAX
                } else {
                    q_start + 1 + rng.next() % 12
                };
                cache.query_into(dir, q_start, q_end, &mut out);
                let mut got = out.clone();
                got.sort_unstable();
                assert_eq!(
                    got,
                    naive(&group, q_start, q_end),
                    "group {group:?} query [{q_start}, {q_end})"
                );
            }
        }
    }

    #[test]
    fn adjacency_shares_no_point_and_rays_hit_everything_after_their_start() {
        let mut cache = OverlapCache::default();
        let group = [(0u64, 5u64, 0u32), (5, 9, 1), (7, u64::MAX, 2)];
        let dir = cache.get_or_build(&[7], |t| t.extend_from_slice(&group));
        let mut out = Vec::new();
        // [5, 7): adjacent to [0,5) — excluded; overlaps [5,9); the ray
        // starts at its end boundary — excluded (half-open).
        cache.query_into(dir, 5, 7, &mut out);
        assert_eq!(out, vec![1]);
        // A query ray from 6 hits the open segment and the ray.
        cache.query_into(dir, 6, u64::MAX, &mut out);
        out.sort_unstable();
        assert_eq!(out, vec![1, 2]);
        // A window strictly before every segment matches nothing.
        cache.query_into(dir, 0, 0, &mut out);
        assert!(out.is_empty(), "an empty window matches nothing");
    }

    #[test]
    fn groups_build_once_and_key_on_full_words() {
        let mut cache = OverlapCache::default();
        let mut builds = 0usize;
        for _ in 0..3 {
            cache.get_or_build(&[1, 2], |t| {
                builds += 1;
                t.push((0, 4, 9));
            });
        }
        assert_eq!(builds, 1, "second touch is a cache hit");
        // A distinct key must not alias, whatever the hash does.
        let other = cache.get_or_build(&[1, 3], |t| {
            t.push((10, 14, 5));
        });
        let mut out = Vec::new();
        cache.query_into(other, 11, 12, &mut out);
        assert_eq!(out, vec![5]);
        cache.reset();
        let mut rebuilt = 0usize;
        cache.get_or_build(&[1, 2], |t| {
            rebuilt += 1;
            t.push((0, 4, 9));
        });
        assert_eq!(rebuilt, 1, "reset drops every index");
    }

    /// Many keys force directory growth mid-stream; every earlier index
    /// stays reachable and correct after the rehash.
    #[test]
    fn directory_growth_preserves_every_index() {
        let mut cache = OverlapCache::default();
        let dirs: Vec<u32> = (0..300u64)
            .map(|k| {
                cache.get_or_build(&[k], |t| {
                    t.push((k, k + 2, u32::try_from(k).expect("small")));
                })
            })
            .collect();
        let mut out = Vec::new();
        for (k, &dir) in dirs.iter().enumerate() {
            let k64 = k as u64;
            assert_eq!(cache.get_or_build(&[k64], |_| panic!("already built")), dir);
            cache.query_into(dir, k64 + 1, k64 + 2, &mut out);
            assert_eq!(out, vec![u32::try_from(k).expect("small")]);
        }
    }
}
