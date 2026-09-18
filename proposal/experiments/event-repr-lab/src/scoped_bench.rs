//! Exact staged scoped contraction through Free Join. Coupled fixtures are
//! not admitted to relational_core::Algebra's full-product interpretation.
use super::*;
use rustc_hash::FxHashMap;

#[derive(Clone)]
struct ScopedPlan<C: Carrier> {
    base: Memo<C>,
    maps: [Permutation; 4],
    mask: u64,
    mode: &'static str,
    products: FxHashMap<(Id, Id), Id>,
}
impl<C: Carrier> ScopedPlan<C> {
    fn new(c: C, width: u32, memo: bool) -> Self {
        let mut maps = [[0, 1, 2], [1, 2, 0], [0, 2, 1], [0, 1, 2]].map(|faces| {
            Permutation::new(
                (0..3 * width)
                    .map(|v| faces[(v / width) as usize] * width + v % width)
                    .collect(),
            )
            .unwrap()
        });
        // Rotate physical bits within each logical face. This preserves the
        // binary cube but need not preserve a non-power-of-two legal domain.
        maps[0] = Permutation::new(
            (0..3 * width)
                .map(|v| (v / width) * width + (v % width + 1) % width)
                .collect(),
        )
        .unwrap();
        Self {
            base: Memo::new(c, memo),
            maps,
            mask: ((1u64 << width) - 1) << width,
            mode: if C::VIEW_PRODUCT {
                relational_core::product_mode()
            } else {
                "materialized"
            },
            products: FxHashMap::default(),
        }
    }
    fn product_strategy(&self) -> &'static str {
        self.mode
    }
    fn contract(&mut self, a: Id, b: Id) -> Id {
        // Roots stay ordered because their fixed maps differ. Owner, support,
        // maps and mask are immutable for this memo's entire lifetime.
        if self.base.enabled {
            if let Some(&r) = self.products.get(&(a, b)) {
                return r;
            }
        }
        let c = &mut self.base.inner;
        let r = if self.mode == "materialized" {
            let a = c.permute(a, &self.maps[0]);
            let b = c.permute(b, &self.maps[1]);
            let q = c.relprod(a, b, self.mask);
            c.permute(q, &self.maps[2])
        } else {
            let om = if self.mode == "views-inputs" { 3 } else { 2 };
            let q = c
                .view_product(
                    a,
                    &self.maps[0],
                    b,
                    &self.maps[1],
                    self.mask,
                    &self.maps[om],
                )
                .expect("scoped product capability");
            if om == 3 {
                c.permute(q, &self.maps[2])
            } else {
                q
            }
        };
        if self.base.enabled {
            self.products.insert((a, b), r);
        }
        r
    }
    fn bytes(&self) -> usize {
        self.base.bytes()
            + self.products.capacity() * 32
            + self.maps.iter().map(|m| m.bytes()).sum::<usize>()
    }
}

fn legal(name: &str, size: usize, x: usize, y: usize, z: usize) -> bool {
    match name {
        "full" => true,
        "legal-product" => x < size - 3 && y < size - 3 && z < size - 3,
        "copied-face" => x == z && y < size - 3,
        "asymmetric" => x < size - 3 && y < size - 2 && z < size - 1 && x != z,
        _ => panic!("unknown scoped support"),
    }
}

fn query<C: Carrier>(
    engine: &mut native::Native,
    algebra: &mut ScopedPlan<C>,
    groups: usize,
) -> (Vec<Id>, u64, usize) {
    let mut out = vec![algebra.base.inner.empty(); groups];
    let mut rows = 0;
    engine.run(|[g, scope, r, q, _]| {
        assert_eq!(scope, 1);
        let product = algebra.contract(r, q);
        out[g as usize] = algebra.base.op(14, out[g as usize], product);
        rows += 1;
    });
    let checksum = output_checksum(&algebra.base.inner, &out);
    (out, checksum, rows)
}

pub fn bench<C: Carrier>(nbits: u32) {
    let fixture = relations::fixture(nbits);
    let width = fixture.width;
    let size = 1usize << width;
    let side = size - 1;
    let support_name =
        std::env::var("EVENT_LAB_SCOPED_SUPPORT").unwrap_or_else(|_| "legal-product".into());
    let support = bits(fixture.n, |w| {
        legal(
            &support_name,
            size,
            w & side,
            (w >> width) & side,
            w >> (2 * width),
        )
    });
    let mut oracle = FxHashMap::<(usize, usize), Vec<u64>>::default();
    let mut expected = vec![vec![0u64; fixture.n.div_ceil(64)]; fixture.groups];
    let mut expected_rows = 0;
    for a in &fixture.raw[0] {
        for b in &fixture.raw[1] {
            for d in &fixture.raw[2] {
                if a[0] != b[0] || a[0] != d[0] {
                    continue;
                }
                expected_rows += 1;
                let key = (a[3] as usize, b[3] as usize);
                let value = oracle.entry(key).or_insert_with(|| {
                    // Logical XYZ oracle checks each intermediate support explicitly.
                    // It does not implement the inverse-map support-gate identity.
                    let mut matrix = vec![false; size * size];
                    for x in 0..size {
                        for z in 0..size {
                            matrix[x * size + z] = (0..size).any(|y| {
                                let rotate = |v: usize| (v >> 1) | ((v & 1) << (width - 1));
                                let (ax, ay, az) = (rotate(x), rotate(y), rotate(z));
                                legal(&support_name, size, x, y, z)
                                    && legal(&support_name, size, ax, ay, az)
                                    && legal(&support_name, size, y, z, x)
                                    && at(
                                        &fixture.bank[key.0],
                                        ax | (ay << width) | (az << (2 * width)),
                                    )
                                    && at(
                                        &fixture.bank[key.1],
                                        y | (z << width) | (x << (2 * width)),
                                    )
                            });
                        }
                    }
                    bits(fixture.n, |w| {
                        let (x, y, z) = (w & side, (w >> width) & side, w >> (2 * width));
                        legal(&support_name, size, x, y, z)
                            && legal(&support_name, size, x, z, y)
                            && matrix[x * size + y]
                    })
                });
                for (a, b) in expected[a[0] as usize].iter_mut().zip(value.iter()) {
                    *a |= *b;
                }
            }
        }
    }
    assert_eq!(expected_rows, fixture.expected_rows);
    let expected_checksum: u64 = expected
        .iter()
        .enumerate()
        .map(|(i, b)| (i + 1) as u64 * b.iter().map(|w| w.count_ones() as u64).sum::<u64>())
        .sum();
    assert!(
        expected_checksum > 0,
        "fixture must retain possible outputs"
    );
    let mut builds = vec![];
    let mut prepared = None;
    for _ in 0..trials() {
        let start = Instant::now();
        let mut carrier = C::with_order(&support, fixture.n, relation_order(nbits));
        let ids: Vec<_> = fixture.bank.iter().map(|b| carrier.import(b)).collect();
        let algebra = ScopedPlan::new(carrier, fixture.width, false);
        builds.push(start.elapsed().as_secs_f64());
        prepared = Some((algebra, ids));
    }
    let (input, ids) = prepared.unwrap();
    let data = fixture.raw.each_ref().map(|rows| {
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
        let mut initial_bytes = 0;
        let mut initial_nodes = 0;
        let mut initial_storage = None;
        for _ in 0..trials() {
            let mut c = base.clone();
            initial_bytes = c.bytes();
            initial_nodes = c.base.inner.nodes();
            initial_storage = c.base.inner.memory_stats();
            let start = Instant::now();
            let (out, checksum, rows) = query(&mut engine, &mut c, fixture.groups);
            fresh.push(start.elapsed().as_secs_f64());
            assert_eq!(
                (black_box(checksum), rows),
                (expected_checksum, expected_rows)
            );
            for (&id, expected) in out.iter().zip(&expected) {
                assert_eq!(c.base.inner.export(id), *expected);
            }
            retained = Some(c);
        }
        let mut c = retained.unwrap();
        let mut warm = vec![];
        for _ in 0..trials() {
            let start = Instant::now();
            let (out, checksum, rows) = query(&mut engine, &mut c, fixture.groups);
            warm.push(start.elapsed().as_secs_f64());
            assert_eq!(
                (black_box(checksum), rows),
                (expected_checksum, expected_rows)
            );
            for (&id, expected) in out.iter().zip(&expected) {
                assert_eq!(c.base.inner.export(id), *expected);
            }
        }
        println!(
            "EVENT_LAB {{\"kind\":\"scoped_product_free_join\",\"candidate\":\"{}\",\"product_mode\":\"{}\",\"support\":\"{}\",\"admissible_worlds\":{},\"bits\":{},\"worlds\":{},\"rows\":{},\"groups\":{},\"memo\":{},\"build_s\":{:?},\"join_only_s\":{:?},\"fresh_s\":{:?},\"warm_s\":{:?},\"input_bytes_est\":{},\"final_bytes_est\":{},\"input_nodes\":{},\"final_nodes\":{},\"input_storage\":{},\"final_storage\":{},\"checksum\":{},\"verified\":true}}",
            C::NAME,
            c.product_strategy(),
            support_name,
            support.iter().map(|w| w.count_ones() as u64).sum::<u64>(),
            nbits,
            fixture.n,
            expected_rows,
            fixture.groups,
            memo,
            builds,
            join_only,
            fresh,
            warm,
            initial_bytes,
            c.bytes(),
            initial_nodes,
            c.base.inner.nodes(),
            memory_json(initial_storage),
            memory_json(c.base.inner.memory_stats()),
            expected_checksum
        );
    }
}
