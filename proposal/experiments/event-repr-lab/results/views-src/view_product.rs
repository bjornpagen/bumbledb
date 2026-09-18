//! Exact mapped conjunction/projection without resident renamed operands.
//! Maps and elimination are fixed per invocation; memo entries never escape it.
use super::{Arena, Ref, View, axes, scatter, word_kernels, words};
use rustc_hash::FxHashMap;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Default)]
pub struct Statistics {
    pub subproblems: usize,
    pub memo_hits: usize,
    pub local_cubes: usize,
    pub borrowed_planes: usize,
    pub cofactor_steps: usize,
    pub permutation_steps: usize,
    pub broadcast_steps: usize,
    pub branch_planes: usize,
    pub branch_assignments: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Restricted {
    root: Ref,
    pinned: u64,
    values: u64,
}
impl Restricted {
    fn new(root: Ref) -> Self {
        Self {
            root,
            pinned: 0,
            values: 0,
        }
    }
    fn normalize<const K: u32>(mut self, arena: &Arena<K>) -> Self {
        loop {
            let variables = arena.variables(self.root);
            self.pinned &= variables;
            self.values &= self.pinned;
            if variables & !self.pinned == 0 {
                return Self::new(arena.evaluate(self.root, self.values) as Ref);
            }
            if let View::Branch {
                variable,
                low,
                high,
            } = arena.view(self.root)
            {
                if self.pinned >> variable & 1 != 0 {
                    self.root = if self.values >> variable & 1 == 0 {
                        low
                    } else {
                        high
                    };
                    continue;
                }
            }
            return self;
        }
    }
    fn pin<const K: u32>(mut self, arena: &Arena<K>, coordinate: u32, high: bool) -> Self {
        if arena.variables(self.root) >> coordinate & 1 != 0 {
            self.pinned |= 1 << coordinate;
            self.values = (self.values & !(1 << coordinate)) | (u64::from(high) << coordinate);
        }
        self.normalize(arena)
    }
}

struct Map {
    destinations: Vec<u32>,
    sources: Vec<u32>,
}
impl Map {
    fn checked(dimensions: u32, destinations: &[u32]) {
        assert_eq!(
            destinations.len(),
            dimensions as usize,
            "map dimension mismatch"
        );
        let mut seen = 0u64;
        for &v in destinations {
            assert!(
                v < dimensions && seen >> v & 1 == 0,
                "map must be a bijection"
            );
            seen |= 1 << v;
        }
    }
    fn compose(input: &[u32], output: &[u32]) -> Self {
        let destinations: Vec<_> = input.iter().map(|&v| output[v as usize]).collect();
        let mut sources = vec![0; input.len()];
        for (old, &new) in destinations.iter().enumerate() {
            sources[new as usize] = old as u32;
        }
        Self {
            destinations,
            sources,
        }
    }
    fn mask(&self, mut source: u64) -> u64 {
        let mut mapped = 0;
        while source != 0 {
            mapped |= 1 << self.destinations[source.trailing_zeros() as usize];
            source &= source - 1;
        }
        mapped
    }
}

enum Plane<'a> {
    Constant(bool),
    Table { data: Cow<'a, [u64]>, flip: u64 },
}
impl Plane<'_> {
    fn word(&self, i: usize) -> u64 {
        match self {
            Self::Constant(value) => {
                if *value {
                    !0
                } else {
                    0
                }
            }
            Self::Table { data, flip } => data[i] ^ flip,
        }
    }
}

fn plane<'a, const K: u32>(
    arena: &'a Arena<K>,
    view: Restricted,
    map: &Map,
    target: u64,
    stats: &mut Statistics,
) -> Plane<'a> {
    assert_eq!(map.mask(view.pinned) & target, 0);
    match arena.view(view.root) {
        View::Constant(value) => Plane::Constant(value),
        View::Table {
            mut variables,
            words,
            complemented,
        } => {
            let mut data = Cow::Borrowed(words);
            for v in axes(view.pinned) {
                let position = (variables & ((1 << v) - 1)).count_ones();
                data = Cow::Owned(word_kernels::cofactor(
                    &data,
                    variables.count_ones(),
                    position,
                    view.values >> v & 1 != 0,
                ));
                variables &= !(1 << v);
                stats.cofactor_steps += 1;
            }
            let mut mapped = map.mask(variables);
            assert_eq!(mapped & !target, 0);
            let names = axes(mapped);
            let permutation: Vec<_> = axes(variables)
                .iter()
                .map(|&v| {
                    names
                        .iter()
                        .position(|&w| w == map.destinations[v as usize])
                        .unwrap() as u32
                })
                .collect();
            if permutation
                .iter()
                .enumerate()
                .any(|(i, &v)| i != v as usize)
            {
                word_kernels::permute(data.to_mut(), &permutation);
                stats.permutation_steps += 1;
            }
            for v in axes(target & !mapped) {
                let position = (mapped & ((1 << v) - 1)).count_ones();
                data = Cow::Owned(word_kernels::broadcast(
                    &data,
                    mapped.count_ones(),
                    position,
                ));
                mapped |= 1 << v;
                stats.broadcast_steps += 1;
            }
            assert_eq!(mapped, target);
            if matches!(&data, Cow::Borrowed(_)) {
                stats.borrowed_planes += 1;
            }
            Plane::Table {
                data,
                flip: if complemented { !0 } else { 0 },
            }
        }
        View::Branch { .. } => {
            // A different target order can pin a descendant before its root.
            // Small residual dependence does not make this raw node a table.
            stats.branch_planes += 1;
            let source_coordinates: Vec<_> = axes(target)
                .iter()
                .map(|&v| map.sources[v as usize])
                .collect();
            let cells = 1usize << target.count_ones();
            stats.branch_assignments += cells;
            let data = words(cells, |i| {
                arena.evaluate(view.root, view.values | scatter(i, &source_coordinates))
            });
            Plane::Table {
                data: Cow::Owned(data),
                flip: 0,
            }
        }
    }
}

struct Product {
    maps: [Map; 2],
    eliminate: u64,
    memo: FxHashMap<[Restricted; 2], Ref>,
    stats: Statistics,
}
impl Product {
    fn visit<const K: u32>(&mut self, arena: &mut Arena<K>, views: [Restricted; 2]) -> Ref {
        if views.iter().any(|v| v.root == 0) {
            return 0;
        }
        if views.iter().all(|v| v.root == 1) {
            return 1;
        }
        if let Some(&out) = self.memo.get(&views) {
            self.stats.memo_hits += 1;
            return out;
        }
        self.stats.subproblems += 1;
        assert!(
            self.stats.subproblems <= 2_000_000,
            "RESOURCE_CAP: mapped product subproblems"
        );
        let live = views.map(|v| arena.variables(v.root) & !v.pinned);
        let variables = self.maps[0].mask(live[0]) | self.maps[1].mask(live[1]);
        // Complementary roots cancel only under the same effective substitution.
        if views[0].root == (views[1].root ^ 1)
            && views[0].pinned == views[1].pinned
            && views[0].values == views[1].values
            && axes(live[0]).iter().all(|&v| {
                self.maps[0].destinations[v as usize] == self.maps[1].destinations[v as usize]
            })
        {
            return 0;
        }
        let out = if variables.count_ones() <= K {
            self.stats.local_cubes += 1;
            // Finish immutable reads before canonical insertion can grow slabs.
            let mut data: Vec<u64> = {
                let a = plane(arena, views[0], &self.maps[0], variables, &mut self.stats);
                let b = plane(arena, views[1], &self.maps[1], variables, &mut self.stats);
                (0..(1usize << variables.count_ones()).div_ceil(64))
                    .map(|i| a.word(i) & b.word(i))
                    .collect()
            };
            let mut retained = variables;
            for v in axes(variables & self.eliminate) {
                let position = (retained & ((1 << v) - 1)).count_ones();
                let low = word_kernels::cofactor(&data, retained.count_ones(), position, false);
                let high = word_kernels::cofactor(&data, retained.count_ones(), position, true);
                data = low.iter().zip(high).map(|(a, b)| *a | b).collect();
                retained &= !(1 << v);
            }
            arena.table(retained, data)
        } else {
            let v = arena.top(variables);
            let low_views = std::array::from_fn(|i| {
                views[i].pin(arena, self.maps[i].sources[v as usize], false)
            });
            let low = self.visit(arena, low_views);
            if self.eliminate >> v & 1 != 0 && low == 1 {
                1
            } else {
                let high_views = std::array::from_fn(|i| {
                    views[i].pin(arena, self.maps[i].sources[v as usize], true)
                });
                let high = self.visit(arena, high_views);
                if self.eliminate >> v & 1 != 0 {
                    arena.apply(14, low, high)
                } else {
                    arena.branch(v, low, high)
                }
            }
        };
        self.memo.insert(views, out);
        out
    }
}

impl<const K: u32> Arena<K> {
    /// permute(exists(permute(a,am) & permute(b,bm), mask), output).
    /// All maps are checked bijections over this raw full binary presentation.
    pub fn mapped_product(
        &mut self,
        a: Ref,
        am: &[u32],
        b: Ref,
        bm: &[u32],
        mask: u64,
        output: &[u32],
    ) -> Ref {
        self.mapped_product_observed(a, am, b, bm, mask, output).0
    }
    pub fn mapped_product_observed(
        &mut self,
        a: Ref,
        am: &[u32],
        b: Ref,
        bm: &[u32],
        mask: u64,
        output: &[u32],
    ) -> (Ref, Statistics) {
        // Validate even constant operands before permitting an early answer.
        for map in [am, bm, output] {
            Map::checked(self.dimensions, map);
        }
        assert_eq!(
            mask >> self.dimensions,
            0,
            "elimination coordinate outside presentation"
        );
        let mut product = Product {
            maps: [Map::compose(am, output), Map::compose(bm, output)],
            eliminate: axes(mask)
                .iter()
                .fold(0, |m, &v| m | (1 << output[v as usize])),
            memo: FxHashMap::default(),
            stats: Statistics::default(),
        };
        let views = [a, b].map(|r| Restricted::new(r).normalize(self));
        let result = product.visit(self, views);
        (result, product.stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(world: u64, map: &[u32]) -> u64 {
        map.iter()
            .enumerate()
            .fold(0, |w, (v, &target)| w | ((world >> target & 1) << v))
    }
    fn oracle<const K: u32>(
        arena: &Arena<K>,
        a: Ref,
        am: &[u32],
        b: Ref,
        bm: &[u32],
        mask: u64,
        output: &[u32],
        target: u64,
    ) -> bool {
        let before = source(target, output);
        let hidden = axes(mask);
        (0..1usize << hidden.len()).any(|i| {
            let w = (before & !mask) | scatter(i, &hidden);
            arena.evaluate(a, source(w, am)) && arena.evaluate(b, source(w, bm))
        })
    }

    fn exhaustive<const K: u32>() {
        for order in [vec![0, 1], vec![1, 0]] {
            let mut arena = Arena::<K>::new(2, &order);
            let ids: Vec<_> = (0..16).map(|n| arena.table(3, vec![n])).collect();
            for am in [[0, 1], [1, 0]] {
                for bm in [[0, 1], [1, 0]] {
                    for output in [[0, 1], [1, 0]] {
                        for mask in 0..4 {
                            for &a in &ids {
                                for &b in &ids {
                                    let data = words(4, |w| {
                                        oracle(&arena, a, &am, b, &bm, mask, &output, w as u64)
                                    });
                                    let actual =
                                        arena.mapped_product(a, &am, b, &bm, mask, &output);
                                    assert_eq!(actual, arena.table(3, data));
                                    let left = arena.rename(a, &am);
                                    let right = arena.rename(b, &bm);
                                    let product = arena.relprod(left, right, mask);
                                    assert_eq!(actual, arena.rename(product, &output));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn all_small_predicates_maps_and_eliminations_match_exact_ids() {
        exhaustive::<1>();
        exhaustive::<2>();
        exhaustive::<3>();
    }

    #[test]
    fn reordered_branches_retain_pending_pins() {
        let mut arena = Arena::<1>::new(3, &[0, 1, 2]);
        let mut fallbacks = 0;
        for pattern in [0x96, 0xd8, 0xe8, 0x69, 0x81] {
            let a = arena.table(7, vec![pattern]);
            let b = arena.variable(2);
            for mask in 0..8 {
                let data = words(8, |w| {
                    oracle(
                        &arena,
                        a,
                        &[2, 1, 0],
                        b,
                        &[0, 1, 2],
                        mask,
                        &[1, 2, 0],
                        w as u64,
                    )
                });
                let (actual, stats) =
                    arena.mapped_product_observed(a, &[2, 1, 0], b, &[0, 1, 2], mask, &[1, 2, 0]);
                assert_eq!(actual, arena.table(7, data));
                fallbacks += stats.branch_planes;
            }
        }
        assert!(
            fallbacks > 0,
            "exercise the raw-branch/small-residual boundary"
        );
    }

    fn scattered<const K: u32>() {
        let dimensions = 62;
        let order: Vec<_> = (0..dimensions).rev().collect();
        let mut arena = Arena::<K>::new(dimensions, &order);
        let coordinates = [0, 2, 5, 8, 13, 21, 29, 34, 40, 47, 55, 61];
        let mask = coordinates.iter().fold(0, |m, &v| m | 1 << v);
        let a = arena.table(mask, words(4096, |i| i.count_ones() % 3 == 1));
        let b = arena.table(mask, words(4096, |i| i.count_ones() % 4 < 2));
        let identity: Vec<_> = (0..dimensions).collect();
        let rotate: Vec<_> = (0..dimensions).map(|v| (v + 17) % dimensions).collect();
        let reverse: Vec<_> = (0..dimensions).rev().collect();
        let quantified = (1 << 2) | (1 << 40);
        let mut cases = 0;
        for bm in [&identity, &rotate] {
            for am in [&identity, &reverse] {
                for output in [&identity, &rotate] {
                    for (left, right) in [(a, b), (a ^ 1, b), (a, b ^ 1), (a ^ 1, b ^ 1)] {
                        let actual = arena.mapped_product(left, am, right, bm, quantified, output);
                        let ar = arena.rename(left, am);
                        let br = arena.rename(right, bm);
                        let p = arena.relprod(ar, br, quantified);
                        assert_eq!(actual, arena.rename(p, output));
                        for seed in 0..80u64 {
                            let world = seed.wrapping_mul(0x9e3779b97f4a7c15) & ((1u64 << 62) - 1);
                            assert_eq!(
                                arena.evaluate(actual, world),
                                oracle(&arena, left, am, right, bm, quantified, output, world)
                            );
                        }
                        cases += 1;
                    }
                }
            }
        }
        assert_eq!(cases, 32);
    }
    #[test]
    fn scattered_multiword_maps_and_complements_match_materialization() {
        scattered::<6>();
        scattered::<9>();
    }

    #[test]
    fn map_identity_is_part_of_the_product() {
        let mut arena = Arena::<1>::new(2, &[0, 1]);
        let x = arena.variable(0);
        assert_eq!(
            arena.mapped_product(x, &[0, 1], x ^ 1, &[0, 1], 0, &[0, 1]),
            0
        );
        let different = arena.mapped_product(x, &[0, 1], x ^ 1, &[1, 0], 0, &[0, 1]);
        assert_ne!(different, 0);
        assert_eq!(different, arena.table(3, vec![2]));
        for (am, bm, mask, output) in [
            (vec![0], vec![0, 1], 0, vec![0, 1]),
            (vec![0, 1], vec![0, 0], 0, vec![0, 1]),
            (vec![0, 1], vec![0, 1], 4, vec![0, 1]),
            (vec![0, 1], vec![0, 1], 0, vec![0, 2]),
        ] {
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || arena.mapped_product(0, &am, 1, &bm, mask, &output)
                ))
                .is_err()
            );
        }
    }
}
