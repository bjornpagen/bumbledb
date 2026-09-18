//! One relation product per complete binding, grouped through actual Free Join.
//! This is a separate answer contract from the full closure/residual program.
use super::relational_core::{Algebra, lifted};
use super::*;

fn query<C: Carrier>(
    engine: &mut native::Native,
    algebra: &mut Algebra<C>,
    groups: usize,
) -> (Vec<Id>, u64, usize) {
    let mut out = vec![algebra.base.inner.empty(); groups];
    let mut rows = 0;
    engine.run(|[g, scope, r, q, _]| {
        assert_eq!(scope, 1);
        let product = algebra.compose(r, q);
        out[g as usize] = algebra.base.op(14, out[g as usize], product);
        rows += 1;
    });
    let checksum = output_checksum(&algebra.base.inner, &out);
    (out, checksum, rows)
}

pub fn bench<C: Carrier>(nbits: u32) {
    let fixture = relations::fixture(nbits);
    let size = 1usize << fixture.width;
    let mut matrices = vec![vec![false; size * size]; fixture.groups];
    let mut expected_rows = 0;
    for a in &fixture.raw[0] {
        for b in &fixture.raw[1] {
            for d in &fixture.raw[2] {
                if a[0] != b[0] || a[0] != d[0] {
                    continue;
                }
                expected_rows += 1;
                for x in 0..size {
                    for z in 0..size {
                        matrices[a[0] as usize][x * size + z] |= (0..size).any(|y| {
                            at(&fixture.bank[a[3] as usize], x | y << fixture.width)
                                && at(&fixture.bank[b[3] as usize], y | z << fixture.width)
                        });
                    }
                }
            }
        }
    }
    assert_eq!(expected_rows, fixture.expected_rows);
    let expected: Vec<_> = matrices.iter().map(|m| lifted(m, fixture.width)).collect();
    let expected_checksum: u64 = expected
        .iter()
        .enumerate()
        .map(|(i, b)| (i + 1) as u64 * b.iter().map(|w| w.count_ones() as u64).sum::<u64>())
        .sum();
    let mut builds = vec![];
    let mut prepared = None;
    for _ in 0..trials() {
        let start = Instant::now();
        let mut carrier =
            C::with_order(&bits(fixture.n, |_| true), fixture.n, relation_order(nbits));
        let ids: Vec<_> = fixture.bank.iter().map(|b| carrier.import(b)).collect();
        let algebra = Algebra::new(carrier, fixture.width, false);
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
            let (_, checksum, rows) = query(&mut engine, &mut c, fixture.groups);
            warm.push(start.elapsed().as_secs_f64());
            assert_eq!(
                (black_box(checksum), rows),
                (expected_checksum, expected_rows)
            );
        }
        println!(
            "EVENT_LAB {{\"kind\":\"product_free_join\",\"candidate\":\"{}\",\"product_mode\":\"{}\",\"bits\":{},\"worlds\":{},\"rows\":{},\"groups\":{},\"memo\":{},\"build_s\":{:?},\"join_only_s\":{:?},\"fresh_s\":{:?},\"warm_s\":{:?},\"input_bytes_est\":{},\"final_bytes_est\":{},\"input_nodes\":{},\"final_nodes\":{},\"input_storage\":{},\"final_storage\":{},\"checksum\":{},\"verified\":true}}",
            C::NAME,
            c.product_strategy(),
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
