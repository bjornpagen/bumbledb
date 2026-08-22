use super::*;
use crate::image::view::{Const, FilterPredicate};
use crate::ir::WordCmp;
use std::collections::BTreeSet;

#[test]
fn lowering_splits_eq_constants_into_selections() {
    let mut occ = occurrence(0, 0, &[(1, X)]);
    occ.filters = vec![FilterPredicate::Compare {
        field: FieldId(2).into(),
        op: WordCmp::Eq,
        value: Const::Param(crate::ir::ParamId(0)),
    }];
    let query = normalized(vec![occ], vec![]);
    let plan = binary2fj(&query, &order(&[0]));
    let validated = validate(&plan, &query, &schema(1, 3), &BTreeSet::new()).expect("valid plan");
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

#[test]
fn residuals_and_field_compares_stay_filters() {
    let mut occ = occurrence(0, 0, &[(1, X)]);
    occ.filters = vec![
        FilterPredicate::Compare {
            field: FieldId(2).into(),
            op: WordCmp::Ge,
            value: Const::Word(9),
        },
        FilterPredicate::Compare {
            field: FieldId(2).into(),
            op: WordCmp::Eq,
            value: Const::Word(5),
        },
        FilterPredicate::FieldsCompare {
            left: FieldId(1).into(),
            right: FieldId(2).into(),
            op: WordCmp::Eq,
        },
        FilterPredicate::Compare {
            field: FieldId(0).into(),
            op: WordCmp::Eq,
            value: Const::Byte(1),
        },
    ];
    let query = normalized(vec![occ], vec![]);
    let plan = binary2fj(&query, &order(&[0]));
    let validated = validate(&plan, &query, &schema(1, 3), &BTreeSet::new()).expect("valid plan");
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
                field: FieldId(2).into(),
                op: WordCmp::Ge,
                value: Const::Word(9),
            },
            FilterPredicate::FieldsCompare {
                left: FieldId(1).into(),
                right: FieldId(2).into(),
                op: WordCmp::Eq,
            },
        ],
        "residuals keep their order"
    );

    let again = validate(&plan, &query, &schema(1, 3), &BTreeSet::new()).expect("valid plan");
    assert_eq!(validated.occurrences(), again.occurrences());
}

#[test]
fn a_leaked_eq_filter_fails_selection_validation() {
    let bad = PlanOccurrence {
        occ_id: OccId(3),
        bind: OccBind::Edb(RelationId(0)),
        role: crate::ir::normalize::Role::Positive,
        vars: vec![],
        selections: vec![],
        filters: vec![FilterPredicate::Compare {
            field: FieldId(0).into(),
            op: WordCmp::Eq,
            value: Const::Word(1),
        }],
        point_filters: vec![],
        spans: Box::new([]),
        trie_schema: vec![],
        key_widths: vec![],
    };
    assert_eq!(
        check_selections(std::slice::from_ref(&bad)),
        Err(PlanError::SelectionOnFilteredField { occ: OccId(3) })
    );
}
