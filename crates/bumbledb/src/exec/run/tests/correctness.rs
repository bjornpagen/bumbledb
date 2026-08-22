use super::*;
use crate::ir::WordCmp;

#[test]
fn clover_on_the_papers_instance() {
    let dir = TempDir::new("run-clover");
    let schema = schema(3);
    let n = 20u64;

    let mut r = vec![(0, 100)];
    let mut s = vec![(0, 200)];
    let mut t = vec![(0, 300)];
    for i in 1..=n {
        r.push((1, 100 + i));
        r.push((2, 100 + n + i));
        s.push((2, 200 + i));
        s.push((3, 200 + n + i));
        t.push((3, 300 + i));
        t.push((1, 300 + n + i));
    }
    let views = views_of(&dir, &schema, &[r.clone(), s.clone(), t.clone()]);

    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
            occurrence(2, 2, &[(0, 0), (1, 3)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1, 2]);
    let results = run(&plan, &views);

    let mut expected = BTreeSet::new();
    for (rx, ra) in &r {
        for (sx, sb) in &s {
            for (tx, tc) in &t {
                if rx == sx && sx == tx {
                    expected.insert(vec![*rx, *ra, *sb, *tc]);
                }
            }
        }
    }
    assert_eq!(results, expected);
    assert_eq!(results.len(), 1, "only the center of the clover joins");
}

#[test]
fn chain_query_matches_the_nested_loop_oracle() {
    let dir = TempDir::new("run-chain");
    let schema = schema(3);
    let r: Vec<(u64, u64)> = (0..10).map(|i| (i, i + 1)).collect();
    let s: Vec<(u64, u64)> = (0..10).map(|i| (i + 1, i + 2)).collect();
    let t: Vec<(u64, u64)> = (0..10).map(|i| (i + 2, i + 3)).collect();
    let views = views_of(&dir, &schema, &[r.clone(), s.clone(), t.clone()]);

    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            occurrence(2, 2, &[(0, 2), (1, 3)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1, 2]);
    let results = run(&plan, &views);

    let mut expected = BTreeSet::new();
    for (rx, ry) in &r {
        for (sy, sz) in &s {
            for (tz, tw) in &t {
                if ry == sy && sz == tz {
                    expected.insert(vec![*rx, *ry, *sz, *tw]);
                }
            }
        }
    }
    assert_eq!(results, expected);
    assert!(!results.is_empty());
}

#[test]
fn self_join_grandparent() {
    let dir = TempDir::new("run-grandparent");
    let schema = schema(1);

    let edges = vec![(0u64, 1u64), (1, 2), (2, 3), (4, 1)];
    let views = views_of(&dir, &schema, std::slice::from_ref(&edges));

    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 0, &[(0, 1), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);

    let results = run(&plan, &views);

    let mut expected = BTreeSet::new();
    for (c, p) in &edges {
        for (p2, g) in &edges {
            if p == p2 {
                expected.insert(vec![*c, *p, *g]);
            }
        }
    }
    assert_eq!(results, expected);
    assert_eq!(results.len(), 3); 
}

#[test]
fn triangle_is_wcoj_honest() {
    let dir = TempDir::new("run-triangle");
    let schema = schema(3);

    let r: Vec<(u64, u64)> = (0..6).flat_map(|x| (0..6).map(move |y| (x, y))).collect();
    let s: Vec<(u64, u64)> = (0..6).map(|y| (y, (y + 1) % 6)).collect();
    let t: Vec<(u64, u64)> = (0..6).map(|z| (z, (z + 2) % 6)).collect();
    let views = views_of(&dir, &schema, &[r.clone(), s.clone(), t.clone()]);

    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            occurrence(2, 2, &[(0, 2), (1, 0)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1, 2]);
    let results = run(&plan, &views);

    let mut expected = BTreeSet::new();
    for (rx, ry) in &r {
        for (sy, sz) in &s {
            for (tz, tx) in &t {
                if ry == sy && sz == tz && tx == rx {
                    expected.insert(vec![*rx, *ry, *sz]);
                }
            }
        }
    }
    assert_eq!(results, expected);
    assert!(!results.is_empty());
}

#[test]
fn zero_binding_atom_gates_the_query() {
    let dir = TempDir::new("run-gate");
    let schema = schema(2);
    let r = vec![(1u64, 2u64), (3, 4)];

    for (gate_rows, expect_rows) in [(vec![(9u64, 9u64)], 2usize), (vec![], 0)] {
        let dir2 = TempDir::new(&format!("run-gate-{expect_rows}"));
        let views = views_of(&dir2, &schema, &[r.clone(), gate_rows]);
        let normalized = normalized(
            vec![occurrence(0, 0, &[(0, 0), (1, 1)]), occurrence(1, 1, &[])],
            vec![],
        );
        let plan = planned(&normalized, &schema, &[0, 1]);
        let results = run(&plan, &views);
        assert_eq!(results.len(), expect_rows, "gate case {expect_rows}");
    }
    drop(dir);
}

#[test]
fn empty_relations_yield_empty_results() {
    let dir = TempDir::new("run-empty");
    let schema = schema(2);
    let views = views_of(&dir, &schema, &[vec![(1, 2)], vec![]]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    assert!(run(&plan, &views).is_empty());
}

#[test]
fn duplicate_heavy_skew_collapses_to_the_distinct_binding_set() {
    let dir = TempDir::new("run-skew");
    let schema = schema(2);

    let r: Vec<(u64, u64)> = (0..50).map(|i| (i % 2, i % 3)).collect();
    let s: Vec<(u64, u64)> = (0..50).map(|i| (i % 3, i % 5)).collect();
    let views = views_of(&dir, &schema, &[r.clone(), s.clone()]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    let results = run(&plan, &views);
    let mut expected = BTreeSet::new();
    for (ra, rb) in &r {
        for (sa, sb) in &s {
            if rb == sa {
                expected.insert(vec![*ra, *rb, *sb]);
            }
        }
    }
    assert_eq!(results, expected);
}

#[test]
fn residuals_filter_across_atoms() {
    let dir = TempDir::new("run-residuals");
    let schema = schema(2);
    let r: Vec<(u64, u64)> = (0..10).map(|i| (i, i)).collect();
    let s: Vec<(u64, u64)> = (0..10).map(|i| (i, 9 - i)).collect();
    let views = views_of(&dir, &schema, &[r.clone(), s.clone()]);

    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![FilterPredicate::FieldsCompare {
            left: OperandAddr::from(VarId(1)),
            right: OperandAddr::from(VarId(2)),
            op: WordCmp::Lt,
        }],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    let results = run(&plan, &views);
    let mut expected = BTreeSet::new();
    for (rx, ra) in &r {
        for (sx, sb) in &s {
            if rx == sx && ra < sb {
                expected.insert(vec![*rx, *ra, *sb]);
            }
        }
    }
    assert_eq!(results, expected);
    assert_eq!(results.len(), 5); 
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one differential harness, generator to oracle — clearer kept together"
)]
fn randomized_differential_against_the_nested_loop_oracle() {

    let mut state = 0x1234_5678_9ABC_DEF0_u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 33
    };

    let schema = schema(3);
    for case in 0..60u32 {

        let domain = 1 + next() % 8;
        let mut data: Vec<Vec<(u64, u64)>> = Vec::new();
        for _ in 0..3 {
            let rows = 1 + next() % 40;
            let mut rel = Vec::new();
            for _ in 0..rows {
                rel.push((next() % domain, next() % domain));
            }
            rel.sort_unstable();
            rel.dedup();
            data.push(rel);
        }
        let dir = TempDir::new(&format!("run-differential-{case}"));
        let views = views_of(&dir, &schema, &data);

        let shape = case % 3;
        let occurrences = match shape {
            0 => vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
            ],
            1 => vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 0), (1, 2)]),
            ],
            _ => vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 0), (1, 2)]),
            ],
        };
        let n = occurrences.len();
        let normalized = normalized(occurrences, vec![]);

        let mut order: Vec<u16> = (0..u16::try_from(n).expect("small")).collect();
        for i in (1..order.len()).rev() {
            let j = usize::try_from(next()).expect("64-bit") % (i + 1);
            order.swap(i, j);
        }

        let plan = {
            let join_order = JoinOrder {
                order: order.iter().map(|o| OccId(*o)).collect(),
                estimates: vec![0; n],
            };
            let mut fj = binary2fj(&normalized, &join_order);
            factor(&mut fj);
            crate::plan::fj::gj_split(&mut fj);
            validate(&fj, &normalized, &schema, &BTreeSet::new()).expect("valid plan")
        };

        let mut expected = BTreeSet::new();
        match shape {
            0 => {
                for (a, b) in &data[0] {
                    for (c, d) in &data[1] {
                        if b == c {
                            expected.insert(vec![*a, *b, *d]);
                        }
                    }
                }
            }
            1 => {
                for (a, b) in &data[0] {
                    for (c, d) in &data[1] {
                        for (e, g) in &data[2] {
                            if b == c && a == e && d == g {
                                expected.insert(vec![*a, *b, *d]);
                            }
                        }
                    }
                }
            }
            _ => {
                for (a, b) in &data[0] {
                    for (c, d) in &data[1] {
                        if a == c {
                            expected.insert(vec![*a, *b, *d]);
                        }
                    }
                }
            }
        }

        for batch in [1usize, 7, 128] {

            // VarId order before comparing with the oracle.
            let got: BTreeSet<Vec<u64>> = run_at(&plan, &views, batch)
                .into_iter()
                .map(|row| {
                    (0..3u16)
                        .map(|v| row[plan.slot_of(VarId(v))])
                        .collect::<Vec<u64>>()
                })
                .collect();
            assert_eq!(
                got, expected,
                "case {case} shape {shape} order {order:?} batch {batch} domain {domain}"
            );
        }
    }
}
