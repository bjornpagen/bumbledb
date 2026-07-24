use super::*;

#[test]
fn binary2fj_and_factor_match_the_papers_clover_example() {
    let normalized = clover();
    let mut plan = binary2fj(&normalized, &order(&[0, 1, 2]));
    // Fig. 7 output: [[R(x,a),S(x)],[S(b),T(x)],[T(c)]].
    assert_eq!(
        plan.nodes,
        vec![
            Node {
                subatoms: vec![subatom(0, &[X, A]), subatom(1, &[X])]
            },
            Node {
                subatoms: vec![subatom(1, &[B]), subatom(2, &[X])]
            },
            Node {
                subatoms: vec![subatom(2, &[C])]
            },
        ]
    );
    factor(&mut plan);
    // Fig. 8 output: [[R(x,a),S(x),T(x)],[S(b)],[T(c)]].
    assert_eq!(
        plan.nodes,
        vec![
            Node {
                subatoms: vec![subatom(0, &[X, A]), subatom(1, &[X]), subatom(2, &[X])]
            },
            Node {
                subatoms: vec![subatom(1, &[B])]
            },
            Node {
                subatoms: vec![subatom(2, &[C])]
            },
        ]
    );
}

/// The GJ split on the triangle: the closing probe T(z, x) carries two
/// variables first bound at different nodes, so it splits into T(x) at
/// node 0 and T(z) at node 1 — the §3.3 worked GJ plan, and node 1
/// gains its second cover (S(z) and T(z) both bind exactly {z}).
#[test]
fn gj_split_lowers_the_triangle_to_the_gj_plan() {
    // Q :- R(x,y), S(y,z), T(z,x) with order [R, S, T].
    let query = normalized(
        vec![
            occurrence(0, 0, &[(1, X), (2, Y)]),
            occurrence(1, 1, &[(1, Y), (2, Z)]),
            occurrence(2, 2, &[(1, Z), (2, X)]),
        ],
        vec![],
    );
    let mut plan = binary2fj(&query, &order(&[0, 1, 2]));
    factor(&mut plan);
    // factor cannot hoist T(z, x): z is unavailable before node 1.
    assert_eq!(
        plan.nodes,
        vec![
            Node {
                subatoms: vec![subatom(0, &[X, Y]), subatom(1, &[Y])]
            },
            Node {
                subatoms: vec![subatom(1, &[Z]), subatom(2, &[Z, X])]
            },
            Node {
                subatoms: vec![subatom(2, &[])]
            },
        ]
    );
    gj_split(&mut plan);
    assert_eq!(
        plan.nodes,
        vec![
            Node {
                subatoms: vec![subatom(0, &[X, Y]), subatom(1, &[Y]), subatom(2, &[X])]
            },
            Node {
                subatoms: vec![subatom(1, &[Z]), subatom(2, &[Z])]
            },
            Node {
                subatoms: vec![subatom(2, &[])]
            },
        ]
    );
    let validated = validate(
        &plan,
        &query,
        &schema(3, 3),
        vec![0; 3],
        &std::collections::BTreeSet::new(),
    )
    .expect("the split plan validates");
    assert_eq!(validated.nodes()[1].covers, vec![0, 1]);
    assert_eq!(
        validated.occurrence(OccId(2)).trie_schema,
        vec![vec![X], vec![Z], vec![]]
    );
}

/// The chain has no probe subatom spanning two binding nodes — every
/// lookup carries exactly one variable — so the split is the identity
/// and chains stay binary-shaped.
#[test]
fn gj_split_leaves_the_chain_binary_shaped() {
    let query = normalized(
        vec![
            occurrence(0, 0, &[(1, X), (2, Y)]),
            occurrence(1, 1, &[(1, Y), (2, Z)]),
            occurrence(2, 2, &[(1, Z), (2, U)]),
            occurrence(3, 3, &[(1, U), (2, V)]),
        ],
        vec![],
    );
    let mut plan = binary2fj(&query, &order(&[0, 1, 2, 3]));
    factor(&mut plan);
    let factored = plan.clone();
    gj_split(&mut plan);
    assert_eq!(plan, factored);
}

/// A two-variable probe whose variables bind at ONE node stays whole:
/// the split is per binding node, never per variable — a two-word probe
/// into one submap is the right access for same-node variables.
#[test]
fn gj_split_keeps_same_node_variable_pairs_whole() {
    // Q :- R(x,y), S(x,y): S probes both vars, both bound at node 0.
    let query = normalized(
        vec![
            occurrence(0, 0, &[(1, X), (2, Y)]),
            occurrence(1, 1, &[(1, X), (2, Y)]),
        ],
        vec![],
    );
    let mut plan = binary2fj(&query, &order(&[0, 1]));
    factor(&mut plan);
    let factored = plan.clone();
    gj_split(&mut plan);
    assert_eq!(plan, factored);
}

/// The fold-aware level split (the scan-fold pushdown's planner half):
/// a single-atom GROUP BY's one flat level splits into the group
/// prefix and the fold-domain suffix — the leaf run becomes
/// group-constant. Estimates stay node-aligned by duplication.
#[test]
fn fold_split_prefixes_group_variables() {
    // Sales(promo g, qty a, price b) grouped by promo (A here).
    let query = normalized(vec![occurrence(0, 0, &[(0, A), (1, X), (2, B)])], vec![]);
    let mut plan = binary2fj(&query, &order(&[0]));
    factor(&mut plan);
    assert_eq!(
        plan.nodes,
        vec![Node {
            subatoms: vec![subatom(0, &[A, X, B])]
        }]
    );
    let group: std::collections::BTreeSet<VarId> = [A].into_iter().collect();
    let mut estimates = vec![500_000];
    fold_split(&mut plan, &group, &mut estimates);
    assert_eq!(
        plan.nodes,
        vec![
            Node {
                subatoms: vec![subatom(0, &[A])]
            },
            Node {
                subatoms: vec![subatom(0, &[X, B])]
            },
        ]
    );
    assert_eq!(estimates, vec![500_000, 500_000]);
    let validated = validate(
        &plan,
        &query,
        &schema(1, 3),
        estimates,
        &std::collections::BTreeSet::new(),
    )
    .expect("the split plan validates");
    assert_eq!(
        validated.occurrence(OccId(0)).trie_schema,
        vec![vec![A], vec![X, B]]
    );
}

/// The split's identity cases: an opening level with no group variable
/// (nothing to prefix) and one that is ALL group variables (already a
/// prefix) both pass through unchanged.
#[test]
fn fold_split_leaves_unmixed_levels_alone() {
    let query = normalized(vec![occurrence(0, 0, &[(0, A), (1, X)])], vec![]);
    let mut plan = binary2fj(&query, &order(&[0]));
    let shape = plan.clone();
    let mut estimates = vec![10];

    let none: std::collections::BTreeSet<VarId> = [C].into_iter().collect();
    fold_split(&mut plan, &none, &mut estimates);
    assert_eq!(plan, shape);

    let all: std::collections::BTreeSet<VarId> = [A, X].into_iter().collect();
    fold_split(&mut plan, &all, &mut estimates);
    assert_eq!(plan, shape);
    assert_eq!(estimates, vec![10]);
}

#[test]
fn binary2fj_matches_the_papers_chain_example() {
    // Q :- R(x,y), S(y,z), T(z,u), W(u,v) with plan [R,S,T,W] (§4.1).
    let query = normalized(
        vec![
            occurrence(0, 0, &[(1, X), (2, Y)]),
            occurrence(1, 1, &[(1, Y), (2, Z)]),
            occurrence(2, 2, &[(1, Z), (2, U)]),
            occurrence(3, 3, &[(1, U), (2, V)]),
        ],
        vec![],
    );
    let plan = binary2fj(&query, &order(&[0, 1, 2, 3]));
    assert_eq!(
        plan.nodes,
        vec![
            Node {
                subatoms: vec![subatom(0, &[X, Y]), subatom(1, &[Y])]
            },
            Node {
                subatoms: vec![subatom(1, &[Z]), subatom(2, &[Z])]
            },
            Node {
                subatoms: vec![subatom(2, &[U]), subatom(3, &[U])]
            },
            Node {
                subatoms: vec![subatom(3, &[V])]
            },
        ]
    );
}
