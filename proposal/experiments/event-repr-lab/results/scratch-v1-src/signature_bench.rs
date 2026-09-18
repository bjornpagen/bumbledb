//! Relationship filtering over real Free Join bindings in a retained owner.
//! This is a computed-stage sink, not a planner-pushed native Event residual.
use super::signature::{Cache, Mode};
use super::transfer::Space;
use super::*;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
struct Answer {
    histogram: [usize; 16],
    groups: Vec<usize>,
    rows: usize,
}

fn query<C: Carrier>(
    engine: &mut native::Native,
    c: &mut C,
    scope: u64,
    groups: usize,
    cache: &mut Cache,
    mode: Mode,
    keep: &[u8; 16],
) -> Answer {
    let mut out = Answer {
        histogram: [0; 16],
        groups: vec![0; groups],
        rows: 0,
    };
    engine.run(|[g, token, a, b, _]| {
        assert_eq!(
            token, scope,
            "native binding must retain its validated scope"
        );
        let sig = cache.classify(scope, c, a, b, mode);
        out.histogram[sig as usize] += 1;
        out.groups[g as usize] += keep[sig as usize] as usize;
        out.rows += 1;
    });
    out
}

// Independent per-supported-world oracle. It neither imports Event handles nor
// calls the word/traversal classifiers used by the timed candidates.
fn pair_oracle(support: &[u64], bank: &[Vec<u64>], n: usize) -> Vec<(u8, [bool; 3])> {
    let worlds: Vec<_> = (0..n).filter(|&w| at(support, w)).collect();
    let mut pairs = Vec::with_capacity(bank.len() * bank.len());
    for a in bank {
        for b in bank {
            let mut signature = 0;
            let (mut included, mut disjoint, mut overlap) = (true, true, false);
            for &w in &worlds {
                let (a, b) = (at(a, w), at(b, w));
                signature |= 1 << (2 * a as u8 + b as u8);
                included &= !a || b;
                disjoint &= !(a && b);
                overlap |= a && b;
            }
            pairs.push((signature, [included, disjoint, overlap]));
        }
    }
    pairs
}
fn oracle(
    data: &[Vec<native::Row>; 3],
    shape: &str,
    pairs: &[(u8, [bool; 3])],
    bank: usize,
    groups: usize,
    predicate: usize,
) -> Answer {
    let mut out = Answer {
        histogram: [0; 16],
        groups: vec![0; groups],
        rows: 0,
    };
    for a in &data[0] {
        for b in &data[1] {
            if (if shape == "triangle" { a[1] } else { a[0] }) != b[0] {
                continue;
            }
            for c in &data[2] {
                if c[0] != a[0] || (shape == "triangle" && c[1] != b[1]) {
                    continue;
                }
                let (sig, keep) = pairs[a[3] as usize * bank + b[3] as usize];
                out.histogram[sig as usize] += 1;
                out.groups[a[0] as usize] += keep[predicate] as usize;
                out.rows += 1;
            }
        }
    }
    out
}

pub fn bench<C: Carrier>(scenario: &str, shape: &str) {
    let mode = match std::env::var("EVENT_LAB_CLASSIFY").as_deref() {
        Ok("direct") => {
            assert!(C::DIRECT_SIGNATURE);
            Mode::Direct
        }
        Ok("cells") | Err(_) => Mode::Cells,
        _ => panic!("unknown classifier algorithm"),
    };
    let (n, mut bank) = workload(scenario);
    let support = scenario_support(scenario, n);
    let complements: Vec<_> = bank
        .iter()
        .map(|r| support.iter().zip(r).map(|(&s, &r)| s & !r).collect())
        .collect();
    bank.extend(complements);
    bank.push(vec![0; support.len()]);
    bank.push(support.clone());
    let groups = 64;
    let input_rows = native::data(shape, groups, 4, bank.len());
    let pairs = pair_oracle(&support, &bank, n);
    let mut c = C::new(&support, n);
    let ids: Vec<_> = bank.iter().map(|r| c.import(r)).collect();
    for (predicate, name) in ["included", "disjoint", "overlap"].into_iter().enumerate() {
        let keep = signature::keep_table(name);
        let expected = oracle(&input_rows, shape, &pairs, bank.len(), groups, predicate);
        assert_eq!(expected.histogram[0], 0);
        if scenario == "ordered_65536" {
            assert_eq!(expected.histogram[15], 0, "ordered cuts forbid full occupancy");
        }
        assert!(expected.groups.iter().sum::<usize>() > 0);
        assert!(expected.groups.iter().sum::<usize>() < expected.rows);
        for cached in [false, true] {
            let (mut fresh, mut warm, mut joins, mut setups) = (vec![], vec![], vec![], vec![]);
            let (mut final_bytes, mut final_nodes, mut cache_bytes, mut cache_entries) =
                (0, 0, 0, 0);
            let mut warm_loops = 0;
            let mut input_stats = None;
            let mut final_storage = None;
            for _ in 0..trials() {
                // Only the input arena is cloned; no expected/signature roots.
                let owner = Space::new(c.clone(), (0..c.dimensions() as u64).collect());
                let inputs = owner.publish(&ids);
                let scope = inputs.keys()[0].space;
                let weak = Arc::downgrade(&owner);
                // Clone may shrink Vec capacity. Measure the actual retained
                // query arena, not its construction template's spare capacity.
                let before = owner
                    .inspect_inputs(inputs.keys(), |c, _| (c.nodes(), c.bytes(), c.memory_stats()))
                    .unwrap();
                if let Some(previous) = input_stats {
                    assert_eq!(before, previous);
                }
                input_stats = Some(before);
                let data = input_rows.each_ref().map(|rows| {
                    rows.iter()
                        .map(|r| {
                            let key = inputs.keys()[r[3] as usize];
                            [r[0], r[1], key.space, key.region]
                        })
                        .collect()
                });
                let begin = Instant::now();
                let mut engine = native::Native::new(&data, shape);
                setups.push(begin.elapsed().as_secs_f64());
                let begin = Instant::now();
                let rows = owner
                    .inspect_inputs(inputs.keys(), |_, _| {
                        let mut rows = 0;
                        engine.run(|r| {
                            black_box(r);
                            rows += 1;
                        });
                        rows
                    })
                    .unwrap();
                joins.push(begin.elapsed().as_secs_f64());
                assert_eq!(rows, expected.rows);
                let mut cache = Cache::new(scope, cached);
                let begin = Instant::now();
                let answer = owner
                    .inspect_inputs(inputs.keys(), |c, scope| {
                        query(&mut engine, c, scope, groups, &mut cache, mode, &keep)
                    })
                    .unwrap();
                fresh.push(begin.elapsed().as_secs_f64());
                assert_eq!(answer, expected, "{} {shape} {name}", C::NAME);
                let mut loops = 1;
                loop {
                    let begin = Instant::now();
                    for _ in 0..loops {
                        black_box(
                            owner
                                .inspect_inputs(inputs.keys(), |c, scope| {
                                    query(&mut engine, c, scope, groups, &mut cache, mode, &keep)
                                })
                                .unwrap(),
                        );
                    }
                    if begin.elapsed().as_secs_f64() >= 0.01 || loops >= 1024 {
                        break;
                    }
                    loops *= 2;
                }
                warm_loops = loops;
                let begin = Instant::now();
                let mut last = None;
                for _ in 0..loops {
                    last = Some(
                        owner
                            .inspect_inputs(inputs.keys(), |c, scope| {
                                query(&mut engine, c, scope, groups, &mut cache, mode, &keep)
                            })
                            .unwrap(),
                    );
                    black_box(&last);
                }
                warm.push(begin.elapsed().as_secs_f64() / loops as f64);
                assert_eq!(last.unwrap(), expected);
                (final_nodes, final_bytes, final_storage) = owner
                    .inspect_inputs(inputs.keys(), |c, _| (c.nodes(), c.bytes(), c.memory_stats()))
                    .unwrap();
                if mode == Mode::Direct {
                    assert_eq!((final_nodes, final_bytes, final_storage), before);
                }
                cache_bytes = cache.bytes();
                cache_entries = cache.entries();
                drop(engine);
                drop(inputs);
                drop(owner);
                assert!(weak.upgrade().is_none());
            }
            let (input_nodes, input_bytes, input_storage) = input_stats.unwrap();
            println!(
                "EVENT_LAB {{\"kind\":\"signature_free_join\",\"candidate\":\"{}\",\"scenario\":\"{}\",\"shape\":\"{}\",\"classifier\":\"{}\",\"predicate\":\"{}\",\"memo\":{},\"rows\":{},\"accepted\":{},\"histogram\":{:?},\"input_nodes\":{},\"final_nodes\":{},\"input_bytes_est\":{},\"final_bytes_est\":{},\"input_storage\":{},\"final_storage\":{},\"signature_cache_bytes_est\":{},\"signature_cache_entries\":{},\"native_setup_s\":{:?},\"join_only_s\":{:?},\"fresh_s\":{:?},\"warm_s\":{:?},\"warm_loops\":{},\"verified\":true}}",
                C::NAME,
                scenario,
                shape,
                mode.name(),
                name,
                cached,
                expected.rows,
                expected.groups.iter().sum::<usize>(),
                expected.histogram,
                input_nodes,
                final_nodes,
                input_bytes,
                final_bytes,
                memory_json(input_storage),
                memory_json(final_storage),
                cache_bytes,
                cache_entries,
                setups,
                joins,
                fresh,
                warm,
                warm_loops
            );
        }
    }
}
