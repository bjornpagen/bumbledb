use super::*;

/// The clover query over the paper's Fig. 4 instance: only
/// (x0, a0, b0, c0) joins.
#[test]
fn clover_on_the_papers_instance() {
    let dir = TempDir::new("run-clover");
    let schema = schema(3);
    let n = 20u64;
    // R = {(x0,a0)} u {(x1,ai_l), (x2,ai_r)}; S, T rotated (Fig. 4).
    // Encode x0..x3 as 0..3 and the a/b/c values as 100+i / 200+i.
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

    // Q(x,a,b,c) :- R(x,a), S(x,b), T(x,c).
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
            occurrence(2, 2, &[(0, 0), (1, 3)]),
        ],
        residuals: vec![],
    };
    let plan = planned(&normalized, &schema, &[0, 1, 2]);
    let results = run(&plan, &views);

    // Naive oracle: triple loop.
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

    // Q(x,y,z,w) :- R(x,y), S(y,z), T(z,w).
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            occurrence(2, 2, &[(0, 2), (1, 3)]),
        ],
        residuals: vec![],
    };
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
    // OrgParent(child, parent): 0->1->2->3 plus a fork 4->1.
    let edges = vec![(0u64, 1u64), (1, 2), (2, 3), (4, 1)];
    let views = views_of(&dir, &schema, std::slice::from_ref(&edges));

    // Grandparent(c, g) :- OrgParent(c, p), OrgParent(p, g) — two
    // occurrences of relation 0.
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 0, &[(0, 1), (1, 2)]),
        ],
        residuals: vec![],
    };
    let plan = planned(&normalized, &schema, &[0, 1]);
    // Both occurrences read relation 0: views vector must be indexed by
    // occurrence, not relation — build colts by occurrence's relation.
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
    assert_eq!(results.len(), 3); // 0->1->2, 1->2->3, 4->1->2
}

#[test]
fn triangle_is_wcoj_honest() {
    let dir = TempDir::new("run-triangle");
    let schema = schema(3);
    // R(x,y), S(y,z), T(z,x) over a small dense instance.
    let r: Vec<(u64, u64)> = (0..6).flat_map(|x| (0..6).map(move |y| (x, y))).collect();
    let s: Vec<(u64, u64)> = (0..6).map(|y| (y, (y + 1) % 6)).collect();
    let t: Vec<(u64, u64)> = (0..6).map(|z| (z, (z + 2) % 6)).collect();
    let views = views_of(&dir, &schema, &[r.clone(), s.clone(), t.clone()]);

    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            occurrence(2, 2, &[(0, 2), (1, 0)]),
        ],
        residuals: vec![],
    };
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
    // Gate nonempty: results flow; gate empty: nothing.
    for (gate_rows, expect_rows) in [(vec![(9u64, 9u64)], 2usize), (vec![], 0)] {
        let dir2 = TempDir::new(&format!("run-gate-{expect_rows}"));
        let views = views_of(&dir2, &schema, &[r.clone(), gate_rows]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                Occurrence {
                    occ_id: OccId(1),
                    relation: RelationId(1),
                    vars: vec![],
                    filters: vec![],
                },
            ],
            residuals: vec![],
        };
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
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
        ],
        residuals: vec![],
    };
    let plan = planned(&normalized, &schema, &[0, 1]);
    assert!(run(&plan, &views).is_empty());
}

#[test]
fn duplicate_heavy_skew_collapses_to_the_distinct_binding_set() {
    let dir = TempDir::new("run-skew");
    let schema = schema(2);
    // Heavy duplication in the join column (post-collapse the binding
    // set is small).
    let r: Vec<(u64, u64)> = (0..50).map(|i| (i % 2, i % 3)).collect();
    let s: Vec<(u64, u64)> = (0..50).map(|i| (i % 3, i % 5)).collect();
    let views = views_of(&dir, &schema, &[r.clone(), s.clone()]);
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
        ],
        residuals: vec![],
    };
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
    // R(x, a), S(x, b), a < b.
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        residuals: vec![PlacedComparison {
            op: CmpOp::Lt,
            lhs: VarId(1),
            rhs: VarId(2),
        }],
    };
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
    assert_eq!(results.len(), 5); // i in 0..=4: i < 9-i
}

/// The randomized differential family (docs/architecture/50-validation.md):
/// random instances and join orders over three query shapes, the whole
/// production lowering (binary2fj + factor + validate), compared against
/// a brute-force nested-loop oracle at batch sizes {1, 7, 128}. This is
/// the harness that catches plan/executor bugs hand-picked fixtures
/// miss — the cover-rebind bug needed only mild skew.
#[test]
fn randomized_differential_against_the_nested_loop_oracle() {
    // Deterministic LCG (no rand dependency; reproducible failures).
    let mut state = 0x1234_5678_9ABC_DEF0_u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 33
    };

    let schema = schema(3);
    for case in 0..60u32 {
        // Random small instances with skew: values in 0..=domain where
        // a small domain forces duplicates and multi-position chunks.
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

        // Three shapes over vars x=0, y=1, z=2:
        //   chain:    R0(x,y), R1(y,z)
        //   triangle: R0(x,y), R1(y,z), R2(x,z)
        //   clover:   R0(x,y), R1(x,z) (self-shaped star)
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
        let normalized = NormalizedQuery {
            occurrences,
            residuals: vec![],
        };
        // Random join order (a permutation drawn by rejection).
        let mut order: Vec<u16> = (0..u16::try_from(n).expect("small")).collect();
        for i in (1..order.len()).rev() {
            let j = usize::try_from(next()).expect("64-bit") % (i + 1);
            order.swap(i, j);
        }
        let plan = planned(&normalized, &schema, &order);

        // The oracle: brute-force nested loops over the shape.
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
            // Slot order follows the join order; reorder each row into
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
