//! Direct conditional construction against explicit truth tables, independently
//! of the staged Boolean implementation and decoder-normalization traversal.
use super::carrier::{bits, mix};
use super::essential_raw::{Arena, axes};

fn small<const K: u32>() {
    let mut count = 0;
    for order in [vec![0, 1], vec![1, 0]] {
        let mut a = Arena::<K>::new(2, &order);
        let roots: Vec<_> = (0..16).map(|f| a.table(3, vec![f])).collect();
        for s in 0..16usize {
            for h in 0..16usize {
                for l in 0..16usize {
                    let expected = (s & h) | ((!s & l) & 15);
                    let got = a.ite(roots[s], roots[h], roots[l]);
                    assert_eq!(got, roots[expected], "exhaustive conditional identity");
                    count += 1;
                }
            }
        }
    }
    println!(
        "EVENT_LAB {{\"kind\":\"ite_small_verification\",\"cutoff\":{K},\"cases\":{count},\"passed\":true}}"
    );
}

fn scattered<const K: u32>() {
    // Twelve logical axes spread across a 61-coordinate owner. Each operand
    // uses a different overlapping nine-axis set; the result can cross cutoff.
    let coordinates = [0, 2, 5, 8, 13, 21, 27, 32, 39, 45, 54, 60];
    let mask = coordinates.iter().fold(0, |m, &v| m | (1u64 << v));
    let scatter = |i: usize, names: &[u32]| {
        names
            .iter()
            .enumerate()
            .fold(0, |w, (j, &v)| w | (((i >> j) & 1) as u64) << v)
    };
    let mut count = 0;
    for order in [(0..61).collect::<Vec<_>>(), (0..61).rev().collect()] {
        let mut a = Arena::<K>::new(61, &order);
        for seed in 0..48u64 {
            let predicates = [seed * 3 + 1, seed * 3 + 2, seed * 3 + 3];
            let masks: Vec<_> = (0..3)
                .map(|k| {
                    coordinates
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| i % 4 != k)
                        .fold(0, |m, (_, &v)| m | 1u64 << v)
                })
                .collect();
            let mut roots = Vec::new();
            for k in 0..3 {
                let names = axes(masks[k]);
                let data = bits(1 << names.len(), |i| {
                    mix(scatter(i, &names).wrapping_add(predicates[k])) % 7 < 3
                });
                roots.push(a.table(masks[k], data));
            }
            let got = a.ite(roots[0], roots[1], roots[2]);
            let oracle = bits(4096, |i| {
                let w = scatter(i, &coordinates);
                let value = |k: usize| mix((w & masks[k]).wrapping_add(predicates[k])) % 7 < 3;
                if value(0) { value(1) } else { value(2) }
            });
            assert_eq!(
                got,
                a.table(mask, oracle.clone()),
                "scattered canonical root"
            );
            for i in 0..4096 {
                assert_eq!(
                    a.evaluate(got, scatter(i, &coordinates)),
                    oracle[i / 64] >> (i % 64) & 1 != 0
                );
            }
            assert_eq!(a.ite(roots[0] ^ 1, roots[2], roots[1]), got);
            assert_eq!(a.ite(roots[0], roots[1] ^ 1, roots[2] ^ 1), got ^ 1);
            let before = (a.nodes(), a.bytes());
            let graph = a.reachable(&[got]);
            assert_eq!(a.reachable(&[got, got ^ 1, got]), graph);
            assert_eq!((a.nodes(), a.bytes()), before);
            let d = a.variables(got).count_ones();
            let bound = if d <= K {
                1
            } else {
                (1usize << (d - K + 1)) - 1
            };
            assert!(graph.0 <= bound, "essential graph unfolded bound");
            count += 1;
        }
    }
    println!(
        "EVENT_LAB {{\"kind\":\"ite_scattered_verification\",\"cutoff\":{K},\"cases\":{count},\"worlds_per_case\":4096,\"passed\":true}}"
    );
}

pub fn all() {
    small::<1>();
    small::<2>();
    small::<6>();
    small::<9>();
    scattered::<6>();
    scattered::<9>();
}
