//! Binary relations on checked equal-domain products, retaining environments.
//! Permutations are local bijections, not the proposed cross-space map registry.
use super::carrier::*;
use rustc_hash::FxHashMap;

#[derive(Clone)]
pub(super) struct Algebra<C: RegionOps> {
    pub(super) base: Memo<C>,
    maps: [Permutation; 5],
    permutations: FxHashMap<(usize, Id), Id>,
    products: FxHashMap<(Id, Id, u64), Id>,
    view_products: FxHashMap<((Id, usize), (Id, usize), u64, usize), Id>,
    strategy: &'static str,
    support_strategy: &'static str,
    pub(super) width: u32,
    identity: Id,
}
impl<C: RegionOps> Algebra<C> {
    pub(super) fn new(c: C, width: u32, memo: bool) -> Self {
        Self::try_new(c, width, memo, &identity_mode()).unwrap_or_else(|why| panic!("{why}"))
    }
    pub(super) fn try_new(c: C, width: u32, memo: bool, mode: &str) -> Result<Self, &'static str> {
        if width.checked_mul(3) != Some(c.dimensions()) || c.dimensions() >= 63 {
            return Err("relation program requires three equal binary faces");
        }
        // Permutation invariance alone is insufficient: X=Y=Z is symmetric
        // but not the full product. This finite-binary lab has exact u64 counts.
        if c.count(c.full()) != 1u64 << c.dimensions() {
            return Err("relation program requires unconstrained full Cartesian support");
        }
        Self::finish(c, width, memo, mode, "gated")
    }
    pub(super) fn try_new_legal(
        mut c: C,
        domains: &super::legal::Domains,
        memo: bool,
        mode: &str,
        gates: &'static str,
    ) -> Result<Self, &'static str> {
        domains.verify(&mut c)?;
        Self::finish(c, domains.width(), memo, mode, gates)
    }
    fn finish(
        mut c: C,
        width: u32,
        memo: bool,
        mode: &str,
        gates: &'static str,
    ) -> Result<Self, &'static str> {
        if !matches!(gates, "gated" | "certified") {
            return Err("unknown support strategy");
        }
        let maps = [[1, 0, 2], [1, 2, 0], [0, 2, 1], [2, 0, 1], [0, 1, 2]].map(|faces| {
            Permutation::new(
                (0..c.dimensions())
                    .map(|k| {
                        if k < 3 * width {
                            faces[(k / width) as usize] * width + k % width
                        } else {
                            k
                        }
                    })
                    .collect(),
            )
            .unwrap()
        });
        // Full binary products and verified equal legal-domain products are
        // invariant under these face maps; environment coordinates stay fixed.
        for map in &maps {
            let full = c.full();
            assert_eq!(c.permute(full, map), full);
        }
        let pairs: Vec<_> = (0..width).map(|i| (i, width + i)).collect();
        let identity = match mode {
            "native" => c.diagonal(&pairs),
            "symbolic" => symbolic_diagonal(&mut c, &pairs),
            "table" => table_diagonal(&mut c, &pairs),
            _ => return Err("unknown diagonal constructor"),
        }?;
        Ok(Self {
            base: Memo::new(c, memo),
            maps,
            permutations: FxHashMap::default(),
            products: FxHashMap::default(),
            view_products: FxHashMap::default(),
            strategy: if C::VIEW_PRODUCT {
                product_mode()
            } else {
                "materialized"
            },
            support_strategy: gates,
            width,
            identity,
        })
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
    pub(super) fn product_strategy(&self) -> &'static str {
        self.strategy
    }
    pub(super) fn support_strategy(&self) -> &'static str {
        if C::VIEW_PRODUCT && self.strategy != "materialized" {
            self.support_strategy
        } else {
            "materialized"
        }
    }
    fn execute_product(&mut self, a: Id, am: usize, b: Id, bm: usize, mask: u64, out: usize) -> Id {
        let c = &mut self.base.inner;
        let result = if self.support_strategy == "certified" {
            c.preserving_product(a, &self.maps[am], b, &self.maps[bm], mask, &self.maps[out])
        } else {
            c.view_product(a, &self.maps[am], b, &self.maps[bm], mask, &self.maps[out])
        };
        result.expect("checked product maps must have the selected capability")
    }
    pub(super) fn product(&mut self, a: Id, b: Id, mask: u64) -> Id {
        let key = (a.min(b), a.max(b), mask);
        if self.base.enabled {
            if let Some(&out) = self.products.get(&key) {
                return out;
            }
        }
        let out = if self.strategy != "materialized" {
            self.execute_product(a, 4, b, 4, mask, 4)
        } else {
            self.base.inner.relprod(a, b, mask)
        };
        if self.base.enabled {
            self.products.insert(key, out);
        }
        out
    }
    pub(super) fn compose(&mut self, r: Id, q: Id) -> Id {
        if self.strategy != "materialized" {
            return self.mapped_product(r, 4, q, 1, ((1u64 << self.width) - 1) << self.width, 2);
        }
        let q_yz = self.rename(q, 1);
        let xz = self.product(r, q_yz, ((1u64 << self.width) - 1) << self.width);
        self.rename(xz, 2)
    }
    // Largest Q for which R ; Q is contained in T, returned on the XY face.
    pub(super) fn residual(&mut self, r: Id, t: Id) -> Id {
        if self.strategy != "materialized" {
            let neg = self.base.inner.not(t);
            let bad = self.mapped_product(r, 4, neg, 2, (1u64 << self.width) - 1, 3);
            return self.base.inner.not(bad);
        }
        let t_xz = self.rename(t, 2);
        let neg = self.base.inner.not(t_xz);
        let bad_yz = self.product(r, neg, (1u64 << self.width) - 1);
        let safe_yz = self.base.inner.not(bad_yz);
        self.rename(safe_yz, 3)
    }
    fn mapped_product(&mut self, a: Id, am: usize, b: Id, bm: usize, mask: u64, out: usize) -> Id {
        // Include each root's map and the output/elimination context. The map
        // table, owner and support are immutable within this Algebra instance.
        let mut operands = [(a, am), (b, bm)];
        operands.sort_unstable();
        let key = (operands[0], operands[1], mask, out);
        if self.base.enabled {
            if let Some(&value) = self.view_products.get(&key) {
                return value;
            }
        }
        // Input views and output fusion are independent choices. Delaying
        // output renaming preserves the working order for elimination.
        let output_map = if self.strategy == "views-inputs" {
            4
        } else {
            out
        };
        let mut value = self.execute_product(a, am, b, bm, mask, output_map);
        if self.strategy == "views-inputs" && out != 4 {
            value = self.rename(value, out);
        }
        if self.base.enabled {
            self.view_products.insert(key, value);
        }
        value
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
            + self.view_products.capacity()
                * (std::mem::size_of::<(((Id, usize), (Id, usize), u64, usize), Id)>() + 1)
            + self.maps.iter().map(|p| p.bytes()).sum::<usize>()
    }
}

pub(super) fn product_mode() -> &'static str {
    match std::env::var("EVENT_LAB_PRODUCT").as_deref() {
        Ok("materialized") | Err(_) => "materialized",
        Ok("views") => "views",
        Ok("views-inputs") => "views-inputs",
        _ => panic!("unknown relation product mode"),
    }
}

pub(super) fn identity_mode() -> String {
    std::env::var("EVENT_LAB_IDENTITY").unwrap_or_else(|_| "native".into())
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
    verify_operations(C::new(&[255], 8));
    let mut borrowed = C::new(&[255], 8);
    verify_operations(&mut borrowed);
}
fn verify_operations<C: RegionOps>(carrier: C) {
    let mut c = Algebra::new(carrier, 1, true);
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
