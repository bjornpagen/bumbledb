use super::*;
use crate::ir::CmpOp;
use std::collections::BTreeSet;

/// The string shape (docs/architecture/30-execution.md): one occurrence, `memo = ?0`
/// lowered as a filter by normalize, split into a selection here.
#[test]
fn lowering_splits_eq_constants_into_selections() {
    let mut occ = occurrence(0, 0, &[(1, X)]);
    occ.filters = vec![FilterPredicate::Compare {
        field: FieldId(2),
        op: CmpOp::Eq,
        value: Const::Param(crate::ir::ParamId(0)),
    }];
    let normalized = NormalizedQuery {
        occurrences: vec![occ],
        residuals: vec![],
    };
    let plan = binary2fj(&normalized, &order(&[0]));
    let validated = validate(&plan, &normalized, &schema(1, 3), vec![0], &BTreeSet::new())
        .expect("valid plan");
    let lowered = validated.occurrence(OccId(0));
    assert_eq!(
        lowered.selections,
        vec![Selection {
            field: FieldId(2),
            value: Const::Param(crate::ir::ParamId(0)),
        }]
    );
    assert!(lowered.filters.is_empty());
}

/// Range/Ne compares and every `FieldsCompare` stay filters; selections
/// come out ordered by field id whatever the filter order was.
#[test]
fn residuals_and_field_compares_stay_filters() {
    let mut occ = occurrence(0, 0, &[(1, X)]);
    occ.filters = vec![
        FilterPredicate::Compare {
            field: FieldId(2),
            op: CmpOp::Ge,
            value: Const::Word(9),
        },
        FilterPredicate::Compare {
            field: FieldId(2),
            op: CmpOp::Eq,
            value: Const::Word(5),
        },
        FilterPredicate::FieldsCompare {
            left: FieldId(1),
            right: FieldId(2),
            op: CmpOp::Eq,
        },
        FilterPredicate::Compare {
            field: FieldId(0),
            op: CmpOp::Eq,
            value: Const::Byte(1),
        },
    ];
    let normalized = NormalizedQuery {
        occurrences: vec![occ],
        residuals: vec![],
    };
    let plan = binary2fj(&normalized, &order(&[0]));
    let validated = validate(&plan, &normalized, &schema(1, 3), vec![0], &BTreeSet::new())
        .expect("valid plan");
    let lowered = validated.occurrence(OccId(0));
    assert_eq!(
        lowered.selections,
        vec![
            Selection {
                field: FieldId(0),
                value: Const::Byte(1),
            },
            Selection {
                field: FieldId(2),
                value: Const::Word(5),
            },
        ],
        "selections ordered by field id"
    );
    assert_eq!(
        lowered.filters,
        vec![
            FilterPredicate::Compare {
                field: FieldId(2),
                op: CmpOp::Ge,
                value: Const::Word(9),
            },
            FilterPredicate::FieldsCompare {
                left: FieldId(1),
                right: FieldId(2),
                op: CmpOp::Eq,
            },
        ],
        "residuals keep their order"
    );

    // Determinism: the same query lowers to the same plan.
    let again = validate(&plan, &normalized, &schema(1, 3), vec![0], &BTreeSet::new())
        .expect("valid plan");
    assert_eq!(validated.occurrences(), again.occurrences());
}

/// The boundary check: a hand-built occurrence that bypassed the split
/// (an Eq-constant still in `filters`) is rejected by name.
#[test]
fn a_leaked_eq_filter_fails_selection_validation() {
    let bad = PlanOccurrence {
        occ_id: OccId(3),
        relation: RelationId(0),
        vars: vec![],
        selections: vec![],
        filters: vec![FilterPredicate::Compare {
            field: FieldId(0),
            op: CmpOp::Eq,
            value: Const::Word(1),
        }],
        trie_schema: vec![],
    };
    assert_eq!(
        check_selections(std::slice::from_ref(&bad)),
        Err(PlanError::SelectionOnFilteredField { occ: OccId(3) })
    );
}
