use super::*;

#[test]
fn binary2fj_and_factor_match_the_papers_clover_example() {
    let normalized = clover();
    let mut plan = binary2fj(&normalized, &order(&[0, 1, 2]));

    assert_eq!(
        plan.nodes,
        vec![
            Node {
                estimate: 0,
                subatoms: vec![subatom(0, &[X, A]), subatom(1, &[X])]
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(1, &[B]), subatom(2, &[X])]
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(2, &[C])]
            },
        ]
    );
    factor(&mut plan);

    assert_eq!(
        plan.nodes,
        vec![
            Node {
                estimate: 0,
                subatoms: vec![subatom(0, &[X, A]), subatom(1, &[X]), subatom(2, &[X])]
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(1, &[B])]
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(2, &[C])]
            },
        ]
    );
}

#[test]
fn gj_split_lowers_the_triangle_to_the_gj_plan() {

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
                estimate: 0,
                subatoms: vec![subatom(0, &[X, Y]), subatom(1, &[Y])]
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(1, &[Z]), subatom(2, &[Z, X])]
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(2, &[])]
            },
        ]
    );
    gj_split(&mut plan);
    assert_eq!(
        plan.nodes,
        vec![
            Node {
                estimate: 0,
                subatoms: vec![subatom(0, &[X, Y]), subatom(1, &[Y]), subatom(2, &[X])]
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(1, &[Z]), subatom(2, &[Z])]
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(2, &[])]
            },
        ]
    );
    let validated = validate(
        &plan,
        &query,
        &schema(3, 3),
        &std::collections::BTreeSet::new(),
    )
    .expect("the split plan validates");
    assert_eq!(validated.nodes()[1].covers, vec![0, 1]);
    assert_eq!(
        validated.occurrence(OccId(2)).trie_schema,
        vec![vec![X], vec![Z], vec![]]
    );
}

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

#[test]
fn gj_split_keeps_same_node_variable_pairs_whole() {

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

#[test]
fn fold_split_prefixes_group_variables() {

    let query = normalized(vec![occurrence(0, 0, &[(0, A), (1, X), (2, B)])], vec![]);
    let mut plan = binary2fj(&query, &order(&[0]));
    factor(&mut plan);
    assert_eq!(
        plan.nodes,
        vec![Node {
            estimate: 0,
            subatoms: vec![subatom(0, &[A, X, B])]
        }]
    );
    let group: std::collections::BTreeSet<VarId> = [A].into_iter().collect();
    plan.nodes[0].estimate = 500_000;
    fold_split(&mut plan, &group);
    assert_eq!(
        plan.nodes,
        vec![
            Node {
                estimate: 500_000,
                subatoms: vec![subatom(0, &[A])]
            },
            Node {
                estimate: 500_000,
                subatoms: vec![subatom(0, &[X, B])]
            },
        ]
    );
    let validated = validate(
        &plan,
        &query,
        &schema(1, 3),
        &std::collections::BTreeSet::new(),
    )
    .expect("the split plan validates");
    assert_eq!(
        validated.occurrence(OccId(0)).trie_schema,
        vec![vec![A], vec![X, B]]
    );
}

#[test]
fn fold_split_moves_group_only_lookups_to_the_prefix() {

    let query = normalized(
        vec![
            occurrence(0, 0, &[(1, A), (2, X)]),
            occurrence(1, 1, &[(1, A)]),
            occurrence(2, 2, &[(1, A), (2, X)]),
        ],
        vec![],
    );
    let mut plan = FjPlan {
        nodes: vec![Node {
            estimate: 0,
            subatoms: vec![subatom(0, &[A, X]), subatom(1, &[A]), subatom(2, &[A, X])],
        }],
    };
    let group: std::collections::BTreeSet<VarId> = [A].into_iter().collect();
    plan.nodes[0].estimate = 9_000;
    fold_split(&mut plan, &group);
    assert_eq!(
        plan.nodes,
        vec![
            Node {
                estimate: 9_000,
                subatoms: vec![subatom(0, &[A]), subatom(1, &[A])]
            },
            Node {
                estimate: 9_000,
                subatoms: vec![subatom(0, &[X]), subatom(2, &[A, X])]
            },
        ],
        "S rides the prefix; T stays with the fold domain"
    );
    let validated = validate(
        &plan,
        &query,
        &schema(3, 3),
        &std::collections::BTreeSet::new(),
    )
    .expect("the split plan validates");
    assert_eq!(
        validated.occurrence(OccId(0)).trie_schema,
        vec![vec![A], vec![X]]
    );
}

#[test]
fn fold_split_leaves_unmixed_levels_alone() {
    let query = normalized(vec![occurrence(0, 0, &[(0, A), (1, X)])], vec![]);
    let mut plan = binary2fj(&query, &order(&[0]));
    let shape = plan.clone();

    let none: std::collections::BTreeSet<VarId> = [C].into_iter().collect();
    fold_split(&mut plan, &none);
    assert_eq!(plan, shape);

    let all: std::collections::BTreeSet<VarId> = [A, X].into_iter().collect();
    fold_split(&mut plan, &all);
    assert_eq!(plan, shape);
}

#[test]
fn binary2fj_matches_the_papers_chain_example() {

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
                estimate: 0,
                subatoms: vec![subatom(0, &[X, Y]), subatom(1, &[Y])]
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(1, &[Z]), subatom(2, &[Z])]
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(2, &[U]), subatom(3, &[U])]
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(3, &[V])]
            },
        ]
    );
}
