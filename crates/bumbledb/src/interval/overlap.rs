//! The order-based overlap index:
//! per key group, the start-sorted position list under an implicit
//! max-end tree, so "every position whose interval pair overlaps
//! `[q_start, q_end)`" enumerates in ~O(log n + out) instead of the
//! group's full n — the `Σ n_k²` all-pairs walk becomes
//! `Σ n_k log n_k + out` across a per-key self-join. Small groups skip
use std::num::NonZeroU32;

const FLAT_SWEEP_CEILING: usize = 128;

#[derive(Debug, Clone, Copy)]
enum Dir {
    Tallied {
        key_start: u32,
        key_len: u32,
    },
    Built {
        key_start: u32,
        key_len: u32,
        base: u32,
        len: u32,
        tree_base: u32,
        p: NonZeroU32,
    },
}

impl Dir {
    fn key_span(self) -> (u32, u32) {
        match self {
            Self::Tallied { key_start, key_len }
            | Self::Built {
                key_start, key_len, ..
            } => (key_start, key_len),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Probe {
    Declined,
    Ready(u32),
}

struct Walk<'a> {
    tree: &'a [u64],
    positions: &'a [u32],

    hi: usize,
    q_start: u64,
}

impl Walk<'_> {
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

#[derive(Default)]
pub(crate) struct OverlapCache {
    table: Vec<u32>,
    dirs: Vec<Dir>,
    keys: Vec<u64>,

    starts: Vec<u64>,

    positions: Vec<u32>,

    tree: Vec<u64>,

    triples: Vec<(u64, u64, u32)>,
}

impl OverlapCache {
    pub(crate) fn reset(&mut self) {
        self.table.iter_mut().for_each(|slot| *slot = 0);
        self.dirs.clear();
        self.keys.clear();
        self.starts.clear();
        self.positions.clear();
        self.tree.clear();
    }

    pub(crate) fn probe(
        &mut self,
        key: &[u64],
        feed: impl FnOnce(&mut Vec<(u64, u64, u32)>),
    ) -> Probe {
        let hash = crate::exec::colt::hash_key(key);
        let Some(found) = self.lookup(hash, key) else {
            let dir = Dir::Tallied {
                key_start: u32::try_from(self.keys.len()).expect("slabs fit u32"),
                key_len: u32::try_from(key.len()).expect("keys are a few words"),
            };
            self.keys.extend_from_slice(key);
            let dir_idx = u32::try_from(self.dirs.len()).expect("dirs fit u32");
            self.dirs.push(dir);
            self.insert(hash, dir_idx);
            return Probe::Declined;
        };
        if matches!(self.dirs[found as usize], Dir::Built { .. }) {
            return Probe::Ready(found);
        }
        let mut triples = std::mem::take(&mut self.triples);
        triples.clear();
        feed(&mut triples);
        triples.sort_unstable_by_key(|&(start, _, _)| start);
        let len = triples.len();
        let p = NonZeroU32::new(
            u32::try_from(len.next_power_of_two().max(1)).expect("positions fit u32"),
        )
        .expect("padded leaf count is ≥ 1");
        let (key_start, key_len) = self.dirs[found as usize].key_span();
        self.dirs[found as usize] = Dir::Built {
            key_start,
            key_len,
            base: u32::try_from(self.starts.len()).expect("slabs fit u32"),
            len: u32::try_from(len).expect("positions fit u32"),
            tree_base: u32::try_from(self.tree.len()).expect("slabs fit u32"),
            p,
        };
        self.starts
            .extend(triples.iter().map(|&(start, _, _)| start));
        self.positions
            .extend(triples.iter().map(|&(_, _, position)| position));
        let tree_base = self.tree.len();
        let p_usize = p.get() as usize;
        self.tree.resize(tree_base + 2 * p_usize, 0);
        for (j, &(_, end, _)) in triples.iter().enumerate() {
            self.tree[tree_base + p_usize + j] = end;
        }
        for i in (1..p_usize).rev() {
            self.tree[tree_base + i] =
                self.tree[tree_base + 2 * i].max(self.tree[tree_base + 2 * i + 1]);
        }
        self.triples = triples;
        Probe::Ready(found)
    }

    /// (a group must not grow between build and query).
    pub(crate) fn len_of(&self, dir: u32) -> usize {
        match self.dirs[dir as usize] {
            Dir::Built { len, .. } => len as usize,
            Dir::Tallied { .. } => unreachable!("len_of is for built indexes"),
        }
    }

    pub(crate) fn query_into(&self, dir: u32, q_start: u64, q_end: u64, out: &mut Vec<u32>) {
        out.clear();
        let d = self.dirs[dir as usize];
        let Dir::Built {
            base,
            len,
            tree_base,
            p,
            ..
        } = d
        else {
            unreachable!("queries touch built indexes only");
        };
        let p = p.get() as usize;
        let base = base as usize;
        let len = len as usize;
        let starts = &self.starts[base..base + len];
        let positions = &self.positions[base..base + len];
        if len <= FLAT_SWEEP_CEILING {
            let ends = &self.tree[tree_base as usize + p..][..len];
            for j in 0..len {
                if starts[j] >= q_end {
                    break;
                }
                if ends[j] > q_start {
                    out.push(positions[j]);
                }
            }
            return;
        }
        let hi = starts.partition_point(|&start| start < q_end);
        if hi == 0 {
            return;
        }
        let walk = Walk {
            tree: &self.tree[tree_base as usize..tree_base as usize + 2 * p],
            positions,
            hi,
            q_start,
        };
        walk.report(1, 0, p, out);
    }

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
                    let (key_start, key_len) = dir.key_span();
                    let stored = &self.keys[key_start as usize..(key_start + key_len) as usize];
                    if stored == key {
                        return Some(entry - 1);
                    }
                    idx = (idx + 1) & mask;
                }
            }
        }
    }

    fn insert(&mut self, hash: u64, dir_idx: u32) {
        if (self.dirs.len() + 1) * 2 > self.table.len() {
            let capacity = (self.table.len() * 2).max(64);
            self.table.clear();
            self.table.resize(capacity, 0);
            for existing in 0..dir_idx {
                let dir = self.dirs[existing as usize];
                let (key_start, key_len) = dir.key_span();
                let stored = &self.keys[key_start as usize..(key_start + key_len) as usize];
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
    use super::{OverlapCache, Probe};

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

    fn build(cache: &mut OverlapCache, key: &[u64], group: &[(u64, u64, u32)]) -> u32 {
        assert_eq!(
            cache.probe(key, |_| panic!("a first probe never builds")),
            Probe::Declined,
            "the first probe declines"
        );
        match cache.probe(key, |triples| triples.extend_from_slice(group)) {
            Probe::Ready(dir) => dir,
            Probe::Declined => panic!("the second probe builds"),
        }
    }

    #[test]
    fn queries_match_the_naive_filter_across_random_groups() {
        let mut rng = Lcg(0x0BEE);
        let mut cache = OverlapCache::default();
        let mut out = Vec::new();
        for round in 0..200u64 {
            let len = (rng.next() % 300) as usize;
            let group = random_group(&mut rng, len);
            let dir = build(&mut cache, &[round], &group);
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
        let dir = build(&mut cache, &[7], &group);
        let mut out = Vec::new();

        cache.query_into(dir, 5, 7, &mut out);
        assert_eq!(out, vec![1]);

        cache.query_into(dir, 6, u64::MAX, &mut out);
        out.sort_unstable();
        assert_eq!(out, vec![1, 2]);
        // A window strictly before every segment matches nothing.
        cache.query_into(dir, 0, 0, &mut out);
        assert!(out.is_empty(), "an empty window matches nothing");
    }

    #[test]
    fn groups_build_once_on_the_second_probe_and_key_on_full_words() {
        let mut cache = OverlapCache::default();
        let mut builds = 0usize;
        for touch in 0..4 {
            let dir = cache.probe(&[1, 2], |t| {
                builds += 1;
                t.push((0, 4, 9));
            });
            assert_eq!(
                dir == Probe::Declined,
                touch == 0,
                "only the first probe declines"
            );
        }
        assert_eq!(builds, 1, "the second probe builds; later touches hit");
        // A distinct key must not alias, whatever the hash does.
        let other = build(&mut cache, &[1, 3], &[(10, 14, 5)]);
        let mut out = Vec::new();
        cache.query_into(other, 11, 12, &mut out);
        assert_eq!(out, vec![5]);
        cache.reset();
        assert_eq!(
            cache.probe(&[1, 2], |_| panic!("reset drops every tally")),
            Probe::Declined,
            "reset drops every index and tally"
        );
    }

    #[test]
    fn once_probed_groups_never_build() {
        let mut cache = OverlapCache::default();
        for k in 0..300u64 {
            assert_eq!(
                cache.probe(&[k], |_| panic!("a once-probed group must not build")),
                Probe::Declined
            );
        }
    }

    /// stays reachable and correct after the rehash.
    #[test]
    fn directory_growth_preserves_every_index() {
        let mut cache = OverlapCache::default();
        let dirs: Vec<u32> = (0..300u64)
            .map(|k| {
                build(
                    &mut cache,
                    &[k],
                    &[(k, k + 2, u32::try_from(k).expect("small"))],
                )
            })
            .collect();
        let mut out = Vec::new();
        for (k, &dir) in dirs.iter().enumerate() {
            let k64 = k as u64;
            assert_eq!(
                cache.probe(&[k64], |_| panic!("already built")),
                Probe::Ready(dir),
                "a built key stays a pure lookup"
            );
            cache.query_into(dir, k64 + 1, k64 + 2, &mut out);
            assert_eq!(out, vec![u32::try_from(k).expect("small")]);
        }
    }
}
