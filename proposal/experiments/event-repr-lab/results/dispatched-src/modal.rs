//! A non-Boolean query through the real executor: composition, domain and Must.
use super::*;
use rustc_hash::FxHashMap;

#[derive(Clone)]
struct Ops<C: Carrier> {
    base: Memo<C>,
    products: FxHashMap<(Id, Id, u32), Id>,
    abstractions: FxHashMap<(Id, u32), Id>,
}
impl<C: Carrier> Ops<C> {
    fn product(&mut self, a: Id, b: Id, mask: u32) -> Id {
        let key = (a.min(b), a.max(b), mask);
        if self.base.enabled {
            if let Some(&r) = self.products.get(&key) {
                return r;
            }
        }
        let r = self.base.inner.relprod(a, b, mask);
        if self.base.enabled {
            self.products.insert(key, r);
        }
        r
    }
    fn exists(&mut self, a: Id, mask: u32) -> Id {
        if self.base.enabled {
            if let Some(&r) = self.abstractions.get(&(a, mask)) {
                return r;
            }
        }
        let r = self.base.inner.exists(a, mask);
        if self.base.enabled {
            self.abstractions.insert((a, mask), r);
        }
        r
    }
    fn bytes(&self) -> usize {
        self.base.bytes() + self.products.capacity() * 40 + self.abstractions.capacity() * 32
    }
}

fn query_modal<C: Carrier>(
    native: &mut native::Native,
    c: &mut Ops<C>,
    groups: usize,
    width: u32,
) -> (Vec<Id>, u64, usize) {
    let side = (1u32 << width) - 1;
    let mut enabled = vec![c.base.inner.empty(); groups];
    let mut bad = enabled.clone();
    let mut rows = 0;
    native.run(|[g, scope, a, b, target]| {
        assert_eq!(scope, 1);
        let reach = c.product(a, b, side << width); // R ; Q, hiding the middle state Y
        let domain = c.exists(reach, side << (2 * width));
        let neg = c.base.inner.not(target);
        let counterexample = c.product(reach, neg, side << (2 * width));
        let i = g as usize;
        enabled[i] = c.base.op(14, enabled[i], domain);
        bad[i] = c.base.op(14, bad[i], counterexample);
        rows += 1;
    });
    let out: Vec<_> = enabled
        .into_iter()
        .zip(bad)
        .map(|(d, b)| c.base.op(4, d, b))
        .collect();
    let checksum = output_checksum(&c.base.inner, &out);
    (out, checksum, rows)
}

pub fn bench_modal<C: Carrier>(nbits: u32) {
    let trials = trials();
    let n = 1usize << nbits;
    let width = nbits / 3;
    let size = 1usize << width;
    let side = size - 1;
    let groups = 16;
    let bank_size = 8;
    // Stable nondeterministic local transitions; alternate enabled/dead states.
    let bank: Vec<_> = (0..3)
        .flat_map(|family| {
            (0..bank_size).map(move |i| {
                bits(n, |w| {
                    let (x, y, z) = (w & side, (w >> width) & side, w >> (2 * width));
                    match family {
                        0 => x % 7 != i % 7 && (y == (x + i) & side || y == (x + i + 1) & side),
                        1 => y % 5 != i % 5 && (z == (y + i) & side || z == (y + i + 2) & side),
                        _ => (z + i) % size < size * 3 / 4,
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
    // Independent pointwise definition, without carrier operations or axis reduction.
    let mut domain = vec![vec![false; size]; groups];
    let mut bad = domain.clone();
    let mut expected_rows = 0;
    for a in &raw[0] {
        for b in &raw[1] {
            if a[0] != b[0] {
                continue;
            }
            for c in &raw[2] {
                if a[0] != c[0] {
                    continue;
                }
                expected_rows += 1;
                for x in 0..size {
                    for y in 0..size {
                        for z in 0..size {
                            let w = x | (y << width) | (z << (2 * width));
                            if at(&bank[a[3] as usize], w) && at(&bank[b[3] as usize], w) {
                                domain[a[0] as usize][x] = true;
                                bad[a[0] as usize][x] |= !at(&bank[c[3] as usize], w);
                            }
                        }
                    }
                }
            }
        }
    }
    // Result is an X predicate lifted to the original XYZ presentation.
    let expected: Vec<_> = domain
        .iter()
        .zip(&bad)
        .map(|(d, b)| bits(n, |w| d[w & side] && !b[w & side]))
        .collect();
    assert!(
        expected.iter().any(|b| b.iter().any(|&w| w != 0)),
        "nontrivial Must result"
    );
    let expected_checksum: u64 = expected
        .iter()
        .enumerate()
        .map(|(i, b)| b.iter().map(|w| w.count_ones() as u64).sum::<u64>() * (i + 1) as u64)
        .sum();
    let support = bits(n, |_| true);
    let mut builds = vec![];
    let mut prepared = None;
    for _ in 0..trials {
        let start = Instant::now();
        let mut c = C::new(&support, n);
        let ids: Vec<_> = bank.iter().map(|b| c.import(b)).collect();
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
    engine.run(|r| {
        black_box(r);
    });
    let mut join_only = vec![];
    for _ in 0..trials {
        let start = Instant::now();
        let mut rows = 0;
        engine.run(|r| {
            black_box(r);
            rows += 1;
        });
        join_only.push(start.elapsed().as_secs_f64());
        assert_eq!(rows, expected_rows);
    }
    for memo in [false, true] {
        let base = Ops {
            base: Memo::new(input.clone(), memo),
            products: FxHashMap::default(),
            abstractions: FxHashMap::default(),
        };
        let mut fresh = vec![];
        let mut retained = None;
        for _ in 0..trials {
            let mut c = base.clone();
            let start = Instant::now();
            let (out, checksum, rows) = query_modal(&mut engine, &mut c, groups, width);
            fresh.push(start.elapsed().as_secs_f64());
            assert_eq!(
                (black_box(checksum), rows),
                (expected_checksum, expected_rows)
            );
            for (&id, b) in out.iter().zip(&expected) {
                assert_eq!(c.base.inner.export(id), *b);
            }
            retained = Some(c);
        }
        let mut c = retained.unwrap();
        let mut loops = 1;
        loop {
            let start = Instant::now();
            for _ in 0..loops {
                black_box(query_modal(&mut engine, &mut c, groups, width).1);
            }
            if start.elapsed().as_secs_f64() >= 0.02 || loops >= 4096 {
                break;
            }
            loops *= 2;
        }
        let mut warm = vec![];
        for _ in 0..trials {
            let start = Instant::now();
            let mut checksum = 0;
            for _ in 0..loops {
                checksum = black_box(query_modal(&mut engine, &mut c, groups, width).1);
            }
            warm.push(start.elapsed().as_secs_f64() / loops as f64);
            assert_eq!(checksum, expected_checksum);
        }
        println!(
            "EVENT_LAB {{\"kind\":\"modal_free_join\",\"candidate\":\"{}\",\"bits\":{},\"worlds\":{},\"rows\":{},\"groups\":{},\"memo\":{},\"build_s\":{:?},\"join_only_s\":{:?},\"fresh_s\":{:?},\"warm_s\":{:?},\"input_bytes_est\":{},\"final_bytes_est\":{},\"final_nodes\":{},\"checksum\":{},\"verified\":true}}",
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
            expected_checksum
        );
    }
}

pub fn bench_complement<C: Carrier>(restricted: bool) {
    let trials = trials();
    let n = 4096;
    let support = bits(n, |w| !restricted || mix(w as u64) & 3 != 0);
    let bank = workload("random_4096").1;
    let mut prepared = None;
    let mut build = vec![];
    for _ in 0..trials {
        let start = Instant::now();
        let mut c = C::new(&support, n);
        let ids: Vec<_> = bank.iter().map(|b| c.import(b)).collect();
        build.push(start.elapsed().as_secs_f64());
        prepared = Some((c, ids));
    }
    let (input, ids) = prepared.unwrap();
    let mut cold = vec![];
    let mut retained = None;
    for _ in 0..trials {
        let mut c = input.clone();
        let start = Instant::now();
        let negs: Vec<_> = ids.iter().map(|&id| c.not(black_box(id))).collect();
        black_box(&negs);
        cold.push(start.elapsed().as_secs_f64());
        for ((&id, &neg), raw) in ids.iter().zip(&negs).zip(&bank) {
            assert_eq!(c.not(neg), id);
            let expected: Vec<_> = support.iter().zip(raw).map(|(&s, &a)| s & !a).collect();
            assert_eq!(c.export(neg), expected);
        }
        retained = Some(c);
    }
    let mut c = retained.unwrap();
    let loops = 1024;
    let mut warm = vec![];
    for _ in 0..trials {
        let start = Instant::now();
        for _ in 0..loops {
            for &id in &ids {
                black_box(c.not(black_box(id)));
            }
        }
        warm.push(start.elapsed().as_secs_f64() / (loops * ids.len()) as f64);
    }
    println!(
        "EVENT_LAB {{\"kind\":\"complement\",\"candidate\":\"{}\",\"restricted\":{},\"events\":{},\"build_s\":{:?},\"first_batch_s\":{:?},\"warm_per_not_s\":{:?},\"input_bytes_est\":{},\"final_bytes_est\":{},\"verified\":true}}",
        C::NAME,
        restricted,
        ids.len(),
        build,
        cold,
        warm,
        input.bytes(),
        c.bytes()
    );
}
