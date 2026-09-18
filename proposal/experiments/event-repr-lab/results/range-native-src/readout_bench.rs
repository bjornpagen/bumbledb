//! Publish information readouts after actual Free Join and grouped Event union.
use super::legal::{Domain, Domains};
use super::*;

struct Fixture {
    domains: Domains,
    bank: Vec<Vec<u64>>,
    rows: [Vec<native::Row>; 3],
    expected: Vec<[Vec<u64>; 4]>,
    bindings: usize,
    checksum: u64,
    changed_possible: usize,
    changed_guaranteed: usize,
    nonconstant_readouts: usize,
}

fn fixture(width: u32, name: &str, hidden: u64) -> Fixture {
    let size = 1usize << width;
    let holes = || Domain::Values((0..size as u64 - 1).filter(|v| (v + 1) % 3 != 0).collect());
    let domains = match name {
        "full" => Domains::new(width, 0, vec![Domain::Below(size as u64)]),
        "below" => Domains::new(width, 0, vec![Domain::Below(size as u64 - 3)]),
        "holes" => Domains::new(width, 0, vec![holes()]),
        "fibred" => Domains::new(width, 1, vec![Domain::Below(size as u64 - 3), holes()]),
        _ => panic!("unknown legal readout domain"),
    }
    .unwrap();
    let n = 1usize << domains.dimensions();
    let support = bits(n, |w| domains.contains(w as u64));
    let bank: Vec<_> = (0..3)
        .flat_map(|family| (0..8).map(move |seed| (family, seed)))
        .map(|(family, seed)| {
            bits(n, |w| {
                let (x, y, e) = (w & (size - 1), (w >> width) & (size - 1), w >> (3 * width));
                let block = size.min(8);
                match family {
                    0 => {
                        x % 7 != (seed + 2 * e) % 7
                            && x / block == y / block
                            && (y % block == (x + seed + e) % block
                                || y % block == (x + seed + 1 + 2 * e) % block)
                    }
                    1 => (x + seed + e) % 5 != 0 && (y + 2 * seed + e) % 7 != 0,
                    _ => (y + seed + 3 * e) % size < size / 3,
                }
            })
        })
        .collect();
    let mut rows = native::data("clover", 16, 2, 8);
    for (family, rs) in rows.iter_mut().enumerate() {
        for row in rs {
            row[3] += (8 * family) as u64;
        }
    }
    let mut groups = vec![vec![0; n.div_ceil(64)]; 16];
    let mut bindings = 0;
    // Direct scalar join plus word-set oracle, independent of the native plan.
    for a in &rows[0] {
        for b in &rows[1] {
            for q in &rows[2] {
                if a[0] == b[0] && a[0] == q[0] {
                    bindings += 1;
                    for (i, word) in groups[a[0] as usize].iter_mut().enumerate() {
                        *word |= bank[a[3] as usize][i]
                            & bank[b[3] as usize][i]
                            & bank[q[3] as usize][i];
                    }
                }
            }
        }
    }
    let expected: Vec<_> = groups
        .iter()
        .map(|event| readout::reference(&support, event, n, hidden))
        .collect();
    let checksum = expected
        .iter()
        .flatten()
        .enumerate()
        .map(|(i, words)| (i + 1) as u64 * words.iter().map(|w| w.count_ones() as u64).sum::<u64>())
        .sum();
    let changed_possible = expected.iter().filter(|r| r[0] != r[1]).count();
    let changed_guaranteed = expected.iter().filter(|r| r[0] != r[2]).count();
    let nonconstant_readouts = expected
        .iter()
        .flat_map(|r| &r[1..])
        .filter(|r| r.iter().any(|&w| w != 0) && r.as_slice() != support.as_slice())
        .count();
    Fixture {
        domains,
        bank,
        rows,
        expected,
        bindings,
        checksum,
        changed_possible,
        changed_guaranteed,
        nonconstant_readouts,
    }
}

fn query<C: Carrier>(
    engine: &mut native::Native,
    c: &mut Memo<C>,
    hidden: u64,
) -> (Vec<Id>, usize, f64, f64) {
    let phase = Instant::now();
    let mut grouped = vec![c.inner.empty(); 16];
    let mut bindings = 0;
    engine.run(|[group, scope, relation, allowed, target]| {
        assert_eq!(scope, 1);
        let a = c.op(8, relation, allowed);
        let term = c.op(8, a, target);
        let i = group as usize;
        grouped[i] = c.op(14, grouped[i], term);
        bindings += 1;
    });
    let grouped_s = phase.elapsed().as_secs_f64();
    let phase = Instant::now();
    let mut out = Vec::with_capacity(64);
    for event in grouped {
        out.extend(readout::construct(c, event, hidden));
    }
    (out, bindings, grouped_s, phase.elapsed().as_secs_f64())
}

pub fn bench<C: Carrier>(width: u32) {
    let domain = std::env::var("EVENT_LAB_LEGAL_DOMAIN").unwrap_or_else(|_| "fibred".into());
    let family = std::env::var("EVENT_LAB_READOUT_FAMILY").unwrap_or_else(|_| "suffix-1".into());
    let faces = std::env::var("EVENT_LAB_READOUT_FACES").unwrap_or_else(|_| "xy".into());
    let layout = std::env::var("EVENT_LAB_LAYOUT").unwrap_or_else(|_| "bit-major".into());
    let hidden = readout::mask(width, &family, &faces);
    let f = fixture(width, &domain, hidden);
    assert_eq!(f.bindings, 128);
    assert!(
        f.changed_possible + f.changed_guaranteed > 0,
        "degenerate readout fixture"
    );
    let mut support = vec![];
    let mut imports = vec![];
    let mut admission = vec![];
    let mut build = vec![];
    let mut prepared = None;
    for _ in 0..trials() {
        let start = Instant::now();
        let mut c = C::legal_space(&f.domains, f.domains.order(&layout)).unwrap();
        support.push(start.elapsed().as_secs_f64());
        let phase = Instant::now();
        let ids: Vec<_> = f.bank.iter().map(|words| c.import(words)).collect();
        imports.push(phase.elapsed().as_secs_f64());
        let phase = Instant::now();
        f.domains.verify(&mut c).unwrap();
        admission.push(phase.elapsed().as_secs_f64());
        build.push(start.elapsed().as_secs_f64());
        prepared = Some((c, ids));
    }
    let (owner, ids) = prepared.unwrap();
    let data = f.rows.each_ref().map(|rows| {
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
        assert_eq!(rows, f.bindings);
    }
    for memo in [false, true] {
        let input = Memo::new(owner.clone(), memo);
        let mut fresh = vec![];
        let mut fresh_group = vec![];
        let mut fresh_readout = vec![];
        let mut fresh_compute = vec![];
        let mut fresh_count = vec![];
        let mut retained = None;
        let mut initial = None;
        for _ in 0..trials() {
            let mut c = input.clone();
            let current = (c.inner.nodes(), c.bytes(), c.inner.memory_stats());
            if let Some(old) = initial {
                assert_eq!(current, old);
            }
            initial = Some(current);
            let start = Instant::now();
            let (out, rows, grouped_s, readout_s) = query(&mut engine, &mut c, hidden);
            fresh_group.push(grouped_s);
            fresh_readout.push(readout_s);
            fresh_compute.push(start.elapsed().as_secs_f64());
            let phase = Instant::now();
            let checksum = output_checksum(&c.inner, &out);
            fresh_count.push(phase.elapsed().as_secs_f64());
            fresh.push(start.elapsed().as_secs_f64());
            assert_eq!(black_box(checksum), f.checksum);
            assert_eq!(rows, f.bindings);
            if std::env::var("EVENT_LAB_REACHABILITY").as_deref() == Ok("1") {
                let before = (c.inner.nodes(), c.inner.bytes(), c.inner.memory_stats());
                if let Some(graphs) = c.inner.root_diagnostics(&ids, &out) {
                    assert_eq!(
                        (c.inner.nodes(), c.inner.bytes(), c.inner.memory_stats()),
                        before
                    );
                    println!(
                        concat!(
                            "EVENT_LAB {{\"kind\":\"readout_reachability\",",
                            "\"candidate\":\"{}\",\"width\":{},\"memo\":{},\"graphs\":{},",
                            "\"verified\":true,\"verification_scope\":\"read-only graph census; full output checks follow\"}}"
                        ),
                        C::NAME,
                        width,
                        memo,
                        graphs
                    );
                }
            }
            readout::verify_batch(&c.inner, &out, &f.expected, hidden);
            retained = Some(c);
        }
        let mut c = retained.unwrap();
        let mut warm = vec![];
        let mut warm_group = vec![];
        let mut warm_readout = vec![];
        let mut warm_compute = vec![];
        let mut warm_count = vec![];
        for _ in 0..trials() {
            let start = Instant::now();
            let (out, rows, grouped_s, readout_s) = query(&mut engine, &mut c, hidden);
            warm_group.push(grouped_s);
            warm_readout.push(readout_s);
            warm_compute.push(start.elapsed().as_secs_f64());
            let phase = Instant::now();
            let checksum = output_checksum(&c.inner, &out);
            warm_count.push(phase.elapsed().as_secs_f64());
            warm.push(start.elapsed().as_secs_f64());
            assert_eq!(black_box(checksum), f.checksum);
            assert_eq!(rows, f.bindings);
            readout::verify_batch(&c.inner, &out, &f.expected, hidden);
        }
        let (input_nodes, input_bytes, input_storage) = initial.unwrap();
        println!(
            concat!(
                "EVENT_LAB {{\"kind\":\"readout_free_join\",\"candidate\":\"{}\",\"domain\":\"{}\",",
                "\"readout_family\":\"{}\",\"readout_faces\":\"{}\",\"hidden\":{},\"width\":{},\"bits\":{},\"worlds\":{},",
                "\"admissible_worlds\":{},\"environments\":{},\"rows\":{},\"groups\":16,\"outputs\":64,\"memo\":{},",
                "\"changed_possible_groups\":{},\"changed_guaranteed_groups\":{},\"nonconstant_readouts\":{},",
                "\"build_s\":{:?},\"support_s\":{:?},\"import_s\":{:?},\"admission_s\":{:?},\"join_only_s\":{:?},",
                "\"fresh_s\":{:?},\"fresh_group_s\":{:?},\"fresh_readout_s\":{:?},\"fresh_compute_s\":{:?},\"fresh_count_s\":{:?},",
                "\"warm_s\":{:?},\"warm_group_s\":{:?},\"warm_readout_s\":{:?},\"warm_compute_s\":{:?},\"warm_count_s\":{:?},",
                "\"input_nodes\":{},\"final_nodes\":{},\"input_bytes_est\":{},\"final_bytes_est\":{},",
                "\"input_storage\":{},\"final_storage\":{},\"outer_memo_bytes\":{},\"colt_bytes\":{},\"checksum\":{},\"verified\":true}}"
            ),
            C::NAME,
            domain,
            family,
            faces,
            hidden,
            width,
            f.domains.dimensions(),
            1usize << f.domains.dimensions(),
            f.domains.population(),
            f.domains.environments(),
            f.bindings,
            memo,
            f.changed_possible,
            f.changed_guaranteed,
            f.nonconstant_readouts,
            build,
            support,
            imports,
            admission,
            join_only,
            fresh,
            fresh_group,
            fresh_readout,
            fresh_compute,
            fresh_count,
            warm,
            warm_group,
            warm_readout,
            warm_compute,
            warm_count,
            input_nodes,
            c.inner.nodes(),
            input_bytes,
            c.bytes(),
            memory_json(input_storage),
            memory_json(c.inner.memory_stats()),
            c.bytes() - c.inner.bytes(),
            engine.retained_colt_bytes(),
            f.checksum
        );
    }
}
