//! Full-product finite relation lane. Includes real coordinate movement.
//! Permutations are local bijections, not the proposed cross-space map registry.
use super::*;
use rustc_hash::FxHashMap;

#[derive(Clone)]
struct Algebra<C: Carrier> {
    base: Memo<C>,
    maps: [Permutation; 4],
    permutations: FxHashMap<(usize, Id), Id>,
    products: FxHashMap<(Id, Id, u64), Id>,
    width: u32,
    identity: Id,
}
impl<C: Carrier> Algebra<C> {
    fn new(mut c: C, width: u32, memo: bool) -> Self {
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
    fn rename(&mut self, event: Id, map: usize) -> Id {
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
    fn product(&mut self, a: Id, b: Id, mask: u64) -> Id {
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
    fn compose(&mut self, r: Id, q: Id) -> Id {
        let q_yz = self.rename(q, 1);
        let xz = self.product(r, q_yz, ((1u64 << self.width) - 1) << self.width);
        self.rename(xz, 2)
    }
    // Largest Q for which R ; Q is contained in T, returned on the XY face.
    fn residual(&mut self, r: Id, t: Id) -> Id {
        let t_xz = self.rename(t, 2);
        let neg = self.base.inner.not(t_xz);
        let bad_yz = self.product(r, neg, (1u64 << self.width) - 1);
        let safe_yz = self.base.inner.not(bad_yz);
        self.rename(safe_yz, 3)
    }
    fn closure(&mut self, r: Id) -> (Id, usize) {
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
    fn bytes(&self) -> usize {
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

fn compose_matrix(r: &[bool], q: &[bool], size: usize) -> Vec<bool> {
    (0..size * size)
        .map(|i| {
            let (x, z) = (i / size, i % size);
            (0..size).any(|y| r[x * size + y] && q[y * size + z])
        })
        .collect()
}
fn residual_matrix(r: &[bool], t: &[bool], size: usize) -> Vec<bool> {
    (0..size * size)
        .map(|i| {
            let (y, z) = (i / size, i % size);
            (0..size).all(|x| !r[x * size + y] || t[x * size + z])
        })
        .collect()
}
fn lifted(matrix: &[bool], width: u32) -> Vec<u64> {
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

fn query<C: Carrier>(
    engine: &mut native::Native,
    c: &mut Algebra<C>,
    groups: usize,
) -> (Vec<Id>, u64, usize, usize) {
    let empty = c.base.inner.empty();
    let full = c.base.inner.full();
    let mut relations = vec![empty; groups];
    let mut allowed = vec![full; groups];
    let mut goals = vec![empty; groups];
    let mut rows = 0;
    engine.run(|[g, scope, r, t, goal]| {
        assert_eq!(scope, 1);
        let i = g as usize;
        relations[i] = c.base.op(14, relations[i], r);
        allowed[i] = c.base.op(8, allowed[i], t);
        goals[i] = c.base.op(14, goals[i], goal);
        rows += 1;
    });
    let mut out = Vec::with_capacity(5 * groups);
    let mut iterations = 0;
    let mask = ((1u64 << c.width) - 1) << c.width;
    for i in 0..groups {
        let (reach, steps) = c.closure(relations[i]);
        iterations += steps;
        let converse = c.rename(reach, 0);
        let residual = c.residual(relations[i], allowed[i]);
        let may_reach = c.product(reach, goals[i], mask);
        let neg = c.base.inner.not(goals[i]);
        let bad = c.product(relations[i], neg, mask);
        let enabled = c.product(relations[i], full, mask);
        let must_next = c.base.op(4, enabled, bad);
        out.extend([reach, converse, residual, may_reach, must_next]);
    }
    let checksum = output_checksum(&c.base.inner, &out);
    (out, checksum, rows, iterations)
}

pub fn bench_relations<C: Carrier>(nbits: u32) {
    let width = nbits / 3;
    let n = 1usize << nbits;
    let size = 1usize << width;
    let block = size.min(8);
    let groups = 16;
    let bank_size = 8;
    let bank: Vec<_> = (0..3)
        .flat_map(|family| {
            (0..bank_size).map(move |i| {
                bits(n, |w| {
                    let (x, y) = (w & (size - 1), (w >> width) & (size - 1));
                    match family {
                        0 => {
                            x % 7 != i % 7
                                && x / block == y / block
                                && (y % block == (x + i) % block
                                    || y % block == (x + i + 1) % block)
                        }
                        1 => {
                            x / block == y / block || ((x + i) % size < size / 4 && y % 5 != i % 5)
                        }
                        _ => (y + i) % size < size / 3,
                    }
                })
            })
        })
        .collect();
    let mut raw = native::data("clover", groups, 2, bank_size);
    for (family, rows) in raw.iter_mut().enumerate() {
        for row in rows {
            row[3] += (family * bank_size) as u64;
        }
    }
    // Oracle operates on explicit state-pair matrices with Floyd-Warshall closure.
    let mut rs = vec![vec![false; size * size]; groups];
    let mut ts = vec![vec![true; size * size]; groups];
    let mut goals = vec![vec![false; size]; groups];
    let mut expected_rows = 0;
    for a in &raw[0] {
        for b in &raw[1] {
            for d in &raw[2] {
                if a[0] != b[0] || a[0] != d[0] {
                    continue;
                }
                let g = a[0] as usize;
                expected_rows += 1;
                for x in 0..size {
                    for y in 0..size {
                        let w = x | y << width;
                        rs[g][x * size + y] |= at(&bank[a[3] as usize], w);
                        ts[g][x * size + y] &= at(&bank[b[3] as usize], w);
                        goals[g][y] |= at(&bank[d[3] as usize], w);
                    }
                }
            }
        }
    }
    let mut expected = vec![];
    for g in 0..groups {
        let mut reach = rs[g].clone();
        for x in 0..size {
            reach[x * size + x] = true;
        }
        for y in 0..size {
            for x in 0..size {
                for z in 0..size {
                    reach[x * size + z] |= reach[x * size + y] && reach[y * size + z];
                }
            }
        }
        let converse: Vec<_> = (0..size * size)
            .map(|i| reach[(i % size) * size + i / size])
            .collect();
        let residual = residual_matrix(&rs[g], &ts[g], size);
        let may: Vec<_> = (0..size)
            .map(|x| (0..size).any(|y| reach[x * size + y] && goals[g][y]))
            .collect();
        let must: Vec<_> = (0..size)
            .map(|x| {
                (0..size).any(|y| rs[g][x * size + y])
                    && (0..size).all(|y| !rs[g][x * size + y] || goals[g][y])
            })
            .collect();
        expected.extend([
            lifted(&reach, width),
            lifted(&converse, width),
            lifted(&residual, width),
            bits(n, |w| may[w & (size - 1)]),
            bits(n, |w| must[w & (size - 1)]),
        ]);
    }
    let expected_checksum: u64 = expected
        .iter()
        .enumerate()
        .map(|(i, b)| (i + 1) as u64 * b.iter().map(|w| w.count_ones() as u64).sum::<u64>())
        .sum();
    let mut prepared = None;
    let mut builds = vec![];
    for _ in 0..trials() {
        let start = Instant::now();
        let mut c = C::new(&bits(n, |_| true), n);
        let ids: Vec<_> = bank.iter().map(|b| c.import(b)).collect();
        let c = Algebra::new(c, width, false);
        builds.push(start.elapsed().as_secs_f64());
        prepared = Some((c, ids));
    }
    let (input, ids) = prepared.unwrap();
    let data = raw.each_ref().map(|rows| {
        rows.iter()
            .map(|r| [r[0], r[1], r[2], ids[r[3] as usize]])
            .collect()
    });
    let mut engine = native::Native::new(&data, "clover");
    let mut join_only = vec![];
    for _ in 0..trials() {
        let mut rows = 0;
        let start = Instant::now();
        engine.run(|r| {
            black_box(r);
            rows += 1;
        });
        join_only.push(start.elapsed().as_secs_f64());
        assert_eq!(rows, expected_rows);
    }
    for memo in [false, true] {
        let mut base = input.clone();
        base.base.enabled = memo;
        let mut fresh = vec![];
        let mut retained = None;
        let mut iterations = 0;
        for _ in 0..trials() {
            let mut c = base.clone();
            let start = Instant::now();
            let (out, checksum, rows, steps) = query(&mut engine, &mut c, groups);
            fresh.push(start.elapsed().as_secs_f64());
            assert_eq!(
                (black_box(checksum), rows),
                (expected_checksum, expected_rows)
            );
            for (&id, expected) in out.iter().zip(&expected) {
                assert_eq!(c.base.inner.export(id), *expected);
            }
            iterations = steps;
            retained = Some(c);
        }
        let mut c = retained.unwrap();
        let mut warm = vec![];
        for _ in 0..trials() {
            let start = Instant::now();
            let (_, checksum, rows, steps) = query(&mut engine, &mut c, groups);
            warm.push(start.elapsed().as_secs_f64());
            assert_eq!(
                (black_box(checksum), rows, steps),
                (expected_checksum, expected_rows, iterations)
            );
        }
        println!(
            "EVENT_LAB {{\"kind\":\"relations_free_join\",\"candidate\":\"{}\",\"bits\":{},\"worlds\":{},\"rows\":{},\"groups\":{},\"memo\":{},\"build_s\":{:?},\"join_only_s\":{:?},\"fresh_s\":{:?},\"warm_s\":{:?},\"input_bytes_est\":{},\"final_bytes_est\":{},\"final_nodes\":{},\"closure_iterations\":{},\"checksum\":{},\"verified\":true}}",
            C::NAME,
            nbits,
            n,
            expected_rows,
            groups,
            memo,
            builds,
            join_only,
            fresh,
            warm,
            input.bytes(),
            c.bytes(),
            c.base.inner.nodes(),
            iterations,
            expected_checksum
        );
    }
}
