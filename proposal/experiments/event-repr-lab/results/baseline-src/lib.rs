#![allow(clippy::all, clippy::pedantic, dead_code)]
mod carrier;
mod diagram;
mod finite;
mod native;
use carrier::*;
use diagram::Diagram;
use finite::*;
use std::hint::black_box;
use std::time::Instant;

type Bdd = Diagram<2, true>;
type Root = Diagram<2, false>;
type Mdd = Diagram<4, true>;

fn verify<C: Carrier>() {
    let mut operations = 0;
    for s in 1..16u64 {
        let mut c = C::new(&[s], 4);
        let ids: Vec<_> = (0..16).map(|b| c.import(&[b])).collect();
        for a in 0..16 {
            assert_eq!(c.export(ids[a]), vec![a as u64 & s]);
            let neg = c.not(ids[a]);
            assert_eq!(c.export(neg), vec![s ^ (a as u64 & s)]);
            for b in 0..16 {
                assert_eq!(ids[a] == ids[b], (a as u64 & s) == (b as u64 & s));
                for op in 0..16u8 {
                    let r = c.op(op, ids[a], ids[b]);
                    let expected = (0..4).fold(0u64, |out, w| {
                        out | ((((op as u64) >> ((((a >> w) & 1) * 2) + ((b >> w) & 1))) & 1)
                            & (s >> w & 1))
                            << w
                    });
                    assert_eq!(
                        c.export(r),
                        vec![expected],
                        "{} support {s} op {op} a {a} b {b}",
                        C::NAME
                    );
                    operations += 1;
                }
            }
            for mask in 0..4u32 {
                let r = c.exists(ids[a], mask);
                let expected = (0..4).fold(0u64, |out, w| {
                    let yes = (0..4).any(|v| {
                        v & !(mask as usize) == w & !(mask as usize)
                            && (s >> v & 1) != 0
                            && (a >> v & 1) != 0
                    });
                    out | (((s >> w & 1) != 0 && yes) as u64) << w
                });
                assert_eq!(c.export(r), vec![expected], "{} exists", C::NAME);
            }
        }
    }
    // More than one machine word, invalid padding, and exact cardinality.
    for n in [65, 129, 257] {
        let s = bits(n, |w| w % 3 != 0);
        let mut c = C::new(&s, n);
        let a = c.import(&bits(n, |w| w % 5 < 2));
        let b = c.not(a);
        assert_eq!(
            c.count(a) + c.count(b),
            s.iter().map(|w| w.count_ones() as u64).sum()
        );
        let expected: Vec<_> = s
            .iter()
            .zip(bits(n, |w| w % 5 < 2))
            .map(|(&s, a)| s & !a)
            .collect();
        assert_eq!(c.export(b), expected);
    }
    // Fused product versus independent existential quantification of predicates.
    let n = 4096;
    let mut c = C::new(&bits(n, |_| true), n);
    let ra = bits(n, |w| {
        let (x, y) = (w & 15, (w >> 4) & 15);
        y == x || y == (x + 1) % 16
    });
    let qb = bits(n, |w| {
        let (y, z) = ((w >> 4) & 15, w >> 8);
        z == (3 * y) % 16 || z == (y + 2) % 16
    });
    let r = c.import(&ra);
    let q = c.import(&qb);
    let out = c.relprod(r, q, 0xf0);
    let expected = bits(n, |w| {
        (0..16).any(|y| {
            let v = (w & !0xf0) | (y << 4);
            at(&ra, v) && at(&qb, v)
        })
    });
    assert_eq!(c.export(out), expected);
    println!(
        "EVENT_LAB {{\"kind\":\"verification\",\"candidate\":\"{}\",\"boolean_cases\":{},\"passed\":true}}",
        C::NAME,
        operations
    );
}

#[test]
fn semantics() {
    verify::<Finite<Dense>>();
    verify::<Finite<Sparse>>();
    verify::<Finite<Roaring>>();
    verify::<Finite<Runs>>();
    verify::<Root>();
    verify::<Bdd>();
    verify::<Mdd>();
    // Native join routing is checked independently of every candidate.
    for shape in ["triangle", "clover"] {
        let bank = workload("random_4096").1;
        let d = native::data(shape, 8, 3, bank.len());
        let (_, expected) = native::oracle(&d, shape, &bank, 8);
        let mut n = native::Native::new(&d, shape);
        let mut found = 0;
        n.run(|r| {
            assert_eq!(r[1], 1);
            found += 1;
        });
        assert_eq!(found, expected);
    }
}

fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}
fn workload(name: &str) -> (usize, Vec<Vec<u64>>) {
    if name == "coup_4290" {
        let cards: Vec<_> = (0usize..15).filter(|&c| c != 0 && c != 3).collect();
        let mut deals = Vec::new();
        for (i, &a) in cards.iter().enumerate() {
            for &b in &cards[i + 1..] {
                let left: Vec<_> = cards
                    .iter()
                    .copied()
                    .filter(|&c| c != a && c != b)
                    .collect();
                for (j, &c) in left.iter().enumerate() {
                    for &d in &left[j + 1..] {
                        deals.push([a, b, c, d]);
                    }
                }
            }
        }
        assert_eq!(deals.len(), 4290);
        let mut bank = Vec::new();
        for seat in 0..2 {
            for role in 0..5 {
                bank.push(bits(deals.len(), |w| {
                    deals[w][2 * seat] / 3 == role || deals[w][2 * seat + 1] / 3 == role
                }));
            }
            for &card in &cards {
                bank.push(bits(deals.len(), |w| {
                    deals[w][2 * seat] == card || deals[w][2 * seat + 1] == card
                }));
            }
        }
        return (deals.len(), bank);
    }
    let n = if name.ends_with("65536") { 65536 } else { 4096 };
    let bank = (0..48)
        .map(|i| {
            bits(n, |w| match name {
                "random_4096" => mix((i * n + w) as u64) & 1 != 0,
                "sparse_65536" => mix((i * n + w) as u64) & 1023 < 4,
                "runs_65536" => {
                    let p = (w + i * 1379) % n;
                    p < n / 8 || (p > n / 2 && p < n / 2 + n / 32)
                }
                "structured_65536" => {
                    let a = (w >> (i % 16)) & 1;
                    let b = (w >> ((i * 7 + 3) % 16)) & 1;
                    match i % 3 {
                        0 => a == 1,
                        1 => a == b,
                        _ => a != b,
                    }
                }
                "overlap_4096" => mix(((i % 8) * n + w) as u64) & 7 < 3,
                _ => panic!("unknown workload {name}"),
            })
        })
        .collect();
    (n, bank)
}
fn quantile(v: &[f64], q: f64) -> f64 {
    let mut a = v.to_vec();
    a.sort_by(f64::total_cmp);
    a[((a.len() - 1) as f64 * q).round() as usize]
}
fn output_checksum<C: Carrier>(c: &C, outputs: &[Id]) -> u64 {
    outputs
        .iter()
        .enumerate()
        .map(|(i, &e)| c.count(e).wrapping_mul((i + 1) as u64))
        .sum()
}
fn query<C: Carrier>(
    native: &mut native::Native,
    c: &mut Memo<C>,
    groups: usize,
) -> (Vec<Id>, u64, usize) {
    let mut out = vec![c.inner.empty(); groups];
    let mut rows = 0;
    native.run(|[g, scope, a, b, d]| {
        assert_eq!(scope, 1, "scoped input");
        let e = c.expression(a, b, d);
        let i = g as usize;
        out[i] = c.op(14, out[i], e);
        rows += 1;
    });
    let checksum = output_checksum(&c.inner, &out);
    (out, checksum, rows)
}
fn bench<C: Carrier>(scenario: &str, shape: &str) {
    let trials = std::env::var("EVENT_LAB_TRIALS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7);
    let (n, bank) = workload(scenario);
    let support = bits(n, |_| true);
    let groups = 64;
    let fanout = 4;
    let input_rows = native::data(shape, groups, fanout, bank.len());
    let (expected, expected_rows) = native::oracle(&input_rows, shape, &bank, groups);
    let expected_checksum: u64 = expected
        .iter()
        .enumerate()
        .map(|(i, b)| b.iter().map(|w| w.count_ones() as u64).sum::<u64>() * (i + 1) as u64)
        .sum();
    let mut builds = Vec::new();
    let mut prepared = None;
    for _ in 0..trials {
        let start = Instant::now();
        let mut c = C::new(&support, n);
        let ids: Vec<_> = bank.iter().map(|b| c.import(b)).collect();
        let elapsed = start.elapsed().as_secs_f64();
        builds.push(elapsed);
        prepared = Some((c, ids));
    }
    let (c, ids) = prepared.unwrap();
    let input_bytes = c.bytes();
    let input_nodes = c.nodes();
    let data: [Vec<native::Row>; 3] = input_rows.each_ref().map(|rows| {
        rows.iter()
            .map(|r| [r[0], r[1], r[2], ids[r[3] as usize]])
            .collect()
    });
    let native_start = Instant::now();
    let mut engine = native::Native::new(&data, shape);
    let setup = native_start.elapsed().as_secs_f64();
    let start = Instant::now();
    let mut count = 0;
    engine.run(|r| {
        black_box(r);
        count += 1;
    });
    let first_join = start.elapsed().as_secs_f64();
    assert_eq!(count, expected_rows);
    let mut baselines = Vec::new();
    for _ in 0..trials {
        let start = Instant::now();
        let mut count = 0;
        engine.run(|r| {
            black_box(r);
            count += 1;
        });
        baselines.push(start.elapsed().as_secs_f64());
        assert_eq!(count, expected_rows);
    }
    for cached in [false, true] {
        let input = Memo::new(c.clone(), cached);
        let mut cold = Vec::new();
        let mut last = None;
        for i in 0..trials {
            let mut current = input.clone();
            let start = Instant::now();
            let (out, checksum, rows) = query(&mut engine, &mut current, groups);
            cold.push(start.elapsed().as_secs_f64());
            black_box(checksum);
            assert_eq!((checksum, rows), (expected_checksum, expected_rows));
            if i == 0 {
                for (e, b) in out.iter().zip(&expected) {
                    assert_eq!(current.inner.export(*e), *b, "full grouped result");
                }
            }
            last = Some(current);
        }
        let mut current = last.unwrap();
        let mut warm = Vec::new();
        let mut loops = 1;
        loop {
            let start = Instant::now();
            for _ in 0..loops {
                black_box(query(&mut engine, &mut current, groups).1);
            }
            if start.elapsed().as_secs_f64() >= 0.02 || loops >= 4096 {
                break;
            }
            loops *= 2;
        }
        for _ in 0..trials {
            let start = Instant::now();
            let mut checksum = 0;
            for _ in 0..loops {
                checksum = query(&mut engine, &mut current, groups).1;
                black_box(checksum);
            }
            warm.push(start.elapsed().as_secs_f64() / loops as f64);
            assert_eq!(checksum, expected_checksum);
        }
        let (out, _, _) = query(&mut engine, &mut current, groups);
        for (e, b) in out.iter().zip(&expected) {
            assert_eq!(current.inner.export(*e), *b);
        }
        println!(
            "EVENT_LAB {{\"kind\":\"free_join\",\"candidate\":\"{}\",\"scenario\":\"{}\",\"shape\":\"{}\",\"memo\":{},\"worlds\":{},\"rows\":{},\"groups\":{},\"plan_nodes\":{},\"input_nodes\":{},\"final_nodes\":{},\"input_bytes_est\":{},\"final_bytes_est\":{},\"colt_bytes\":{},\"build_s\":{:?},\"native_setup_s\":{},\"native_first_join_s\":{},\"join_only_s\":{:?},\"fresh_s\":{:?},\"warm_s\":{:?},\"warm_loops\":{},\"checksum\":{},\"verified\":true}}",
            C::NAME,
            scenario,
            shape,
            cached,
            n,
            expected_rows,
            groups,
            engine.plan_nodes(),
            input_nodes,
            current.inner.nodes(),
            input_bytes,
            current.bytes(),
            engine.retained_colt_bytes(),
            builds,
            setup,
            first_join,
            baselines,
            cold,
            warm,
            loops,
            expected_checksum
        );
    }
}
fn elimination<C: Carrier>(nbits: u32) {
    let n = 1usize << nbits;
    let width = nbits / 3;
    let side = (1usize << width) - 1;
    let mask = (side << width) as u32;
    let ra = bits(n, |w| {
        let (x, y) = (w & side, (w >> width) & side);
        y == x || y == (x + 1) & side
    });
    let qb = bits(n, |w| {
        let (y, z) = ((w >> width) & side, (w >> (2 * width)) & side);
        z == (3 * y) & side || z == (y + 2) & side
    });
    let mut c = C::new(&bits(n, |_| true), n);
    let r = c.import(&ra);
    let q = c.import(&qb);
    let mut expected = ra.iter().zip(&qb).map(|(&a, &b)| a & b).collect::<Vec<_>>();
    abstract_dense(&mut expected, n, mask);
    let mut timings = Vec::new();
    let mut final_c = None;
    for _ in 0..7 {
        let mut fresh = c.clone();
        let start = Instant::now();
        let out = fresh.relprod(r, q, mask);
        black_box(fresh.count(out));
        timings.push(start.elapsed().as_secs_f64());
        assert_eq!(fresh.export(out), expected);
        final_c = Some(fresh);
    }
    let current = final_c.unwrap();
    println!(
        "EVENT_LAB {{\"kind\":\"elimination\",\"candidate\":\"{}\",\"bits\":{},\"worlds\":{},\"fresh_s\":{:?},\"nodes\":{},\"bytes_est\":{},\"verified\":true}}",
        C::NAME,
        nbits,
        n,
        timings,
        current.nodes(),
        current.bytes()
    );
}
fn symbolic<const N: usize, const P: bool>(pairs: u32, interleaved: bool) {
    let bit_order = if interleaved {
        (0..pairs).flat_map(|i| [i, pairs + i]).collect::<Vec<_>>()
    } else {
        (0..pairs * 2).collect()
    };
    if N == 4 && interleaved {
        return;
    } // Native 2-bit groups, no false claim of arbitrary MDD grouping.
    let order = if N == 4 {
        (0..pairs * 2).step_by(2).collect()
    } else {
        bit_order
    };
    let start = Instant::now();
    let mut c = Diagram::<N, P>::symbolic(2 * pairs, order);
    let mut event = c.full();
    for i in 0..pairs {
        let a = c.literal(i);
        let b = c.literal(pairs + i);
        let eq = c.op(9, a, b);
        event = c.op(8, event, eq);
    }
    let construction = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let count = c.count(event);
    let observation = start.elapsed().as_secs_f64();
    assert_eq!(count, 1u64 << pairs);
    println!(
        "EVENT_LAB {{\"kind\":\"symbolic\",\"candidate\":\"{}\",\"pairs\":{},\"interleaved\":{},\"worlds\":{},\"build_s\":{},\"count_s\":{},\"nodes\":{},\"bytes_est\":{},\"verified\":true}}",
        <Diagram<N, P> as Carrier>::NAME,
        pairs,
        interleaved,
        1u64 << (2 * pairs),
        construction,
        observation,
        c.nodes(),
        c.bytes()
    );
}

macro_rules! dispatch {($name:expr,$f:ident $(,$arg:expr)*)=>{match $name {
    "dense"=>$f::<Finite<Dense>>($($arg),*),"sparse"=>$f::<Finite<Sparse>>($($arg),*),"roaring"=>$f::<Finite<Roaring>>($($arg),*),"runs"=>$f::<Finite<Runs>>($($arg),*),
    "bdd-root"=>$f::<Root>($($arg),*),"bdd-pair"=>$f::<Bdd>($($arg),*),"mdd4-pair"=>$f::<Mdd>($($arg),*),_=>panic!("unknown candidate")
}};}

#[test]
#[ignore]
fn experiment() {
    let name = std::env::var("EVENT_LAB_CANDIDATE").unwrap();
    let lane = std::env::var("EVENT_LAB_LANE").unwrap_or_else(|_| "join".into());
    if lane == "join" {
        let scenario = std::env::var("EVENT_LAB_SCENARIO").unwrap();
        for shape in ["triangle", "clover"] {
            dispatch!(name.as_str(), bench, &scenario, shape);
        }
    } else if lane == "elimination" {
        for nbits in [12, 18] {
            dispatch!(name.as_str(), elimination, nbits);
        }
    } else {
        let pairs = std::env::var("EVENT_LAB_PAIRS").unwrap().parse().unwrap();
        let interleaved = std::env::var("EVENT_LAB_ORDER").unwrap() == "interleaved";
        match name.as_str() {
            "bdd-root" => symbolic::<2, false>(pairs, interleaved),
            "bdd-pair" => symbolic::<2, true>(pairs, interleaved),
            "mdd4-pair" => symbolic::<4, true>(pairs, interleaved),
            _ => panic!("symbolic lane requires diagram"),
        }
    }
}
