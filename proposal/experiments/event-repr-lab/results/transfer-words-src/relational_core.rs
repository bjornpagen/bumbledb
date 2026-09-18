//! Full-product finite relation lane. Includes real coordinate movement.
//! Permutations are local bijections, not the proposed cross-space map registry.
use super::carrier::*;
use rustc_hash::FxHashMap;

#[derive(Clone)]
pub(super) struct Algebra<C: Carrier> {
    pub(super) base: Memo<C>,
    maps: [Permutation; 4],
    permutations: FxHashMap<(usize, Id), Id>,
    products: FxHashMap<(Id, Id, u64), Id>,
    pub(super) width: u32,
    identity: Id,
}
impl<C: Carrier> Algebra<C> {
    pub(super) fn new(mut c: C, width: u32, memo: bool) -> Self {
        let n = 1usize << (3 * width);
        let mask = (1usize << width) - 1;
        let identity = c.import(&bits(n, |w| w & mask == (w >> width) & mask));
        let maps = [[1, 0, 2], [1, 2, 0], [0, 2, 1], [2, 0, 1]].map(|faces| {
            Permutation::new(
                (0..3 * width)
                    .map(|k| faces[(k / width) as usize] * width + k % width)
                    .collect(),
            )
            .unwrap()
        });
        // This lane's full Cartesian support is invariant under every map.
        for map in &maps {
            let full = c.full();
            assert_eq!(c.permute(full, map), full);
        }
        Self {
            base: Memo::new(c, memo),
            maps,
            permutations: FxHashMap::default(),
            products: FxHashMap::default(),
            width,
            identity,
        }
    }
    pub(super) fn rename(&mut self, event: Id, map: usize) -> Id {
        if self.base.enabled {
            if let Some(&out) = self.permutations.get(&(map, event)) {
                return out;
            }
        }
        let out = self.base.inner.permute(event, &self.maps[map]);
        if self.base.enabled {
            self.permutations.insert((map, event), out);
        }
        out
    }
    pub(super) fn product(&mut self, a: Id, b: Id, mask: u64) -> Id {
        let key = (a.min(b), a.max(b), mask);
        if self.base.enabled {
            if let Some(&out) = self.products.get(&key) {
                return out;
            }
        }
        let out = self.base.inner.relprod(a, b, mask);
        if self.base.enabled {
            self.products.insert(key, out);
        }
        out
    }
    pub(super) fn compose(&mut self, r: Id, q: Id) -> Id {
        let q_yz = self.rename(q, 1);
        let xz = self.product(r, q_yz, ((1u64 << self.width) - 1) << self.width);
        self.rename(xz, 2)
    }
    // Largest Q for which R ; Q is contained in T, returned on the XY face.
    pub(super) fn residual(&mut self, r: Id, t: Id) -> Id {
        let t_xz = self.rename(t, 2);
        let neg = self.base.inner.not(t_xz);
        let bad_yz = self.product(r, neg, (1u64 << self.width) - 1);
        let safe_yz = self.base.inner.not(bad_yz);
        self.rename(safe_yz, 3)
    }
    pub(super) fn closure(&mut self, r: Id) -> (Id, usize) {
        let mut reach = self.base.op(14, r, self.identity);
        for iteration in 1..=self.width as usize + 2 {
            let step = self.compose(reach, reach);
            let next = self.base.op(14, reach, step);
            if reach == next {
                return (reach, iteration);
            }
            reach = next;
        }
        panic!("finite closure failed to stabilize");
    }
    pub(super) fn bytes(&self) -> usize {
        self.base.bytes()
            + self.products.capacity() * 40
            + self.permutations.capacity() * 32
            + self
                .maps
                .iter()
                .map(|p| p.destinations.capacity() * 4)
                .sum::<usize>()
    }
}

pub(super) fn compose_matrix(r: &[bool], q: &[bool], size: usize) -> Vec<bool> {
    (0..size * size)
        .map(|i| {
            let (x, z) = (i / size, i % size);
            (0..size).any(|y| r[x * size + y] && q[y * size + z])
        })
        .collect()
}
pub(super) fn residual_matrix(r: &[bool], t: &[bool], size: usize) -> Vec<bool> {
    (0..size * size)
        .map(|i| {
            let (y, z) = (i / size, i % size);
            (0..size).all(|x| !r[x * size + y] || t[x * size + z])
        })
        .collect()
}
pub(super) fn lifted(matrix: &[bool], width: u32) -> Vec<u64> {
    let size = 1usize << width;
    bits(1usize << (3 * width), |w| {
        matrix[(w & (size - 1)) * size + ((w >> width) & (size - 1))]
    })
}
pub fn verify<C: Carrier>() {
    let mut c = Algebra::new(C::new(&[255], 8), 1, true);
    let matrices: Vec<Vec<bool>> = (0..16)
        .map(|r| (0..4).map(|i| r >> i & 1 != 0).collect())
        .collect();
    let ids: Vec<_> = matrices
        .iter()
        .map(|r| c.base.inner.import(&lifted(r, 1)))
        .collect();
    for (a, r) in matrices.iter().enumerate() {
        let transposed: Vec<_> = (0..4).map(|i| r[(i % 2) * 2 + i / 2]).collect();
        let converse = c.rename(ids[a], 0);
        assert_eq!(c.base.inner.export(converse), lifted(&transposed, 1));
        assert_eq!(c.rename(converse, 0), ids[a]);
        let (closure, _) = c.closure(ids[a]);
        let expected = vec![true, r[1], r[2], true];
        assert_eq!(c.base.inner.export(closure), lifted(&expected, 1));
        for (b, q) in matrices.iter().enumerate() {
            let composed = c.compose(ids[a], ids[b]);
            assert_eq!(
                c.base.inner.export(composed),
                lifted(&compose_matrix(r, q, 2), 1)
            );
            let residual = c.residual(ids[a], ids[b]);
            assert_eq!(
                c.base.inner.export(residual),
                lifted(&residual_matrix(r, q, 2), 1)
            );
            for &test in &ids {
                let rq = c.compose(ids[a], test);
                let left = c.base.op(4, rq, ids[b]) == c.base.inner.empty();
                let right = c.base.op(4, test, residual) == c.base.inner.empty();
                assert_eq!(left, right, "{} residual adjunction", C::NAME);
            }
        }
    }
}

pub(super) fn verify_layout<C: Carrier>(layout: &str) {
    let width = 4;
    let n = 1usize << (3 * width);
    let size = 1usize << width;
    let mut c = Algebra::new(
        C::with_order(&bits(n, |_| true), n, face_order(12, 3, layout)),
        width,
        true,
    );
    let matrices: Vec<Vec<bool>> = (0..3)
        .map(|seed| {
            (0..size * size)
                .map(|i| {
                    let (x, y) = (i / size, i % size);
                    (x / 4 == y / 4 && mix((i + seed * 331) as u64) % 3 != 0)
                        || (x == y && seed == 0)
                })
                .collect()
        })
        .collect();
    let ids: Vec<_> = matrices
        .iter()
        .map(|m| c.base.inner.import(&lifted(m, width)))
        .collect();
    for (i, r) in matrices.iter().enumerate() {
        let converse = c.rename(ids[i], 0);
        let expected: Vec<_> = (0..size * size)
            .map(|i| r[(i % size) * size + i / size])
            .collect();
        assert_eq!(
            c.base.inner.export(converse),
            lifted(&expected, width),
            "{} {layout} converse",
            C::NAME
        );
        assert_eq!(c.rename(converse, 0), ids[i]);
        let (closure, _) = c.closure(ids[i]);
        let mut expected = r.clone();
        for x in 0..size {
            expected[x * size + x] = true;
        }
        for y in 0..size {
            for x in 0..size {
                for z in 0..size {
                    expected[x * size + z] |= expected[x * size + y] && expected[y * size + z];
                }
            }
        }
        assert_eq!(
            c.base.inner.export(closure),
            lifted(&expected, width),
            "{} {layout} closure",
            C::NAME
        );
        for (j, q) in matrices.iter().enumerate() {
            let out = c.compose(ids[i], ids[j]);
            assert_eq!(
                c.base.inner.export(out),
                lifted(&compose_matrix(r, q, size), width),
                "{} {layout} composition",
                C::NAME
            );
            let out = c.residual(ids[i], ids[j]);
            assert_eq!(
                c.base.inner.export(out),
                lifted(&residual_matrix(r, q, size), width),
                "{} {layout} residual",
                C::NAME
            );
        }
    }
}
