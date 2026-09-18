//! Exact mapped conjunction/projection without resident renamed operands.
//! Maps and elimination are fixed per invocation; memo entries never escape it.
use super::{Arena, Ref, View, axes, scatter, word_kernels, words};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reuse {
    Off,
    Bounded(usize),
}
impl Reuse {
    pub fn from_env() -> Self {
        match std::env::var("EVENT_LAB_VIEW_REUSE").as_deref() {
            Ok("off") | Err(_) => Self::Off,
            Ok("bounded") => Self::Bounded(256),
            _ => panic!("unknown mapped-product plane reuse"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Normalization {
    Deferred,
    Source,
}
impl Normalization {
    pub fn from_env() -> Self {
        match std::env::var("EVENT_LAB_VIEW_NORMALIZE").as_deref() {
            Ok("deferred") | Err(_) => Self::Deferred,
            Ok("source") => Self::Source,
            _ => panic!("unknown mapped-product source normalization"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalKernel {
    Assignments,
    Words,
}
impl LocalKernel {
    pub fn from_env() -> Self {
        match std::env::var("EVENT_LAB_VIEW_KERNEL").as_deref() {
            Ok("assignments") | Err(_) => Self::Assignments,
            Ok("words") => Self::Words,
            _ => panic!("unknown mapped-product local kernel"),
        }
    }
}

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
    pub branch_word_merges: usize,
    pub plane_cache_hits: usize,
    pub plane_cache_misses: usize,
    pub plane_cache_evictions: usize,
    /// Held cache payload and slots, excluding incoming alignment workspace.
    pub plane_cache_bytes_peak: usize,
    pub canonicalized_views: usize,
    pub canonical_cofactor_steps: usize,
    pub removed_coordinates: usize,
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
    fn canonicalize<const K: u32>(self, arena: &mut Arena<K>, stats: &mut Statistics) -> Self {
        if self.pinned == 0 {
            return self;
        }
        stats.canonicalized_views += 1;
        let before = arena.variables(self.root) & !self.pinned;
        let mut root = self.root;
        for v in axes(self.pinned) {
            if arena.variables(root) >> v & 1 != 0 {
                root = arena.cofactor(root, v, self.values >> v & 1 != 0);
                stats.canonical_cofactor_steps += 1;
            }
        }
        let after = arena.variables(root);
        assert_eq!(after & !before, 0, "cofactoring cannot add dependence");
        stats.removed_coordinates += (before & !after).count_ones() as usize;
        Self::new(root)
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PlaneKey {
    view: Restricted,
    target: u64,
}
enum SavedPlane {
    Constant(bool),
    // Retain a stable node ID, never an arena/slab address. The payload is
    // borrowed anew only during the immutable read phase of each local cube.
    Source(Ref),
    Words { data: Box<[u64]>, flip: u64 },
}
impl SavedPlane {
    fn payload_bytes(&self) -> usize {
        match self {
            Self::Words { data, .. } => data.len() * 8,
            _ => 0,
        }
    }
}
struct PlaneEntry {
    key: PlaneKey,
    value: SavedPlane,
}
#[derive(Default)]
struct PlaneCache {
    slots: Box<[Option<PlaneEntry>]>,
    payload_bytes: usize,
}
impl PlaneCache {
    fn index(&self, key: &PlaneKey) -> usize {
        let mut hash = rustc_hash::FxHasher::default();
        key.hash(&mut hash);
        hash.finish() as usize & (self.slots.len() - 1)
    }
    fn bytes(&self) -> usize {
        self.slots.len() * std::mem::size_of::<Option<PlaneEntry>>() + self.payload_bytes
    }
    fn prepare<const K: u32>(
        &mut self,
        arena: &Arena<K>,
        view: Restricted,
        map: &Map,
        target: u64,
        kernel: LocalKernel,
        capacity: usize,
        stats: &mut Statistics,
    ) {
        if view.root < 2 {
            return;
        }
        if self.slots.is_empty() {
            assert!(capacity.is_power_of_two());
            self.slots = std::iter::repeat_with(|| None)
                .take(capacity)
                .collect::<Vec<_>>()
                .into_boxed_slice();
        }
        let key = PlaneKey { view, target };
        let index = self.index(&key);
        if self.slots[index]
            .as_ref()
            .is_some_and(|entry| entry.key == key)
        {
            stats.plane_cache_hits += 1;
            return;
        }
        stats.plane_cache_misses += 1;
        if let Some(old) = self.slots[index].take() {
            stats.plane_cache_evictions += 1;
            self.payload_bytes -= old.value.payload_bytes();
        }
        let value = match plane(arena, view, map, target, kernel, stats) {
            Plane::Constant(value) => SavedPlane::Constant(value),
            Plane::Table {
                data: Cow::Borrowed(_),
                ..
            } => SavedPlane::Source(view.root),
            Plane::Table {
                data: Cow::Owned(data),
                flip,
            } => SavedPlane::Words {
                data: data.into_boxed_slice(),
                flip,
            },
        };
        self.payload_bytes += value.payload_bytes();
        self.slots[index] = Some(PlaneEntry { key, value });
    }
    fn read<'a, const K: u32>(
        &'a self,
        arena: &'a Arena<K>,
        view: Restricted,
        target: u64,
    ) -> Plane<'a> {
        if view.root < 2 {
            return Plane::Constant(view.root != 0);
        }
        let key = PlaneKey { view, target };
        let entry = self.slots[self.index(&key)]
            .as_ref()
            .expect("prepared plane");
        assert!(entry.key == key, "each operand has its own fixed-map cache");
        match &entry.value {
            SavedPlane::Constant(value) => Plane::Constant(*value),
            SavedPlane::Source(root) => match arena.view(*root) {
                View::Table {
                    words,
                    complemented,
                    ..
                } => Plane::Table {
                    data: Cow::Borrowed(words),
                    flip: if complemented { !0 } else { 0 },
                },
                _ => unreachable!("source aliases refer to immutable table nodes"),
            },
            SavedPlane::Words { data, flip } => Plane::Table {
                data: Cow::Borrowed(data),
                flip: *flip,
            },
        }
    }
}

fn plane<'a, const K: u32>(
    arena: &'a Arena<K>,
    view: Restricted,
    map: &Map,
    target: u64,
    kernel: LocalKernel,
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
        View::Branch {
            variable,
            low,
            high,
        } => {
            // A different target order can pin a descendant before its root.
            // Small residual dependence does not make this raw node a table.
            stats.branch_planes += 1;
            if kernel == LocalKernel::Words {
                // Pending pins can reach below this source branch. Normalize
                // each child before reading it; no canonical node is created.
                assert_eq!(view.pinned >> variable & 1, 0);
                let lo = Restricted { root: low, ..view }.normalize(arena);
                let hi = Restricted { root: high, ..view }.normalize(arena);
                let a = plane(arena, lo, map, target, kernel, stats);
                let b = plane(arena, hi, map, target, kernel, stats);
                let mapped = map.destinations[variable as usize];
                assert_ne!(target >> mapped & 1, 0);
                let position = (target & ((1 << mapped) - 1)).count_ones();
                let count = (1usize << target.count_ones()).div_ceil(64);
                let data = (0..count)
                    .map(|i| {
                        let selector = word_kernels::literal_word(position, i);
                        (a.word(i) & !selector) | (b.word(i) & selector)
                    })
                    .collect();
                stats.branch_word_merges += count;
                return Plane::Table {
                    data: Cow::Owned(data),
                    flip: 0,
                };
            }
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
    kernel: LocalKernel,
    reuse: Reuse,
    normalization: Normalization,
    // One invocation fixes both maps, arena, support, order and local kernel.
    // Separate operand caches also prevent a second preparation from evicting
    // the first operand before both are read. No cached plane escapes the call.
    planes: [PlaneCache; 2],
}
impl Product {
    fn visit<const K: u32>(&mut self, arena: &mut Arena<K>, mut views: [Restricted; 2]) -> Ref {
        if self.normalization == Normalization::Source {
            for view in &mut views {
                *view = view.canonicalize(arena, &mut self.stats);
            }
        }
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
            let mut data: Vec<u64> = if let Reuse::Bounded(capacity) = self.reuse {
                for i in 0..2 {
                    self.planes[i].prepare(
                        arena,
                        views[i],
                        &self.maps[i],
                        variables,
                        self.kernel,
                        capacity,
                        &mut self.stats,
                    );
                    self.stats.plane_cache_bytes_peak = self
                        .stats
                        .plane_cache_bytes_peak
                        .max(self.planes.iter().map(PlaneCache::bytes).sum());
                }
                let a = self.planes[0].read(arena, views[0], variables);
                let b = self.planes[1].read(arena, views[1], variables);
                (0..(1usize << variables.count_ones()).div_ceil(64))
                    .map(|i| a.word(i) & b.word(i))
                    .collect()
            } else {
                let a = plane(
                    arena,
                    views[0],
                    &self.maps[0],
                    variables,
                    self.kernel,
                    &mut self.stats,
                );
                let b = plane(
                    arena,
                    views[1],
                    &self.maps[1],
                    variables,
                    self.kernel,
                    &mut self.stats,
                );
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
            kernel: self.view_kernel,
            reuse: self.view_reuse,
            normalization: self.view_normalization,
            planes: std::array::from_fn(|_| PlaneCache::default()),
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
        arena.view_normalization = Normalization::Deferred;
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
                        let mut control = arena.clone();
                        arena = arena.clone(); // Match clone capacities before comparing bytes.
                        control.view_kernel = LocalKernel::Assignments;
                        control.view_reuse = Reuse::Off;
                        let (expected, scalar_stats) = control
                            .mapped_product_observed(left, am, right, bm, quantified, output);
                        arena.view_kernel = LocalKernel::Words;
                        let (actual, word_stats) =
                            arena.mapped_product_observed(left, am, right, bm, quantified, output);
                        assert_eq!(
                            actual, expected,
                            "local kernel must preserve canonical insertion order"
                        );
                        assert_eq!(arena.nodes(), control.nodes());
                        assert_eq!(arena.bytes(), control.bytes());
                        assert_eq!(arena.cache_bytes(), control.cache_bytes());
                        assert_eq!(word_stats.subproblems, scalar_stats.subproblems);
                        assert_eq!(word_stats.local_cubes, scalar_stats.local_cubes);
                        assert_eq!(word_stats.branch_assignments, 0);
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
    fn word_shannon_planes_match_assignments_with_pending_pins() {
        for order in [vec![0, 1, 2], vec![2, 0, 1]] {
            let mut arena = Arena::<1>::new(3, &order);
            let mut merges = 0;
            for pattern in 0..256 {
                let root = arena.table(7, vec![pattern]);
                for pinned in 0..8u64 {
                    for values in 0..8u64 {
                        if values & !pinned != 0 {
                            continue;
                        }
                        let view = Restricted {
                            root,
                            pinned,
                            values,
                        }
                        .normalize(&arena);
                        for destinations in [[0, 1, 2], [2, 1, 0], [1, 2, 0]] {
                            let map = Map::compose(&destinations, &[0, 1, 2]);
                            let target = map.mask(arena.variables(view.root) & !view.pinned);
                            let mut stats = Statistics::default();
                            let a = plane(
                                &arena,
                                view,
                                &map,
                                target,
                                LocalKernel::Assignments,
                                &mut stats,
                            );
                            let b =
                                plane(&arena, view, &map, target, LocalKernel::Words, &mut stats);
                            let cells = 1usize << target.count_ones();
                            let mask = (1u64 << cells) - 1;
                            assert_eq!(a.word(0) & mask, b.word(0) & mask);
                            merges += stats.branch_word_merges;
                        }
                    }
                }
            }
            assert!(merges > 0);
        }
    }

    #[test]
    fn plane_cache_collisions_maps_pins_complements_and_arena_growth() {
        for destinations in [
            (0..8).collect::<Vec<_>>(),
            (0..8).rev().collect(),
            (0..8).map(|v| (v + 3) % 8).collect(),
        ] {
            let mut arena = Arena::<3>::new(8, &(0..8).rev().collect::<Vec<_>>());
            let root = arena.table(37, vec![0x96]);
            let map = Map::compose(&destinations, &(0..8).collect::<Vec<_>>());
            // One slot forces replacement on every distinct nonconstant key.
            let mut cache = PlaneCache::default();
            let mut stats = Statistics::default();
            let mut grew = false;
            for flip in [0, 1] {
                for pinned in [0, 1, 4, 32, 5, 33, 36] {
                    for values in 0..64 {
                        if values & !pinned != 0 {
                            continue;
                        }
                        let view = Restricted {
                            root: root ^ flip,
                            pinned,
                            values,
                        }
                        .normalize(&arena);
                        let live = map.mask(arena.variables(view.root) & !view.pinned);
                        for extra in [false, true] {
                            let target = if extra && live.count_ones() < 3 {
                                live | (1
                                    << ((!(live | map.mask(view.pinned)) & 255).trailing_zeros()))
                            } else {
                                live
                            };
                            cache.prepare(
                                &arena,
                                view,
                                &map,
                                target,
                                LocalKernel::Words,
                                1,
                                &mut stats,
                            );
                            cache.prepare(
                                &arena,
                                view,
                                &map,
                                target,
                                LocalKernel::Words,
                                1,
                                &mut stats,
                            );
                            if !grew {
                                let before = arena.nodes();
                                for n in 0..256 {
                                    arena.table(7, vec![n]);
                                }
                                assert!(arena.nodes() > before);
                                grew = true;
                            }
                            let expected = plane(
                                &arena,
                                view,
                                &map,
                                target,
                                LocalKernel::Assignments,
                                &mut Statistics::default(),
                            );
                            let actual = cache.read(&arena, view, target);
                            let valid = (1 << (1 << target.count_ones())) - 1;
                            assert_eq!(actual.word(0) & valid, expected.word(0) & valid);
                            assert!(cache.bytes() <= std::mem::size_of::<Option<PlaneEntry>>() + 8);
                        }
                    }
                }
            }
            assert!(stats.plane_cache_hits > 0 && stats.plane_cache_evictions > 0);
        }
    }

    #[test]
    fn bounded_and_single_slot_caches_preserve_exact_product_construction() {
        let mut input = Arena::<3>::new(8, &(0..8).rev().collect::<Vec<_>>());
        let a = input.table(255, words(256, |i| i.count_ones() % 3 == 1));
        let b = input.table(255, words(256, |i| i.count_ones() % 4 < 2));
        let id: Vec<_> = (0..8).collect();
        let reverse: Vec<_> = (0..8).rev().collect();
        let rotate: Vec<_> = (0..8).map(|v| (v + 3) % 8).collect();
        for normalization in [Normalization::Deferred, Normalization::Source] {
            for kernel in [LocalKernel::Assignments, LocalKernel::Words] {
                for output in [&id, &rotate] {
                    for mask in [0, 85, 255] {
                        let mut control = input.clone();
                        control.view_kernel = kernel;
                        control.view_normalization = normalization;
                        control.view_reuse = Reuse::Off;
                        let (expected, base) =
                            control.mapped_product_observed(a, &id, b ^ 1, &reverse, mask, output);
                        for capacity in [1, 256] {
                            let mut cached = input.clone();
                            cached.view_kernel = kernel;
                            cached.view_normalization = normalization;
                            cached.view_reuse = Reuse::Bounded(capacity);
                            let (actual, work) = cached.mapped_product_observed(
                                a,
                                &id,
                                b ^ 1,
                                &reverse,
                                mask,
                                output,
                            );
                            assert_eq!(actual, expected);
                            assert_eq!(cached.nodes(), control.nodes());
                            assert_eq!(cached.bytes(), control.bytes());
                            assert_eq!(cached.cache_bytes(), control.cache_bytes());
                            assert_eq!(work.subproblems, base.subproblems);
                            assert_eq!(work.local_cubes, base.local_cubes);
                            assert!(
                                work.plane_cache_bytes_peak
                                    <= 2 * capacity
                                        * (std::mem::size_of::<Option<PlaneEntry>>() + 8)
                            );
                        }
                        for alternate in [Normalization::Deferred, Normalization::Source] {
                            control.view_normalization = alternate;
                            assert_eq!(
                                control.mapped_product(a, &id, b ^ 1, &reverse, mask, output),
                                expected
                            );
                        }
                        for world in 0..256 {
                            assert_eq!(
                                control.evaluate(expected, world),
                                oracle(&control, a, &id, b ^ 1, &reverse, mask, output, world)
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn canonical_source_cofactor_removes_false_dependence() {
        let mut arena = Arena::<1>::new(3, &[2, 1, 0]);
        let root = arena.table(7, vec![0xd8]); // if x then y else z
        let view = Restricted {
            root,
            pinned: 1,
            values: 1,
        }
        .normalize(&arena);
        assert_eq!(arena.variables(view.root) & !view.pinned, 6);
        let mut stats = Statistics::default();
        let exact = view.canonicalize(&mut arena, &mut stats);
        assert_eq!(exact.root, arena.variable(1));
        assert_eq!(exact.pinned, 0);
        assert_eq!(stats.canonical_cofactor_steps, 1);
        assert_eq!(stats.removed_coordinates, 1);
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
