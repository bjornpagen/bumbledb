//! Complete typed relation program on legal domains through actual Free Join.
use super::legal::{Checked, Domain, Domains};
use super::*;

fn domains(width: u32, name: &str) -> Domains {
    let size = 1u64 << width;
    let holes = || Domain::Values((0..size - 1).filter(|v| (v + 1) % 3 != 0).collect());
    match name {
        "full" => Domains::new(width, 0, vec![Domain::Below(size)]),
        "below" => Domains::new(width, 0, vec![Domain::Below(size - 3)]),
        "holes" => Domains::new(width, 0, vec![holes()]),
        "fibred" => Domains::new(width, 1, vec![Domain::Below(size - 3), holes()]),
        _ => panic!("unknown legal domain"),
    }
    .unwrap()
}

struct Fixture {
    domains: Domains,
    bank: Vec<Vec<u64>>,
    raw: [Vec<native::Row>; 3],
    expected: Vec<Vec<u64>>,
    checksum: u64,
    rows: usize,
}
fn fixture(width: u32, name: &str) -> Fixture {
    let d = domains(width, name);
    let n = 1usize << d.dimensions();
    let size = 1usize << width;
    let side = size - 1;
    let bank: Vec<_> = (0..3)
        .flat_map(|family| (0..8).map(move |seed| (family, seed)))
        .map(|(family, seed)| {
            bits(n, |w| {
                let (x, y, e) = (w & side, (w >> width) & side, w >> (3 * width));
                let block = size.min(8);
                match family {
                    0 => {
                        x % 7 != (seed + 2 * e) % 7
                            && x / block == y / block
                            && (y % block == (x + seed + e) % block
                                || y % block == (x + seed + 1 + 2 * e) % block)
                    }
                    1 => {
                        x / block == y / block
                            || ((x + seed + e) % size < size / 4 && y % 5 != seed % 5)
                    }
                    _ => (y + seed + 3 * e) % size < size / 3,
                }
            })
        })
        .collect();
    let mut raw = native::data("clover", 16, 2, 8);
    for (family, rows) in raw.iter_mut().enumerate() {
        for row in rows {
            row[3] += (family * 8) as u64;
        }
    }
    let mut rs = vec![vec![vec![false; size * size]; d.environments()]; 16];
    let mut ts = rs.clone();
    let mut goals = vec![vec![vec![false; size]; d.environments()]; 16];
    for g in 0..16 {
        for e in 0..d.environments() {
            for x in 0..size {
                for y in 0..size {
                    ts[g][e][x * size + y] =
                        d.domain(e).contains(x as u64) && d.domain(e).contains(y as u64);
                }
            }
        }
    }
    let mut rows = 0;
    for a in &raw[0] {
        for b in &raw[1] {
            for q in &raw[2] {
                if a[0] != b[0] || a[0] != q[0] {
                    continue;
                }
                rows += 1;
                let g = a[0] as usize;
                for e in 0..d.environments() {
                    for x in 0..size {
                        for y in 0..size {
                            if !d.domain(e).contains(x as u64) || !d.domain(e).contains(y as u64) {
                                continue;
                            }
                            let w = x | (y << width) | (e << (3 * width));
                            rs[g][e][x * size + y] |= at(&bank[a[3] as usize], w);
                            ts[g][e][x * size + y] &= at(&bank[b[3] as usize], w);
                            goals[g][e][y] |= at(&bank[q[3] as usize], w);
                        }
                    }
                }
            }
        }
    }
    let mut expected = vec![];
    for g in 0..16 {
        let mut reach = rs[g].clone();
        let mut reverse = reach.clone();
        let mut residual = reach.clone();
        let mut may = vec![vec![false; size]; d.environments()];
        let mut must = may.clone();
        for e in 0..d.environments() {
            for x in 0..size {
                reach[e][x * size + x] = d.domain(e).contains(x as u64);
            }
            for y in 0..size {
                for x in 0..size {
                    for z in 0..size {
                        reach[e][x * size + z] |= reach[e][x * size + y] && reach[e][y * size + z];
                    }
                }
            }
            for x in 0..size {
                for y in 0..size {
                    reverse[e][x * size + y] = reach[e][y * size + x];
                    residual[e][x * size + y] = d.domain(e).contains(x as u64)
                        && d.domain(e).contains(y as u64)
                        && (0..size).all(|z| {
                            !d.domain(e).contains(z as u64)
                                || !rs[g][e][z * size + x]
                                || ts[g][e][z * size + y]
                        });
                }
                may[e][x] = (0..size).any(|y| reach[e][x * size + y] && goals[g][e][y]);
                must[e][x] = (0..size).any(|y| rs[g][e][x * size + y])
                    && (0..size).all(|y| !rs[g][e][x * size + y] || goals[g][e][y]);
            }
        }
        let pair = |matrix: &[Vec<bool>]| {
            bits(n, |w| {
                d.contains(w as u64)
                    && matrix[w >> (3 * width)][(w & side) * size + ((w >> width) & side)]
            })
        };
        let state = |matrix: &[Vec<bool>]| {
            bits(n, |w| {
                d.contains(w as u64) && matrix[w >> (3 * width)][w & side]
            })
        };
        expected.extend([
            pair(&reach),
            pair(&reverse),
            pair(&residual),
            state(&may),
            state(&must),
        ]);
    }
    let checksum = expected
        .iter()
        .enumerate()
        .map(|(i, b)| (i + 1) as u64 * b.iter().map(|w| w.count_ones() as u64).sum::<u64>())
        .sum();
    Fixture {
        domains: d,
        bank,
        raw,
        expected,
        checksum,
        rows,
    }
}

fn query<C: RegionOps>(engine: &mut native::Native, c: &mut Checked<C>) -> (Vec<Id>, usize, usize) {
    let empty = c.empty();
    let full = c.full();
    let empty_id = c.root(empty).unwrap();
    let no_goal = c.goal(empty_id).unwrap();
    let mut relations = vec![empty; 16];
    let mut allowed = vec![full; 16];
    let mut goals = vec![no_goal; 16];
    let mut rows = 0;
    engine.run(|[g, scope, r, t, goal]| {
        assert_eq!(scope, 1);
        let i = g as usize;
        let r = c.admit(r).unwrap();
        let t = c.admit(t).unwrap();
        let goal = c.goal(goal).unwrap();
        relations[i] = c.boolean(14, relations[i], r).unwrap();
        allowed[i] = c.boolean(8, allowed[i], t).unwrap();
        goals[i] = c.union_goals(goals[i], goal).unwrap();
        rows += 1;
    });
    let mut out = Vec::with_capacity(80);
    let mut iterations = 0;
    for i in 0..16 {
        let (reach, steps) = c.closure(relations[i]).unwrap();
        iterations += steps;
        let converse = c.converse(reach).unwrap();
        let residual = c.residual(relations[i], allowed[i]).unwrap();
        let may = c.may(reach, goals[i]).unwrap();
        let must = c.must(relations[i], goals[i]).unwrap();
        out.extend([
            c.root(reach).unwrap(),
            c.root(converse).unwrap(),
            c.root(residual).unwrap(),
            may,
            must,
        ]);
    }
    (out, rows, iterations)
}

pub fn bench<C: Carrier>(width: u32) {
    let name = std::env::var("EVENT_LAB_LEGAL_DOMAIN").unwrap_or_else(|_| "below".into());
    let gates = match std::env::var("EVENT_LAB_SUPPORT_GATES").as_deref() {
        Ok("certified") => "certified",
        Ok("gated") | Err(_) => "gated",
        _ => panic!("unknown support strategy"),
    };
    let f = fixture(width, &name);
    let n = 1usize << f.domains.dimensions();
    let layout = std::env::var("EVENT_LAB_LAYOUT").unwrap_or_else(|_| "bit-major".into());
    let mut prepared = None;
    let mut build = vec![];
    let mut support = vec![];
    let mut imports = vec![];
    let mut admission = vec![];
    for _ in 0..trials() {
        let start = Instant::now();
        let mut c = C::legal_space(&f.domains, f.domains.order(&layout)).unwrap();
        support.push(start.elapsed().as_secs_f64());
        let phase = Instant::now();
        let ids: Vec<_> = f.bank.iter().map(|b| c.import(b)).collect();
        imports.push(phase.elapsed().as_secs_f64());
        let phase = Instant::now();
        let c = Checked::new(c, &f.domains, false, gates).unwrap();
        admission.push(phase.elapsed().as_secs_f64());
        build.push(start.elapsed().as_secs_f64());
        prepared = Some((c, ids));
    }
    let (input, ids) = prepared.unwrap();
    let data = f.raw.each_ref().map(|rows| {
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
        assert_eq!(rows, f.rows);
    }
    for memo in [false, true] {
        let mut base = input.clone();
        base.memo(memo);
        let mut fresh = vec![];
        let mut retained = None;
        let mut initial_bytes = 0;
        let mut initial_nodes = 0;
        let mut initial_storage = None;
        let verify = |c: &Checked<C>, out: &[Id], rows: usize| {
            assert_eq!(rows, f.rows);
            assert_eq!(out.len(), 80);
            for (&id, expected) in out.iter().zip(&f.expected) {
                assert_eq!(
                    c.carrier().export(id),
                    *expected,
                    "{} {name} legal relation output",
                    C::NAME
                );
            }
        };
        for _ in 0..trials() {
            let mut c = base.clone();
            initial_bytes = c.bytes();
            initial_nodes = c.carrier().nodes();
            initial_storage = c.carrier().memory_stats();
            let start = Instant::now();
            let (out, rows, _) = query(&mut engine, &mut c);
            let checksum = output_checksum(c.carrier(), &out);
            fresh.push(start.elapsed().as_secs_f64());
            assert_eq!(black_box(checksum), f.checksum);
            verify(&c, &out, rows);
            retained = Some(c);
        }
        let mut c = retained.unwrap();
        let mut warm = vec![];
        for _ in 0..trials() {
            let start = Instant::now();
            let (out, rows, _) = query(&mut engine, &mut c);
            let checksum = output_checksum(c.carrier(), &out);
            warm.push(start.elapsed().as_secs_f64());
            assert_eq!(black_box(checksum), f.checksum);
            verify(&c, &out, rows);
        }
        println!(
            "EVENT_LAB {{\"kind\":\"legal_relation_free_join\",\"candidate\":\"{}\",\"domain\":\"{}\",\"product_mode\":\"{}\",\"support_gates\":\"{}\",\"bits\":{},\"width\":{},\"environments\":{},\"worlds\":{},\"admissible_worlds\":{},\"rows\":{},\"groups\":16,\"outputs\":80,\"memo\":{},\"build_s\":{:?},\"support_s\":{:?},\"import_s\":{:?},\"admission_s\":{:?},\"join_only_s\":{:?},\"fresh_s\":{:?},\"warm_s\":{:?},\"input_bytes_est\":{},\"final_bytes_est\":{},\"input_nodes\":{},\"final_nodes\":{},\"input_storage\":{},\"final_storage\":{},\"checksum\":{},\"verified\":true}}",
            C::NAME,
            name,
            c.strategy(),
            c.gates(),
            f.domains.dimensions(),
            width,
            f.domains.environments(),
            n,
            f.domains.population(),
            f.rows,
            memo,
            build,
            support,
            imports,
            admission,
            join_only,
            fresh,
            warm,
            initial_bytes,
            c.bytes(),
            initial_nodes,
            c.carrier().nodes(),
            memory_json(initial_storage),
            memory_json(c.carrier().memory_stats()),
            f.checksum
        );
    }
}
