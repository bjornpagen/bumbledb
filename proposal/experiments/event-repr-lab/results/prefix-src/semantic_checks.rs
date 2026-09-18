use super::blocks::Block64;
use super::carrier::*;
use super::diagram::{Anchored, Diagram};
use super::essential::Essential;
use super::finite::*;
use super::packed::Packed;
use super::relational_core;
use super::retraction::Retraction;
type Root = Diagram<2, false>;
type Bdd = Diagram<2, true>;
type Mdd = Diagram<4, true>;

fn verify<C: Carrier>() {
    super::legal_checks::finite::<C>();
    verify_view_product::<C>();
    super::signature::verify::<C>();
    super::diagonal_checks::finite::<C>();
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
            for mask in 0..4u64 {
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
        let raw_a = bits(n, |w| w % 5 < 2);
        let raw_b = bits(n, |w| w % 11 < 5);
        let a = c.import(&raw_a);
        let b = c.import(&raw_b);
        for op in 0..16 {
            let out = c.op(op, a, b);
            let expected: Vec<_> = raw_a
                .iter()
                .zip(&raw_b)
                .zip(&s)
                .map(|((&a, &b), &s)| binary(op, a, b) & s)
                .collect();
            assert_eq!(c.export(out), expected, "{} multiword op {op}", C::NAME);
            assert_eq!(out, c.import(&expected));
        }
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
    if let Some(memory) = c.memory_stats() {
        assert_eq!(memory.bytes(), c.bytes());
        assert_eq!(memory.records, c.nodes());
        assert_eq!((&mut c).memory_stats(), Some(memory));
        let copy = c.clone();
        assert_eq!(copy.memory_stats().unwrap().bytes(), copy.bytes());
        assert_eq!(
            copy.memory_stats().unwrap().logical_words,
            memory.logical_words
        );
    }
    let expected = bits(n, |w| {
        (0..16).any(|y| {
            let v = (w & !0xf0) | (y << 4);
            at(&ra, v) && at(&qb, v)
        })
    });
    assert_eq!(c.export(out), expected);
    let n = 1024;
    let support = bits(n, |w| w % 7 != 0 && ((w >> 9) ^ (w >> 4)) & 1 == 0);
    verify_boundaries(C::new(&support, n), &support, n);
    relational_core::verify::<C>();
    super::observation_checks::verify::<C>();
    super::dependencies::verify::<C>();
    for layout in ["face-major", "bit-major", "pair-major"] {
        relational_core::verify_layout::<C>(layout);
    }
    for order in [(0..10).collect(), vec![9, 0, 8, 1, 7, 2, 6, 3, 5, 4]] {
        verify_boundaries(C::with_order(&support, n, order), &support, n);
    }
    println!(
        "EVENT_LAB {{\"kind\":\"verification\",\"candidate\":\"{}\",\"boolean_cases\":{},\"passed\":true}}",
        C::NAME,
        operations
    );
}

fn verify_view_product<C: Carrier>() {
    if !C::VIEW_PRODUCT {
        return;
    }
    let mut cases = 0;
    for dimensions in [3u32, 6, 9] {
        let n = 1usize << dimensions;
        let maps = [
            Permutation::new((0..dimensions).collect()).unwrap(),
            Permutation::new((0..dimensions).rev().collect()).unwrap(),
            Permutation::new((0..dimensions).map(|v| (v + 2) % dimensions).collect()).unwrap(),
        ];
        for order in [(0..dimensions).collect(), (0..dimensions).rev().collect()] {
            let mut c = C::full_space(dimensions, order).unwrap();
            let bank = [
                bits(n, |w| w.count_ones() % 3 == 1),
                bits(n, |w| (w ^ (w >> 2)) % 5 < 2),
                bits(n, |_| false),
                bits(n, |_| true),
            ];
            let ids: Vec<_> = bank.iter().map(|b| c.import(b)).collect();
            for (i, am) in maps.iter().enumerate() {
                for (j, bm) in maps.iter().enumerate() {
                    let output = &maps[(i + j) % 3];
                    for mask in [0, 1, 5, (1usize << dimensions) - 1] {
                        for (a, b) in [(0, 1), (1, 0), (0, 0), (2, 1), (0, 3)] {
                            let mut witnesses = vec![false; n];
                            for w in 0..n {
                                if at(&bank[a], am.source_world(w))
                                    && at(&bank[b], bm.source_world(w))
                                {
                                    witnesses[w & !mask] = true;
                                }
                            }
                            let expected = bits(n, |w| witnesses[output.source_world(w) & !mask]);
                            let actual = c
                                .view_product(ids[a], am, ids[b], bm, mask as u64, output)
                                .unwrap();
                            assert_eq!(c.export(actual), expected);
                            assert_eq!(actual, c.import(&expected));
                            assert_eq!(
                                Some(actual),
                                (&mut c).view_product(ids[a], am, ids[b], bm, mask as u64, output)
                            );
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    verify_scoped_products::<C>();
    println!(
        "EVENT_LAB {{\"kind\":\"mapped_product_verification\",\"candidate\":\"{}\",\"cases\":{},\"passed\":true}}",
        C::NAME,
        cases
    );
}

fn verify_scoped_products<C: Carrier>() {
    // Exhaust every nonempty two-bit support and every *distinct supported*
    // operand pair, including polarities, all maps, masks and both orders.
    let maps = [
        Permutation::new(vec![0, 1]).unwrap(),
        Permutation::new(vec![1, 0]).unwrap(),
    ];
    let mut cases = 0;
    for support in 1..16u64 {
        for order in [vec![0, 1], vec![1, 0]] {
            let mut c = C::with_order(&[support], 4, order);
            let events: Vec<_> = (0..16u64).filter(|a| a & !support == 0).collect();
            let ids: Vec<_> = events.iter().map(|&a| c.import(&[a])).collect();
            for am in &maps {
                for bm in &maps {
                    for output in &maps {
                        for mask in 0..4usize {
                            for (i, &a) in events.iter().enumerate() {
                                for (j, &b) in events.iter().enumerate() {
                                    let expected = bits(4, |t| {
                                        let v = output.source_world(t);
                                        support >> t & 1 != 0
                                            && support >> v & 1 != 0
                                            && (0..4).any(|w| {
                                                w & !mask == v & !mask
                                                    && support >> w & 1 != 0
                                                    && a >> am.source_world(w) & 1 != 0
                                                    && b >> bm.source_world(w) & 1 != 0
                                            })
                                    });
                                    let actual = c
                                        .view_product(ids[i], am, ids[j], bm, mask as u64, output)
                                        .unwrap();
                                    assert_eq!(
                                        c.export(actual),
                                        expected,
                                        "supported mapped product"
                                    );
                                    assert_eq!(actual, c.import(&expected));
                                    let preserving = [am, output].iter().all(|m| {
                                        (0..4).all(|w| {
                                            ((support >> w) & 1)
                                                == ((support >> m.source_world(w)) & 1)
                                        })
                                    });
                                    assert_eq!(
                                        c.preserving_product(
                                            ids[i],
                                            am,
                                            ids[j],
                                            bm,
                                            mask as u64,
                                            output
                                        ),
                                        preserving.then_some(actual)
                                    );
                                    let pa = c.permute(ids[i], am);
                                    let pb = c.permute(ids[j], bm);
                                    let product = c.relprod(pa, pb, mask as u64);
                                    assert_eq!(actual, c.permute(product, output));
                                    cases += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // Three-cycles distinguish a map from its inverse; asymmetric support
    // exercises both clipping gates. Larger cubes cross local-table cutoffs.
    for dimensions in [3u32, 9, 12] {
        let n = 1usize << dimensions;
        let am = Permutation::new((0..dimensions).map(|v| (v + 1) % dimensions).collect()).unwrap();
        let bm = Permutation::new((0..dimensions).map(|v| (v + 2) % dimensions).collect()).unwrap();
        let output = Permutation::new((0..dimensions).rev().collect()).unwrap();
        let supports = [
            bits(n, |w| w % 5 != 0 && w & 3 != 3),
            bits(n, |w| (w & 1) == ((w >> 2) & 1)),
        ];
        for support in &supports {
            for order in [(0..dimensions).collect(), (0..dimensions).rev().collect()] {
                let mut c = C::with_order(support, n, order);
                let bank = [
                    bits(n, |w| w.count_ones() % 3 == 1),
                    bits(n, |w| (w ^ (w >> 1)) % 7 < 3),
                ];
                let ids = bank.each_ref().map(|v| c.import(v));
                for polarity in 0..4 {
                    let a = if polarity & 1 == 0 {
                        ids[0]
                    } else {
                        c.not(ids[0])
                    };
                    let b = if polarity & 2 == 0 {
                        ids[1]
                    } else {
                        c.not(ids[1])
                    };
                    for mask in [0usize, 1, 5, n - 1] {
                        let mut witnesses = vec![false; n];
                        for w in 0..n {
                            let x = am.source_world(w);
                            let y = bm.source_world(w);
                            if at(support, w)
                                && at(support, x)
                                && at(support, y)
                                && (at(&bank[0], x) ^ (polarity & 1 != 0))
                                && (at(&bank[1], y) ^ (polarity & 2 != 0))
                            {
                                witnesses[w & !mask] = true;
                            }
                        }
                        let expected = bits(n, |t| {
                            let v = output.source_world(t);
                            at(support, t) && at(support, v) && witnesses[v & !mask]
                        });
                        let actual = c
                            .view_product(a, &am, b, &bm, mask as u64, &output)
                            .unwrap();
                        assert_eq!(c.export(actual), expected);
                        assert_eq!(actual, c.import(&expected));
                        let pa = c.permute(a, &am);
                        let pb = c.permute(b, &bm);
                        let q = c.relprod(pa, pb, mask as u64);
                        assert_eq!(actual, c.permute(q, &output));
                        cases += 1;
                    }
                }
            }
        }
    }
    println!(
        "EVENT_LAB {{\"kind\":\"scoped_product_verification\",\"candidate\":\"{}\",\"cases\":{},\"passed\":true}}",
        C::NAME,
        cases
    );
}

fn verify_boundaries<C: Carrier>(mut c: C, support: &[u64], n: usize) {
    let width = n.trailing_zeros();
    let maps = [
        (0..width)
            .map(|k| {
                if k == 1 {
                    5
                } else if k == 5 {
                    1
                } else {
                    k
                }
            })
            .collect(),
        (0..width).collect(),
        (0..width).rev().collect(),
        (0..width).map(|k| (k + 3) % width).collect(),
        (0..width)
            .map(|k| {
                if k == 0 {
                    9
                } else if k == 9 {
                    0
                } else {
                    k
                }
            })
            .collect(),
        (0..width)
            .map(|k| {
                if k == 6 {
                    9
                } else if k == 9 {
                    6
                } else {
                    k
                }
            })
            .collect(),
    ]
    .map(|m| Permutation::new(m).unwrap());
    for seed in 0..8u64 {
        let a = bits(n, |w| mix(w as u64 + 131 * seed) % 5 < 2 && at(support, w));
        let b = bits(n, |w| mix(w as u64 + 997 * seed) % 7 < 2 && at(support, w));
        let ai = c.import(&a);
        let bi = c.import(&b);
        for op in 0..16 {
            let out = c.op(op, ai, bi);
            let expected: Vec<_> = a
                .iter()
                .zip(&b)
                .zip(support)
                .map(|((&a, &b), &s)| binary(op, a, b) & s)
                .collect();
            assert_eq!(
                c.export(out),
                expected,
                "{} constrained boundary op {op}",
                C::NAME
            );
            assert_eq!(
                c.count(out),
                expected.iter().map(|w| w.count_ones() as u64).sum()
            );
            assert_eq!(out, c.import(&expected));
        }
        for mask in [0, 1, 1 << 5, 1 << 6, 1 << 7, 1 << 8, 1 << 9, 0x120, 0x3ff] {
            for product in [false, true] {
                let mut witnesses = vec![false; n];
                for w in 0..n {
                    if at(&a, w) && (!product || at(&b, w)) {
                        witnesses[w & !mask] = true;
                    }
                }
                let expected = bits(n, |w| at(support, w) && witnesses[w & !mask]);
                let out = if product {
                    c.relprod(ai, bi, mask as u64)
                } else {
                    c.exists(ai, mask as u64)
                };
                assert_eq!(
                    c.export(out),
                    expected,
                    "{} projection mask {mask} product {product}",
                    C::NAME
                );
                assert_eq!(out, c.import(&expected));
            }
        }
        for map in &maps {
            let expected = bits(n, |w| at(support, w) && at(&a, map.source_world(w)));
            let out = c.permute(ai, map);
            assert_eq!(
                c.export(out),
                expected,
                "{} coordinate permutation",
                C::NAME
            );
            assert_eq!(out, c.import(&expected));
        }
    }
}

pub fn all() {
    super::retraction_checks::all();
    super::legal_checks::symbolic::<Retraction<6>>();
    super::legal_checks::symbolic::<Retraction<9>>();
    super::legal_checks::symbolic::<Retraction<6, true>>();
    super::legal_checks::symbolic::<Retraction<9, true>>();
    super::diagonal_checks::symbolic::<Root>();
    super::diagonal_checks::symbolic::<Bdd>();
    super::diagonal_checks::symbolic::<Mdd>();
    super::diagonal_checks::symbolic::<Anchored>();
    super::diagonal_checks::symbolic::<Packed<1>>();
    super::diagonal_checks::symbolic::<Packed<4>>();
    super::diagonal_checks::symbolic::<Packed<8>>();
    super::diagonal_checks::symbolic::<Packed<64>>();
    super::diagonal_checks::symbolic::<Block64>();
    super::diagonal_checks::symbolic::<Essential<6>>();
    super::diagonal_checks::symbolic::<Essential<9>>();
    super::legal_checks::symbolic::<Essential<6>>();
    super::legal_checks::symbolic::<Essential<9>>();
    super::legal_checks::symbolic::<Packed<8>>();
    super::transfer_checks::all();
    verify::<Finite<Dense>>();
    verify::<Finite<Dense<true>>>();
    verify::<Finite<Sparse>>();
    verify::<Finite<Roaring>>();
    verify::<Finite<Runs>>();
    verify::<Root>();
    verify::<Bdd>();
    verify::<Mdd>();
    verify::<Anchored>();
    verify::<Packed<1>>();
    verify::<Packed<4>>();
    verify::<Packed<8>>();
    verify::<Packed<64>>();
    verify::<Block64>();
    verify::<Essential<6>>();
    verify::<Essential<9>>();
    verify::<Retraction<6>>();
    verify::<Retraction<9>>();
    verify::<Retraction<6, true>>();
    verify::<Retraction<9, true>>();
    macro_rules! high_coordinates {
        ($constructor:expr) => {{
            let mut c = $constructor;
            let x = c.literal(39);
            let y = c.literal(4);
            let e = c.op(9, x, y);
            assert_eq!(c.exists(e, 1u64 << 39), c.full());
            let neg = c.not(x);
            let out = c.relprod(e, neg, 1u64 << 39);
            assert_eq!(out, c.not(y));
            assert_eq!(c.count(out), 1u64 << 39);
            assert_eq!(c.exists(out, (1u64 << 39) | (1 << 4)), c.full());
            let map = Permutation::new(
                (0..40)
                    .map(|k| {
                        if k == 39 {
                            4
                        } else if k == 4 {
                            39
                        } else {
                            k
                        }
                    })
                    .collect(),
            )
            .unwrap();
            assert_eq!(c.permute(x, &map), y);
            assert_eq!(c.permute(e, &map), e);
            let mut all_heads = c.full();
            for bit in 0..40 {
                let head = c.literal(bit);
                all_heads = c.op(8, all_heads, head);
            }
            super::observation_checks::high(c, all_heads, x);
        }};
    }
    high_coordinates!(Root::symbolic(40, (0..40).rev().collect()));
    high_coordinates!(Bdd::symbolic(40, (0..40).rev().collect()));
    high_coordinates!(Mdd::symbolic(40, (0..20).rev().map(|i| i * 2).collect()));
    high_coordinates!(Anchored::symbolic(40, (0..40).rev().collect()));
    high_coordinates!(Packed::<1>::symbolic(40, (0..40).rev().collect()));
    high_coordinates!(Packed::<4>::symbolic(40, (0..40).rev().collect()));
    high_coordinates!(Packed::<8>::symbolic(40, (0..40).rev().collect()));
    high_coordinates!(Packed::<64>::symbolic(40, (0..40).rev().collect()));
    high_coordinates!(Block64::symbolic(40, (0..40).rev().collect()));
    high_coordinates!(Essential::<6>::symbolic(40, (0..40).rev().collect()));
    high_coordinates!(Essential::<9>::symbolic(40, (0..40).rev().collect()));
    println!(
        "EVENT_LAB {{\"kind\":\"high_coordinate_verification\",\"bits\":40,\"candidates\":11,\"passed\":true}}"
    );
    let odd_mdd = Mdd::symbolic_order(3, vec![2, 0, 1]);
    assert_eq!(odd_mdd.count(odd_mdd.full()), 8);
    assert!(Permutation::new(vec![0, 0]).is_err());
    assert!(Permutation::new(vec![0, 2]).is_err());
    let support = bits(1024, |w| w % 7 != 0 && ((w >> 9) ^ (w >> 4)) & 1 == 0);
    for order in [(0..10).collect(), vec![9, 0, 8, 1, 7, 2, 6, 3, 5, 4]] {
        verify_boundaries(
            Packed::<1>::with_order(&support, 1024, order.clone()),
            &support,
            1024,
        );
        verify_boundaries(
            Packed::<4>::with_order(&support, 1024, order),
            &support,
            1024,
        );
    }
    println!(
        "EVENT_LAB {{\"kind\":\"coordinate_boundary_verification\",\"candidates\":16,\"extra_orders_per_candidate\":2,\"face_layouts_per_candidate\":3,\"passed\":true}}"
    );
    println!(
        "EVENT_LAB {{\"kind\":\"observation_verification\",\"candidates\":16,\"parameter_groups\":[1,2,3],\"high_coordinate_candidates\":11,\"passed\":true}}"
    );
}
