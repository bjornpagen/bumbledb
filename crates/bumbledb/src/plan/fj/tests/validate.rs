use super::*;
use crate::ir::WordCmp;
use std::collections::BTreeSet;

#[test]
fn aggregate_sink_vars_mark_every_node_relevant() {
    let normalized = clover();
    let mut plan = binary2fj(&normalized, &order(&[0, 1, 2]));
    factor(&mut plan);

    let projected: BTreeSet<VarId> = [X].into_iter().collect();
    let narrow = validate(&plan, &normalized, &schema(3, 3), &projected).expect("valid plan");
    assert!(
        narrow
            .nodes()
            .iter()
            .any(|n| n.suffix_skip == SuffixSkip::Licensed),
        "projections keep skippable nodes"
    );

    let all_vars: BTreeSet<VarId> = [X, A, B, C].into_iter().collect();
    let full = validate(&plan, &normalized, &schema(3, 3), &all_vars).expect("valid plan");
    assert!(
        full.nodes()
            .iter()
            .filter(|n| !n.new_vars.is_empty())
            .all(|n| n.suffix_skip == SuffixSkip::Forbidden),
        "every variable-binding node is relevant under aggregation"
    );
}

/// A plan that drops a zero-variable (gate) occurrence must not validate — the
/// executor would skip the nonemptiness check and return all of R instead of
/// the empty set.
#[test]
fn a_plan_dropping_a_gate_occurrence_is_rejected() {

    let query = normalized(
        vec![occurrence(0, 0, &[(1, X)]), occurrence(1, 1, &[])],
        vec![],
    );

    let plan = FjPlan {
        nodes: vec![Node {
            estimate: 0,
            subatoms: vec![subatom(0, &[X])],
        }],
    };
    assert_eq!(
        validate(&plan, &query, &schema(2, 3), &BTreeSet::new())
            .map(|_| ())
            .unwrap_err(),
        PlanError::MissingOccurrence { occ: OccId(1) }
    );

    let all_gates = normalized(vec![occurrence(0, 0, &[])], vec![]);
    let empty = FjPlan { nodes: vec![] };
    assert_eq!(
        validate(&empty, &all_gates, &schema(1, 3), &BTreeSet::new())
            .map(|_| ())
            .unwrap_err(),
        PlanError::MissingOccurrence { occ: OccId(0) }
    );

    let with_gate = FjPlan {
        nodes: vec![Node {
            estimate: 0,
            subatoms: vec![subatom(0, &[X]), subatom(1, &[])],
        }],
    };
    validate(&with_gate, &query, &schema(2, 3), &BTreeSet::new())
        .expect("a gate subatom is the legal form");
}

#[test]
fn a_subatom_with_an_unknown_occurrence_is_rejected() {
    let query = normalized(vec![occurrence(0, 0, &[(1, X)])], vec![]);
    let plan = FjPlan {
        nodes: vec![Node {
            estimate: 0,
            subatoms: vec![subatom(0, &[X]), subatom(99, &[])],
        }],
    };
    assert_eq!(
        validate(&plan, &query, &schema(1, 3), &BTreeSet::new())
            .map(|_| ())
            .unwrap_err(),
        PlanError::UnknownOccurrence {
            node: 0,
            occ: OccId(99)
        }
    );
}

#[test]
fn a_subatom_over_a_negated_occurrence_is_rejected() {
    let query = normalized(
        vec![occurrence(0, 0, &[(1, X)]), negated(1, 1, &[(1, X)])],
        vec![],
    );
    let plan = FjPlan {
        nodes: vec![Node {
            estimate: 0,
            subatoms: vec![subatom(0, &[X]), subatom(1, &[])],
        }],
    };
    assert_eq!(
        validate(&plan, &query, &schema(2, 3), &BTreeSet::new())
            .map(|_| ())
            .unwrap_err(),
        PlanError::NonParticipatingOccurrenceInNode {
            node: 0,
            occ: OccId(1)
        }
    );
}

#[test]
fn anti_probe_attaches_to_the_earliest_all_bound_node() {

    let mut occurrences = clover().occurrences;
    occurrences.push(negated(3, 2, &[(1, X), (2, B)]));
    let query = normalized(occurrences, vec![]);
    let plan = binary2fj(&query, &order(&[0, 1, 2]));
    let validated = validate(&plan, &query, &schema(3, 3), &BTreeSet::new()).expect("valid plan");
    assert!(validated.nodes()[0].anti_probes.is_empty());
    assert_eq!(validated.nodes()[1].anti_probes.len(), 1);
    assert_eq!(validated.nodes()[1].anti_probes[0].occurrence, OccId(3));
    assert!(validated.nodes()[2].anti_probes.is_empty());
}

#[test]
fn root_only_anti_probes_attach_to_the_root() {
    let mut occurrences = clover().occurrences;
    occurrences.push(negated(3, 2, &[(1, X), (2, A)]));
    occurrences.push(negated(4, 1, &[]));
    let query = normalized(occurrences, vec![]);
    let plan = binary2fj(&query, &order(&[0, 1, 2]));
    let validated = validate(&plan, &query, &schema(3, 3), &BTreeSet::new()).expect("valid plan");
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

#[test]
fn negated_occurrences_get_probe_order_trie_schemas() {
    let mut occurrences = clover().occurrences;

    occurrences.push(negated(3, 2, &[(1, B), (2, X)]));
    let query = normalized(occurrences, vec![]);
    let plan = binary2fj(&query, &order(&[0, 1, 2]));
    let validated = validate(&plan, &query, &schema(3, 3), &BTreeSet::new()).expect("valid plan");
    assert_eq!(validated.occurrence(OccId(3)).trie_schema, vec![vec![X, B]]);
    assert_eq!(validated.occurrence(OccId(3)).key_widths, vec![2]);
}

#[test]
fn trie_schemas_match_the_papers_triangle_worked_example() {

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
                estimate: 0,
                subatoms: vec![subatom(0, &[X, Y]), subatom(1, &[Y]), subatom(2, &[X])],
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(1, &[Z]), subatom(2, &[Z])],
            },
        ],
    };
    let validated = validate(&plan, &query, &schema(3, 3), &BTreeSet::new()).expect("valid plan");
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

    let plan = FjPlan {
        nodes: vec![
            Node {
                estimate: 0,
                subatoms: vec![subatom(0, &[X]), subatom(1, &[X]), subatom(2, &[X])],
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(0, &[A])],
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(1, &[B])],
            },
            Node {
                estimate: 0,
                subatoms: vec![subatom(2, &[C])],
            },
        ],
    };
    let validated =
        validate(&plan, &clover(), &schema(3, 3), &BTreeSet::new()).expect("valid plan");
    assert_eq!(validated.nodes()[0].covers, vec![0, 1, 2]);
    assert_eq!(validated.nodes()[1].covers, vec![0]);
}

#[test]
fn residuals_attach_to_the_first_node_binding_both_sides() {

    let query = normalized(
        clover().occurrences,
        vec![FilterPredicate::FieldsCompare {
            left: OperandAddr::from(A),
            right: OperandAddr::from(B),
            op: WordCmp::Lt,
        }],
    );
    let plan = binary2fj(&query, &order(&[0, 1, 2]));
    let validated = validate(&plan, &query, &schema(3, 3), &BTreeSet::new()).expect("valid plan");
    assert!(validated.nodes()[0].residuals.is_empty());
    assert_eq!(validated.nodes()[1].residuals.len(), 1);
    assert!(validated.nodes()[2].residuals.is_empty());
}

/// The bug this pins: consuming one variables iterator across the node scan
/// leaves it exhausted after the first failing node, so the NEXT node passes
/// vacuously — a < c (bound at nodes 0 and 2) attached to node 1, where c is
/// unbound, and the executor compared against a zero slot.
#[test]
fn placement_rechecks_every_variable_at_every_node() {

    let query = normalized(
        clover().occurrences,
        vec![FilterPredicate::FieldsCompare {
            left: OperandAddr::from(A),
            right: OperandAddr::from(C),
            op: WordCmp::Lt,
        }],
    );
    let plan = binary2fj(&query, &order(&[0, 1, 2]));
    let validated = validate(&plan, &query, &schema(3, 3), &BTreeSet::new()).expect("valid plan");
    assert!(validated.nodes()[0].residuals.is_empty());
    assert!(validated.nodes()[1].residuals.is_empty());
    assert_eq!(validated.nodes()[2].residuals.len(), 1);

    let mut occurrences = clover().occurrences;
    occurrences.push(negated(3, 2, &[(1, A), (2, C)]));
    let query = normalized(occurrences, vec![]);
    let plan = binary2fj(&query, &order(&[0, 1, 2]));
    let validated = validate(&plan, &query, &schema(3, 3), &BTreeSet::new()).expect("valid plan");
    assert!(validated.nodes()[0].anti_probes.is_empty());
    assert!(validated.nodes()[1].anti_probes.is_empty());
    assert_eq!(validated.nodes()[2].anti_probes.len(), 1);
}

#[test]
fn self_join_plans_validate_over_occurrences() {

    let query = normalized(
        vec![
            occurrence(0, 0, &[(1, X), (2, Y)]),
            occurrence(1, 0, &[(1, Y), (2, Z)]),
        ],
        vec![],
    );
    let mut plan = binary2fj(&query, &order(&[0, 1]));
    factor(&mut plan);
    let validated =
        validate(&plan, &query, &schema(1, 3), &BTreeSet::new()).expect("self-joins validate");
    assert_eq!(validated.occurrences().len(), 2);
}

#[test]
fn duplicate_occurrence_within_a_node_is_rejected() {
    let plan = FjPlan {
        nodes: vec![Node {
            estimate: 0,
            subatoms: vec![subatom(0, &[X, A]), subatom(0, &[])],
        }],
    };
    let mut query = clover();
    query.occurrences.truncate(1);
    let err = validate(&plan, &query, &schema(3, 3), &BTreeSet::new()).unwrap_err();
    assert_eq!(
        err,
        PlanError::DuplicateOccurrenceInNode {
            node: 0,
            occ: OccId(0)
        }
    );
}

#[test]
fn distinct_witness_tracks_key_coverage() {

    let query = normalized(
        vec![
            occurrence(0, 0, &[(0, X), (1, A)]),
            occurrence(1, 1, &[(0, B), (1, X)]),
        ],
        vec![],
    );
    let plan = binary2fj(&query, &order(&[0, 1]));
    let validated = validate(&plan, &query, &schema(2, 2), &BTreeSet::new()).expect("valid plan");
    assert!(validated.distinct_witness().is_some());

    let query = normalized(
        vec![
            occurrence(0, 0, &[(0, X), (1, A)]),
            occurrence(1, 1, &[(1, X)]),
        ],
        vec![],
    );
    let plan = binary2fj(&query, &order(&[0, 1]));
    let validated = validate(&plan, &query, &schema(2, 2), &BTreeSet::new()).expect("valid plan");
    assert!(validated.distinct_witness().is_none());
}

#[test]
fn binding_slots_follow_node_order() {
    let query = clover();
    let mut plan = binary2fj(&query, &order(&[0, 1, 2]));
    factor(&mut plan);
    let validated = validate(&plan, &query, &schema(3, 3), &BTreeSet::new()).expect("valid plan");

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
