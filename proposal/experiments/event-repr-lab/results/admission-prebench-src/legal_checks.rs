use super::carrier::*;
use super::legal::{Checked, Domain, Domains, Relation};

fn words<C: RegionOps>(c: &Checked<C>, r: Relation) -> Vec<u64> {
    c.carrier().export(c.root(r).unwrap())
}
fn matrix_words(domains: &Domains, matrices: &[Vec<bool>]) -> Vec<u64> {
    let width = domains.width();
    let size = 1usize << width;
    bits(1usize << domains.dimensions(), |w| {
        domains.contains(w as u64)
            && matrices[w >> (3 * width)][(w & (size - 1)) * size + ((w >> width) & (size - 1))]
    })
}
fn compose(domains: &Domains, a: &[Vec<bool>], b: &[Vec<bool>]) -> Vec<Vec<bool>> {
    let size = 1usize << domains.width();
    (0..domains.environments())
        .map(|e| {
            (0..size * size)
                .map(|i| {
                    let (x, z) = (i / size, i % size);
                    let d = domains.domain(e);
                    d.contains(x as u64)
                        && d.contains(z as u64)
                        && (0..size).any(|y| {
                            d.contains(y as u64) && a[e][x * size + y] && b[e][y * size + z]
                        })
                })
                .collect()
        })
        .collect()
}
fn residual(domains: &Domains, a: &[Vec<bool>], t: &[Vec<bool>]) -> Vec<Vec<bool>> {
    let size = 1usize << domains.width();
    (0..domains.environments())
        .map(|e| {
            (0..size * size)
                .map(|i| {
                    let (y, z) = (i / size, i % size);
                    let d = domains.domain(e);
                    d.contains(y as u64)
                        && d.contains(z as u64)
                        && (0..size).all(|x| {
                            !d.contains(x as u64) || !a[e][x * size + y] || t[e][x * size + z]
                        })
                })
                .collect()
        })
        .collect()
}

pub fn finite<C: Carrier>() {
    assert!(Domains::new(2, 0, vec![Domain::Values(vec![1, 1])]).is_err());
    assert!(Domains::new(2, 0, vec![Domain::Below(5)]).is_err());
    assert!(Domains::new(2, 1, vec![Domain::Below(3)]).is_err());
    assert!(Domains::new(21, 0, vec![Domain::Below(1)]).is_err());
    assert!(Domains::new(2, 0, vec![Domain::Below(0)]).is_err());
    let mut admission = 0;
    for support in 1..256 {
        for domain in [Domain::Below(1), Domain::Below(2), Domain::Values(vec![1])] {
            let domains = Domains::new(1, 0, vec![domain]).unwrap();
            let mut c = C::new(&[support], 8);
            let exact = bits(8, |w| domains.contains(w as u64))[0] == support;
            assert_eq!(
                domains.verify(&mut c).is_ok(),
                exact,
                "{} exact legal admission",
                C::NAME
            );
            admission += 1;
        }
    }
    let configurations = [
        Domains::new(2, 0, vec![Domain::Below(4)]).unwrap(),
        Domains::new(2, 0, vec![Domain::Below(3)]).unwrap(),
        Domains::new(2, 0, vec![Domain::Values(vec![1, 3])]).unwrap(),
        Domains::new(2, 1, vec![Domain::Below(3), Domain::Below(2)]).unwrap(),
        Domains::new(
            2,
            1,
            vec![Domain::Values(vec![1, 3]), Domain::Values(vec![0, 2, 3])],
        )
        .unwrap(),
        Domains::new(2, 1, vec![Domain::Below(0), Domain::Below(3)]).unwrap(),
    ];
    let mut pairs = 0;
    for domains in &configurations {
        for layout in ["face-major", "bit-major"] {
            for gates in if C::VIEW_PRODUCT {
                vec!["gated", "certified"]
            } else {
                vec!["gated"]
            } {
                let mut c = C::legal_space(domains, domains.order(layout)).unwrap();
                let n = 1usize << domains.dimensions();
                assert_eq!(c.export(c.full()), bits(n, |w| domains.contains(w as u64)));
                let mut explicit = C::with_order(&c.export(c.full()), n, domains.order(layout));
                assert!(domains.verify(&mut explicit).is_ok());
                let matrices: Vec<Vec<Vec<bool>>> = (0..12)
                    .map(|seed| {
                        (0..domains.environments())
                            .map(|e| {
                                (0..16)
                                    .map(|i| {
                                        domains.domain(e).contains((i / 4) as u64)
                                            && domains.domain(e).contains((i % 4) as u64)
                                            && match seed {
                                                0 => false,
                                                1 => true,
                                                2 => i / 4 == i % 4,
                                                _ => mix((seed * 137 + e * 41 + i) as u64) % 3 != 0,
                                            }
                                    })
                                    .collect()
                            })
                            .collect()
                    })
                    .collect();
                let ids: Vec<_> = matrices
                    .iter()
                    .map(|m| c.import(&matrix_words(domains, m)))
                    .collect();
                let scratch = c.variable(5);
                let wrong_goal = c.variable(1);
                let goal = c.variable(2);
                let environment = if domains.environment_bits() > 0 {
                    Some(c.variable(6))
                } else {
                    None
                };
                let mut p = Checked::new(c, domains, true, gates).unwrap();
                assert!(p.admit(scratch).is_err());
                assert!(p.goal(wrong_goal).is_err());
                assert!(p.goal(goal).is_ok());
                if let Some(e) = environment {
                    assert!(p.admit(e).is_ok());
                    assert!(p.goal(e).is_ok());
                }
                let rs: Vec<_> = ids.iter().map(|&id| p.admit(id).unwrap()).collect();
                let mut clone = p.clone();
                let empty = clone.empty();
                assert!(clone.compose(empty, rs[0]).is_err());
                assert!(clone.boolean(0, empty, rs[0]).is_err());
                assert!(clone.root(rs[0]).is_err());
                for (i, r) in matrices.iter().enumerate() {
                    let out = p.compose(rs[i], rs[2]).unwrap();
                    assert_eq!(out, rs[i]);
                    let out = p.compose(rs[2], rs[i]).unwrap();
                    assert_eq!(out, rs[i]);
                    let converse = p.converse(rs[i]).unwrap();
                    let transposed: Vec<Vec<bool>> = r
                        .iter()
                        .map(|m| (0..16).map(|k| m[(k % 4) * 4 + k / 4]).collect())
                        .collect();
                    assert_eq!(words(&p, converse), matrix_words(domains, &transposed));
                    assert_eq!(p.converse(converse).unwrap(), rs[i]);
                    let closure = p.closure(rs[i]).unwrap().0;
                    let mut closed = r.clone();
                    for e in 0..domains.environments() {
                        for x in 0..4 {
                            closed[e][x * 4 + x] = domains.domain(e).contains(x as u64);
                        }
                        for y in 0..4 {
                            for x in 0..4 {
                                for z in 0..4 {
                                    closed[e][x * 4 + z] |=
                                        closed[e][x * 4 + y] && closed[e][y * 4 + z];
                                }
                            }
                        }
                    }
                    assert_eq!(words(&p, closure), matrix_words(domains, &closed));
                    for (j, q) in matrices.iter().enumerate() {
                        let out = p.compose(rs[i], rs[j]).unwrap();
                        assert_eq!(
                            words(&p, out),
                            matrix_words(domains, &compose(domains, r, q)),
                            "{} composition",
                            C::NAME
                        );
                        let safe = p.residual(rs[i], rs[j]).unwrap();
                        assert_eq!(
                            words(&p, safe),
                            matrix_words(domains, &residual(domains, r, q)),
                            "{} residual",
                            C::NAME
                        );
                        let test = rs[(i * 7 + j * 3) % rs.len()];
                        let rq = p.compose(rs[i], test).unwrap();
                        let left = p.boolean(4, rq, rs[j]).unwrap();
                        let right = p.boolean(4, test, safe).unwrap();
                        let empty = p.empty();
                        assert_eq!(left == empty, right == empty);
                        for op in 0..16 {
                            let result = p.boolean(op, rs[i], rs[j]).unwrap();
                            let a = words(&p, rs[i]);
                            let b = words(&p, rs[j]);
                            let s = p.carrier().export(p.carrier().full());
                            let expected: Vec<_> = a
                                .iter()
                                .zip(&b)
                                .zip(s)
                                .map(|((&a, &b), s)| binary(op, a, b) & s)
                                .collect();
                            assert_eq!(words(&p, result), expected);
                        }
                        pairs += 1;
                    }
                }
                // A borrowed carrier uses the same checked role boundary.
                let mut borrowed = explicit;
                let full = borrowed.full();
                let mut p = Checked::new(&mut borrowed, domains, false, gates).unwrap();
                let r = p.admit(full).unwrap();
                let composed = p.compose(r, r).unwrap();
                assert_eq!(composed, r);
            }
        }
    }
    println!(
        "EVENT_LAB {{\"kind\":\"legal_relation_verification\",\"candidate\":\"{}\",\"admission_cases\":{},\"relation_pairs\":{},\"passed\":true}}",
        C::NAME,
        admission,
        pairs
    );
}

pub fn symbolic<C: Carrier>() {
    let width = 20;
    let d = Domains::new(
        width,
        1,
        vec![
            Domain::Below((1 << width) - 3),
            Domain::Below((1 << width) - 5),
        ],
    )
    .unwrap();
    for gates in if C::VIEW_PRODUCT {
        vec!["gated", "certified"]
    } else {
        vec!["gated"]
    } {
        let mut c = C::legal_space(&d, d.order("bit-major")).unwrap();
        let r = super::diagonal_checks::counter_inputs(&mut c, width)[0];
        let expected = super::diagonal_checks::counter_outputs(&mut c, width);
        let identity = c
            .diagonal(&(0..width).map(|i| (i, width + i)).collect::<Vec<_>>())
            .unwrap();
        let mut p = Checked::new(c, &d, true, gates).unwrap();
        let r = p.admit(r).unwrap();
        let id = p.admit(identity).unwrap();
        assert_eq!(p.compose(r, id).unwrap(), r);
        assert_eq!(p.compose(id, r).unwrap(), r);
        let reach = p.closure(r).unwrap().0;
        assert_eq!(p.root(reach).unwrap(), expected[0]);
        let reverse = p.converse(reach).unwrap();
        assert_eq!(p.root(reverse).unwrap(), expected[1]);
        let count: u64 = [(1u64 << width) - 3, (1u64 << width) - 5]
            .iter()
            .map(|&n| (n * (n + 1) / 2) * n)
            .sum();
        assert_eq!(p.carrier().count(p.root(reach).unwrap()), count);
        let empty = p.empty();
        let full = p.full();
        assert_eq!(p.residual(empty, r).unwrap(), full);
    }
    println!(
        "EVENT_LAB {{\"kind\":\"legal_symbolic_verification\",\"candidate\":\"{}\",\"coordinates\":61,\"population\":{},\"passed\":true}}",
        C::NAME,
        d.population()
    );
}
