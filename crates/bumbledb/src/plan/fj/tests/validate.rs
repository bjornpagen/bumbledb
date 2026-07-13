use super::*;
use crate::ir::CmpOp;
use std::collections::BTreeSet;

/// The sink-relevance bits encode aggregate
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
    let narrow =
        validate(&plan, &normalized, &schema(3, 3), vec![0; 3], &projected).expect("valid plan");
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

/// A plan that drops a zero-variable (gate)
/// occurrence must not validate — the executor would skip the
/// nonemptiness check and return all of R instead of the empty set.
#[test]
fn a_plan_dropping_a_gate_occurrence_is_rejected() {
    // Q(x) :- R(x), Gate() — occurrence 1 binds nothing.
    let query = normalized(
        vec![occurrence(0, 0, &[(1, X)]), occurrence(1, 1, &[])],
        vec![],
    );
    // The hand-built plan covers only the bound occurrence.
    let plan = FjPlan {
        nodes: vec![Node {
            subatoms: vec![subatom(0, &[X])],
        }],
    };
    assert_eq!(
        validate(&plan, &query, &schema(2, 3), vec![0], &BTreeSet::new())
            .map(|_| ())
            .unwrap_err(),
        PlanError::MissingOccurrence { occ: OccId(1) }
    );

    // The degenerate extreme: an all-gates query with an empty plan.
    let all_gates = normalized(vec![occurrence(0, 0, &[])], vec![]);
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
    validate(&with_gate, &query, &schema(2, 3), vec![0], &BTreeSet::new())
        .expect("a gate subatom is the legal form");
}

/// A subatom referencing an occurrence outside the query is a
/// typed rejection, not an executor index panic.
#[test]
fn a_subatom_with_an_unknown_occurrence_is_rejected() {
    let query = normalized(vec![occurrence(0, 0, &[(1, X)])], vec![]);
    let plan = FjPlan {
        nodes: vec![Node {
            subatoms: vec![subatom(0, &[X]), subatom(99, &[])],
        }],
    };
    assert_eq!(
        validate(&plan, &query, &schema(1, 3), vec![0], &BTreeSet::new())
            .map(|_| ())
            .unwrap_err(),
        PlanError::UnknownOccurrence {
            node: 0,
            occ: OccId(99)
        }
    );
}

/// Negated occurrences join no node: a hand-built plan smuggling one
/// into a subatom is rejected by name — the executor reaches negation
/// exclusively through anti-probes.
#[test]
fn a_subatom_over_a_negated_occurrence_is_rejected() {
    let query = normalized(
        vec![occurrence(0, 0, &[(1, X)]), negated(1, 1, &[(1, X)])],
        vec![],
    );
    let plan = FjPlan {
        nodes: vec![Node {
            subatoms: vec![subatom(0, &[X]), subatom(1, &[])],
        }],
    };
    assert_eq!(
        validate(&plan, &query, &schema(2, 3), vec![0], &BTreeSet::new())
            .map(|_| ())
            .unwrap_err(),
        PlanError::NonParticipatingOccurrenceInNode {
            node: 0,
            occ: OccId(1)
        }
    );
}

/// The attachment criterion (PRD 15): a negated atom over variables
/// first all-bound at the second node of a three-node plan lands in
/// that node's `anti_probes` — not earlier, not later.
#[test]
fn anti_probe_attaches_to_the_earliest_all_bound_node() {
    // Unfactored clover plan: node 0 binds {x, a}, node 1 binds {b},
    // node 2 binds {c}. The negated atom reads (x, b): bound at node 1.
    let mut occurrences = clover().occurrences;
    occurrences.push(negated(3, 2, &[(1, X), (2, B)]));
    let query = normalized(occurrences, vec![]);
    let plan = binary2fj(&query, &order(&[0, 1, 2]));
    let validated =
        validate(&plan, &query, &schema(3, 3), vec![0; 3], &BTreeSet::new()).expect("valid plan");
    assert!(validated.nodes()[0].anti_probes.is_empty());
    assert_eq!(validated.nodes()[1].anti_probes.len(), 1);
    assert_eq!(validated.nodes()[1].anti_probes[0].occurrence, OccId(3));
    assert!(validated.nodes()[2].anti_probes.is_empty());
}

/// The attachment criterion's other half: a negated atom over
/// root-bound variables lands at the root — and so does a
/// zero-variable emptiness gate (the empty set is bound everywhere).
#[test]
fn root_only_anti_probes_attach_to_the_root() {
    let mut occurrences = clover().occurrences;
    occurrences.push(negated(3, 2, &[(1, X), (2, A)]));
    occurrences.push(negated(4, 1, &[]));
    let query = normalized(occurrences, vec![]);
    let plan = binary2fj(&query, &order(&[0, 1, 2]));
    let validated =
        validate(&plan, &query, &schema(3, 3), vec![0; 3], &BTreeSet::new()).expect("valid plan");
    let root_probes: Vec<OccId> = validated.nodes()[0]
        .anti_probes
        .iter()
        .map(|p| p.occurrence)
        .collect();
    assert_eq!(root_probes, vec![OccId(3), OccId(4)]);
    assert!(
        validated.nodes()[1..]
            .iter()
            .all(|n| n.anti_probes.is_empty())
    );
}

/// A negated occurrence's trie schema is its single probe level: all
/// its variables in binding (slot) order, per §3.3 — the shape of a
/// fully-hoisted positive lookup.
#[test]
fn negated_occurrences_get_probe_order_trie_schemas() {
    let mut occurrences = clover().occurrences;
    // Written (b, x) in field order — the probe level follows binding
    // order (x bound at node 0, b at node 1), not field order.
    occurrences.push(negated(3, 2, &[(1, B), (2, X)]));
    let query = normalized(occurrences, vec![]);
    let plan = binary2fj(&query, &order(&[0, 1, 2]));
    let validated =
        validate(&plan, &query, &schema(3, 3), vec![0; 3], &BTreeSet::new()).expect("valid plan");
    assert_eq!(validated.occurrence(OccId(3)).trie_schema, vec![vec![X, B]]);
    assert_eq!(validated.occurrence(OccId(3)).key_widths, vec![2]);
}

#[test]
fn trie_schemas_match_the_papers_triangle_worked_example() {
    // Triangle plan [[R(x,y),S(y),T(x)],[S(z),T(z)]] (§3.3): R is a
    // vector, S a map->vector, T a map->map (no trailing [] under COLT
    // laziness — the build-phase question dissolves, 40-execution).
    let query = normalized(
        vec![
            occurrence(0, 0, &[(1, X), (2, Y)]),
            occurrence(1, 1, &[(1, Y), (2, Z)]),
            occurrence(2, 2, &[(1, X), (2, Z)]),
        ],
        vec![],
    );
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
    let validated =
        validate(&plan, &query, &schema(3, 3), vec![0, 0], &BTreeSet::new()).expect("valid plan");
    assert_eq!(validated.occurrence(OccId(0)).trie_schema, vec![vec![X, Y]]);
    assert_eq!(validated.occurrence(OccId(0)).key_widths, vec![2]);
    assert_eq!(
        validated.occurrence(OccId(1)).trie_schema,
        vec![vec![Y], vec![Z]]
    );
    assert_eq!(validated.occurrence(OccId(1)).key_widths, vec![1, 1]);
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
    let query = normalized(
        clover().occurrences,
        vec![PlacedComparison {
            op: CmpOp::Lt,
            lhs: A,
            rhs: B,
        }],
    );
    let plan = binary2fj(&query, &order(&[0, 1, 2]));
    let validated =
        validate(&plan, &query, &schema(3, 3), vec![0; 3], &BTreeSet::new()).expect("valid plan");
    assert!(validated.nodes()[0].residuals.is_empty());
    assert_eq!(validated.nodes()[1].residuals.len(), 1);
    assert!(validated.nodes()[2].residuals.is_empty());
}

/// The placement regression (found executing PRD 16): an item whose
/// variable set first fails at the root must keep being checked in
/// full at every later node. The bug this pins: consuming one
/// variables iterator across the node scan leaves it exhausted after
/// the first failing node, so the NEXT node passes vacuously — a < c
/// (bound at nodes 0 and 2) attached to node 1, where c is unbound,
/// and the executor compared against a zero slot.
#[test]
fn placement_rechecks_every_variable_at_every_node() {
    // Residual half: a < c places on node 2 (c binds there), never 1.
    let query = normalized(
        clover().occurrences,
        vec![PlacedComparison {
            op: CmpOp::Lt,
            lhs: A,
            rhs: C,
        }],
    );
    let plan = binary2fj(&query, &order(&[0, 1, 2]));
    let validated =
        validate(&plan, &query, &schema(3, 3), vec![0; 3], &BTreeSet::new()).expect("valid plan");
    assert!(validated.nodes()[0].residuals.is_empty());
    assert!(validated.nodes()[1].residuals.is_empty());
    assert_eq!(validated.nodes()[2].residuals.len(), 1);

    // Anti-probe half: ¬T(a, c) places on node 2 through the same rule.
    let mut occurrences = clover().occurrences;
    occurrences.push(negated(3, 2, &[(1, A), (2, C)]));
    let query = normalized(occurrences, vec![]);
    let plan = binary2fj(&query, &order(&[0, 1, 2]));
    let validated =
        validate(&plan, &query, &schema(3, 3), vec![0; 3], &BTreeSet::new()).expect("valid plan");
    assert!(validated.nodes()[0].anti_probes.is_empty());
    assert!(validated.nodes()[1].anti_probes.is_empty());
    assert_eq!(validated.nodes()[2].anti_probes.len(), 1);
}

#[test]
fn self_join_plans_validate_over_occurrences() {
    // Grandparent over OrgParent: two occurrences of one relation.
    let query = normalized(
        vec![
            occurrence(0, 0, &[(1, X), (2, Y)]),
            occurrence(1, 0, &[(1, Y), (2, Z)]),
        ],
        vec![],
    );
    let mut plan = binary2fj(&query, &order(&[0, 1]));
    factor(&mut plan);
    let validated = validate(&plan, &query, &schema(1, 3), vec![0, 0], &BTreeSet::new())
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
    let mut query = clover();
    query.occurrences.truncate(1);
    let err = validate(&plan, &query, &schema(3, 3), vec![0], &BTreeSet::new()).unwrap_err();
    assert_eq!(
        err,
        PlanError::DuplicateOccurrenceInNode {
            node: 0,
            occ: OccId(0)
        }
    );
}

#[test]
fn distinct_bindings_flag_tracks_key_coverage() {
    // Fresh-keyed occurrence: field 0 (the auto-key) is var-bound in
    // every occurrence -> flag set.
    let query = normalized(
        vec![
            occurrence(0, 0, &[(0, X), (1, A)]),
            occurrence(1, 1, &[(0, B), (1, X)]),
        ],
        vec![],
    );
    let plan = binary2fj(&query, &order(&[0, 1]));
    let validated =
        validate(&plan, &query, &schema(2, 2), vec![0, 0], &BTreeSet::new()).expect("valid plan");
    assert!(validated.distinct_bindings());

    // Occurrence 1 binds only a non-key field -> flag clear.
    let query = normalized(
        vec![
            occurrence(0, 0, &[(0, X), (1, A)]),
            occurrence(1, 1, &[(1, X)]),
        ],
        vec![],
    );
    let plan = binary2fj(&query, &order(&[0, 1]));
    let validated =
        validate(&plan, &query, &schema(2, 2), vec![0, 0], &BTreeSet::new()).expect("valid plan");
    assert!(!validated.distinct_bindings());
}

#[test]
fn binding_slots_follow_node_order() {
    let query = clover();
    let mut plan = binary2fj(&query, &order(&[0, 1, 2]));
    factor(&mut plan);
    let validated =
        validate(&plan, &query, &schema(3, 3), vec![0; 3], &BTreeSet::new()).expect("valid plan");
    // Factored clover: node 0 binds {x, a}, node 1 binds {b}, node 2
    // binds {c}. Slot order follows; every scalar is one slot wide.
    assert_eq!(
        validated.slots(),
        &[
            (X, SlotWidth::ONE),
            (A, SlotWidth::ONE),
            (B, SlotWidth::ONE),
            (C, SlotWidth::ONE),
        ]
    );
    assert_eq!(validated.slot_of(C), 3);
    assert_eq!(validated.slot_count(), 4);
}
