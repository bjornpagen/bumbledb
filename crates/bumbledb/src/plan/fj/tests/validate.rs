use super::*;
use crate::ir::CmpOp;
use std::collections::BTreeSet;

/// PRD 05 (docs/hardening): the sink-relevance bits encode aggregate
/// skip-illegality. A projection over one variable leaves deeper
/// nodes skippable (the D2 win); the all-variables sink set an
/// aggregate plan passes marks every variable-binding node relevant.
#[test]
fn aggregate_sink_vars_mark_every_node_relevant() {
    let normalized = clover();
    let mut plan = binary2fj(&normalized, &order(&[0, 1, 2]));
    factor(&mut plan);

    // Projection over x only: at least one node binds nothing
    // projected — D2 has something to skip.
    let projected: BTreeSet<VarId> = [X].into_iter().collect();
    let narrow = validate(&plan, &normalized, &schema(3, 3), vec![0; 3], &projected)
        .expect("valid plan");
    assert!(
        narrow.nodes().iter().any(|n| !n.sink_relevant),
        "projections keep skippable nodes"
    );

    // The aggregate rule: every variable sink-relevant — every
    // variable-binding node absorbs any skip that reaches it.
    let all_vars: BTreeSet<VarId> = [X, A, B, C].into_iter().collect();
    let full =
        validate(&plan, &normalized, &schema(3, 3), vec![0; 3], &all_vars).expect("valid plan");
    assert!(
        full.nodes()
            .iter()
            .filter(|n| !n.new_vars.is_empty())
            .all(|n| n.sink_relevant),
        "every variable-binding node is relevant under aggregation"
    );
}

/// PRD 03 (docs/hardening): a plan that drops a zero-variable (gate)
/// occurrence must not validate — the executor would skip the
/// nonemptiness check and return all of R instead of the empty set.
#[test]
fn a_plan_dropping_a_gate_occurrence_is_rejected() {
    // Q(x) :- R(x), Gate() — occurrence 1 binds nothing.
    let normalized = NormalizedQuery {
        occurrences: vec![occurrence(0, 0, &[(1, X)]), occurrence(1, 1, &[])],
        residuals: vec![],
    };
    // The hand-built plan covers only the bound occurrence.
    let plan = FjPlan {
        nodes: vec![Node {
            subatoms: vec![subatom(0, &[X])],
        }],
    };
    assert_eq!(
        validate(&plan, &normalized, &schema(2, 3), vec![0], &BTreeSet::new())
            .map(|_| ())
            .unwrap_err(),
        PlanError::MissingOccurrence { occ: OccId(1) }
    );

    // The degenerate extreme: an all-gates query with an empty plan.
    let all_gates = NormalizedQuery {
        occurrences: vec![occurrence(0, 0, &[])],
        residuals: vec![],
    };
    let empty = FjPlan { nodes: vec![] };
    assert_eq!(
        validate(&empty, &all_gates, &schema(1, 3), vec![], &BTreeSet::new())
            .map(|_| ())
            .unwrap_err(),
        PlanError::MissingOccurrence { occ: OccId(0) }
    );

    // Positive control: the gate carried as an empty-vars subatom —
    // exactly what binary2fj emits — validates.
    let with_gate = FjPlan {
        nodes: vec![Node {
            subatoms: vec![subatom(0, &[X]), subatom(1, &[])],
        }],
    };
    validate(
        &with_gate,
        &normalized,
        &schema(2, 3),
        vec![0],
        &BTreeSet::new(),
    )
    .expect("a gate subatom is the legal form");
}

/// PRD 03: a subatom referencing an occurrence outside the query is a
/// typed rejection, not an executor index panic.
#[test]
fn a_subatom_with_an_unknown_occurrence_is_rejected() {
    let normalized = NormalizedQuery {
        occurrences: vec![occurrence(0, 0, &[(1, X)])],
        residuals: vec![],
    };
    let plan = FjPlan {
        nodes: vec![Node {
            subatoms: vec![subatom(0, &[X]), subatom(99, &[])],
        }],
    };
    assert_eq!(
        validate(&plan, &normalized, &schema(1, 3), vec![0], &BTreeSet::new())
            .map(|_| ())
            .unwrap_err(),
        PlanError::UnknownOccurrence {
            node: 0,
            occ: OccId(99)
        }
    );
}

#[test]
fn trie_schemas_match_the_papers_triangle_worked_example() {
    // Triangle plan [[R(x,y),S(y),T(x)],[S(z),T(z)]] (§3.3): R is a
    // vector, S a map->vector, T a map->map (no trailing [] under COLT
    // laziness — the build-phase question dissolves, 30-execution).
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, &[(1, X), (2, Y)]),
            occurrence(1, 1, &[(1, Y), (2, Z)]),
            occurrence(2, 2, &[(1, X), (2, Z)]),
        ],
        residuals: vec![],
    };
    let plan = FjPlan {
        nodes: vec![
            Node {
                subatoms: vec![subatom(0, &[X, Y]), subatom(1, &[Y]), subatom(2, &[X])],
            },
            Node {
                subatoms: vec![subatom(1, &[Z]), subatom(2, &[Z])],
            },
        ],
    };
    let validated = validate(
        &plan,
        &normalized,
        &schema(3, 3),
        vec![0, 0],
        &BTreeSet::new(),
    )
    .expect("valid plan");
    assert_eq!(validated.occurrence(OccId(0)).trie_schema, vec![vec![X, Y]]);
    assert_eq!(
        validated.occurrence(OccId(1)).trie_schema,
        vec![vec![Y], vec![Z]]
    );
    assert_eq!(
        validated.occurrence(OccId(2)).trie_schema,
        vec![vec![X], vec![Z]]
    );
}

#[test]
fn gj_style_plan_has_multiple_covers_on_the_first_node() {
    // The paper: "for the first node we could have also chosen S(x) or
    // T(x) as cover" — the GJ plan for the clover query.
    let plan = FjPlan {
        nodes: vec![
            Node {
                subatoms: vec![subatom(0, &[X]), subatom(1, &[X]), subatom(2, &[X])],
            },
            Node {
                subatoms: vec![subatom(0, &[A])],
            },
            Node {
                subatoms: vec![subatom(1, &[B])],
            },
            Node {
                subatoms: vec![subatom(2, &[C])],
            },
        ],
    };
    let validated = validate(
        &plan,
        &clover(),
        &schema(3, 3),
        vec![0; 4],
        &BTreeSet::new(),
    )
    .expect("valid plan");
    assert_eq!(validated.nodes()[0].covers, vec![0, 1, 2]);
    assert_eq!(validated.nodes()[1].covers, vec![0]);
}

#[test]
fn residuals_attach_to_the_first_node_binding_both_sides() {
    // Residual a < b: a is bound by node 1 (R's a), b by node 2 (S's b)
    // in the unfactored clover plan — so it places on node 2.
    let mut normalized = clover();
    normalized.residuals = vec![PlacedComparison {
        op: CmpOp::Lt,
        lhs: A,
        rhs: B,
    }];
    let plan = binary2fj(&normalized, &order(&[0, 1, 2]));
    let validated = validate(
        &plan,
        &normalized,
        &schema(3, 3),
        vec![0; 3],
        &BTreeSet::new(),
    )
    .expect("valid plan");
    assert!(validated.nodes()[0].residuals.is_empty());
    assert_eq!(validated.nodes()[1].residuals.len(), 1);
    assert!(validated.nodes()[2].residuals.is_empty());
}

#[test]
fn self_join_plans_validate_over_occurrences() {
    // Grandparent over OrgParent: two occurrences of one relation.
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, &[(1, X), (2, Y)]),
            occurrence(1, 0, &[(1, Y), (2, Z)]),
        ],
        residuals: vec![],
    };
    let mut plan = binary2fj(&normalized, &order(&[0, 1]));
    factor(&mut plan);
    let validated = validate(
        &plan,
        &normalized,
        &schema(1, 3),
        vec![0, 0],
        &BTreeSet::new(),
    )
    .expect("self-joins validate");
    assert_eq!(validated.occurrences().len(), 2);
}

#[test]
fn duplicate_occurrence_within_a_node_is_rejected() {
    let plan = FjPlan {
        nodes: vec![Node {
            subatoms: vec![subatom(0, &[X, A]), subatom(0, &[])],
        }],
    };
    let mut normalized = clover();
    normalized.occurrences.truncate(1);
    let err =
        validate(&plan, &normalized, &schema(3, 3), vec![0], &BTreeSet::new()).unwrap_err();
    assert_eq!(
        err,
        PlanError::DuplicateOccurrenceInNode {
            node: 0,
            occ: OccId(0)
        }
    );
}

#[test]
fn distinct_bindings_flag_tracks_unique_coverage() {
    // Serial-bound occurrence: field 0 (serial) is var-bound in every
    // occurrence -> flag set.
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, &[(0, X), (1, A)]),
            occurrence(1, 1, &[(0, B), (1, X)]),
        ],
        residuals: vec![],
    };
    let plan = binary2fj(&normalized, &order(&[0, 1]));
    let validated = validate(
        &plan,
        &normalized,
        &schema(2, 2),
        vec![0, 0],
        &BTreeSet::new(),
    )
    .expect("valid plan");
    assert!(validated.distinct_bindings());

    // Occurrence 1 binds only a non-unique field -> flag clear.
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, 0, &[(0, X), (1, A)]),
            occurrence(1, 1, &[(1, X)]),
        ],
        residuals: vec![],
    };
    let plan = binary2fj(&normalized, &order(&[0, 1]));
    let validated = validate(
        &plan,
        &normalized,
        &schema(2, 2),
        vec![0, 0],
        &BTreeSet::new(),
    )
    .expect("valid plan");
    assert!(!validated.distinct_bindings());
}

#[test]
fn binding_slots_follow_node_order() {
    let normalized = clover();
    let mut plan = binary2fj(&normalized, &order(&[0, 1, 2]));
    factor(&mut plan);
    let validated = validate(
        &plan,
        &normalized,
        &schema(3, 3),
        vec![0; 3],
        &BTreeSet::new(),
    )
    .expect("valid plan");
    // Factored clover: node 0 binds {x, a}, node 1 binds {b}, node 2
    // binds {c}. Slot order follows.
    assert_eq!(validated.slots(), &[X, A, B, C]);
    assert_eq!(validated.slot_of(C), 3);
}
