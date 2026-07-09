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
