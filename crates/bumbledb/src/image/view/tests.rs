use super::*;
use crate::encoding::{encode_i64, encode_u64};
use std::sync::Arc;

use crate::image::RelationImage;

fn view_apply(
    image: &Arc<RelationImage>,
    predicates: &[FilterPredicate],
    params: &[Const],
    buf: Vec<u32>,
) -> View {
    apply(
        image,
        predicates,
        params,
        buf,
        image.generation().text_eq(None),
    )
    .expect("apply")
}
use crate::image::testsupport::TestSource;
use crate::ir::ParamId;
use crate::ir::Value;
use crate::ir::WordCmp;
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::allen::AllenMask;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, IntervalElement, RelationDescriptor, RelationId, SchemaDescriptor,
    ValueType,
};

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "flag".into(),
                    value_type: ValueType::Bool,
                },
                FieldDescriptor {
                    name: "a".into(),
                    value_type: ValueType::I64,
                },
                FieldDescriptor {
                    name: "b".into(),
                    value_type: ValueType::I64,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const R: RelationId = RelationId(0);

type RRow = (u64, bool, i64, i64);

fn fixture_rows() -> Vec<RRow> {
    (0..50i64)
        .map(|i| {
            let b = if i % 5 == 0 { i - 25 } else { (i % 7) - 3 };
            (i.cast_unsigned(), i % 2 == 0, i - 25, b)
        })
        .collect()
}

fn populated(schema: &Schema) -> TestSource {
    let rows: Vec<Vec<Value>> = fixture_rows()
        .into_iter()
        .map(|(id, flag, a, b)| {
            vec![
                Value::U64(id),
                Value::Bool(flag),
                Value::I64(a),
                Value::I64(b),
            ]
        })
        .collect();
    TestSource::new(schema, &[(R, rows)])
}

/// The naive oracle: filter the fixture tuples directly. The image's row
/// order is canonical-byte order, which for a leading distinct u64 id is
/// ascending id — the same order the fixture generates.
fn oracle(keep: impl Fn(u64, bool, i64, i64) -> bool) -> Vec<u64> {
    fixture_rows()
        .into_iter()
        .filter(|(id, flag, a, b)| keep(*id, *flag, *a, *b))
        .map(|(id, ..)| id)
        .collect()
}

fn survivor_ids(view: &View) -> Vec<u64> {
    view.positions()
        .map(|p| view.bound().expect("apply binds").image().column_words(0)[p as usize])
        .collect()
}

#[test]
fn conjunction_over_mixed_width_fields_matches_the_naive_oracle() {
    let schema = schema();
    let source = populated(&schema);
    let (_cache, image) = source.image_with_cache(R);

    let predicates = vec![
        FilterPredicate::Compare {
            field: FieldId(1).into(),
            op: WordCmp::Eq,
            value: Const::Byte(1),
        },
        FilterPredicate::Compare {
            field: FieldId(2).into(),
            op: WordCmp::Ge,
            value: Const::Word(u64::from_be_bytes(encode_i64(-10))),
        },
        FilterPredicate::Compare {
            field: FieldId(2).into(),
            op: WordCmp::Lt,
            value: Const::Word(u64::from_be_bytes(encode_i64(15))),
        },
    ];
    let view = view_apply(&image, &predicates, &[], Vec::new());
    let expected = oracle(|_, flag, a, _| flag && (-10..15).contains(&a));
    assert_eq!(survivor_ids(&view), expected);
    assert!(!expected.is_empty(), "fixture exercises the filter");
}

#[test]
fn same_fact_field_equality_pairs_work() {
    let schema = schema();
    let source = populated(&schema);
    let (_cache, image) = source.image_with_cache(R);
    let predicates = vec![FilterPredicate::FieldsCompare {
        left: FieldId(2).into(),
        right: FieldId(3).into(),
        op: WordCmp::Eq,
    }];
    let view = view_apply(&image, &predicates, &[], Vec::new());
    let expected = oracle(|_, _, a, b| a == b);
    assert_eq!(survivor_ids(&view), expected);
    assert!(!expected.is_empty(), "fixture exercises the equality");
}

#[test]
fn unsatisfiable_filter_yields_an_empty_survivor_set() {
    let schema = schema();
    let source = populated(&schema);
    let (_cache, image) = source.image_with_cache(R);
    let predicates = vec![FilterPredicate::Compare {
        field: FieldId(0).into(),
        op: WordCmp::Eq,
        value: Const::Word(u64::MAX),
    }];
    let view = view_apply(&image, &predicates, &[], Vec::new());
    assert_eq!(view.len(), 0);
    assert!(view.is_empty());
    assert_eq!(view.positions().count(), 0);
}

#[test]
fn no_predicates_yield_the_all_variant() {
    let schema = schema();
    let source = populated(&schema);
    let (_cache, image) = source.image_with_cache(R);
    let view = view_apply(&image, &[], &[], Vec::new());
    assert!(matches!(view, View::Bound(BoundView::All(_))));
    assert_eq!(view.len(), 50);
    let positions: Vec<u32> = view.positions().collect();
    assert_eq!(positions, (0..50).collect::<Vec<u32>>());
}

#[test]
fn reapplying_the_same_filter_to_the_built_image_is_stable() {
    let schema = schema();
    let source = populated(&schema);
    let (_cache, image) = source.image_with_cache(R);
    let predicates = vec![FilterPredicate::Compare {
        field: FieldId(0).into(),
        op: WordCmp::Ge,
        value: Const::Word(u64::from_be_bytes(encode_u64(40))),
    }];

    let view = view_apply(&image, &predicates, &[], Vec::new());
    let reapplied = view_apply(&image, &predicates, &[], Vec::new());
    assert_eq!(
        view.positions().collect::<Vec<_>>(),
        reapplied.positions().collect::<Vec<_>>()
    );
    assert_eq!(view.len(), 10);
}

fn interval_schema() -> Schema {
    let interval_i64 = ValueType::Interval {
        element: IntervalElement::I64,
    };
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
    };
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "P".into(),
            fields: vec![
                field("id", ValueType::U64),
                field("during", interval_i64),
                field("review", interval_i64),
                field("at", ValueType::I64),
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const P: RelationId = RelationId(0);
const P_ID: FieldId = FieldId(0);
const P_DURING: FieldId = FieldId(1);
const P_REVIEW: FieldId = FieldId(2);
const P_AT: FieldId = FieldId(3);

type PRow = (u64, (i64, i64), (i64, i64), i64);

const P_ROWS: [PRow; 5] = [
    (1, (2, 9), (2, 5), 2),
    (2, (9, 12), (9, 10), 9),
    (3, (-5, 2), (-6, 1), 2),
    (4, (0, 4), (4, 8), 4),
    (5, (1, 3), (1, 3), 1),
];

fn w(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

fn sorted_ids(view: &View) -> Vec<u64> {
    let mut ids = survivor_ids(view);
    ids.sort_unstable();
    ids
}

fn interval_image() -> std::sync::Arc<crate::image::RelationImage> {
    let schema = interval_schema();
    let rows: Vec<Vec<Value>> = P_ROWS
        .iter()
        .map(|(id, during, review, at)| {
            vec![
                Value::U64(*id),
                Value::IntervalI64(
                    bumbledb_theory::Interval::<i64>::new(during.0, during.1)
                        .expect("nonempty interval"),
                ),
                Value::IntervalI64(
                    bumbledb_theory::Interval::<i64>::new(review.0, review.1)
                        .expect("nonempty interval"),
                ),
                Value::I64(*at),
            ]
        })
        .collect();
    let source = TestSource::new(&schema, &[(P, rows)]);
    let (_cache, image) = source.image_with_cache(P);
    image
}

#[test]
fn point_in_keeps_start_boundary_and_drops_end_boundary() {
    let image = interval_image();

    let at_nine = vec![FilterPredicate::PointIn {
        field: P_DURING.into(),
        point: ViewWordSource::Word(w(9)),
        dense: false,
    }];
    assert_eq!(sorted_ids(&view_apply(&image, &at_nine, &[], Vec::new())), [2]);

    let at_two = vec![FilterPredicate::PointIn {
        field: P_DURING.into(),
        point: ViewWordSource::Word(w(2)),
        dense: false,
    }];
    assert_eq!(
        sorted_ids(&view_apply(&image, &at_two, &[], Vec::new())),
        [1, 4, 5]
    );

    let via_param = vec![FilterPredicate::PointIn {
        field: P_DURING.into(),
        point: ViewWordSource::Param(ParamId(0)),
        dense: false,
    }];
    assert_eq!(
        sorted_ids(&view_apply(&image, &via_param, &[Const::Word(w(9))], Vec::new())),
        [2]
    );
}

#[test]
fn any_point_in_matches_any_element_of_the_bound_set() {
    let image = interval_image();
    let predicates = vec![FilterPredicate::AnyPointIn {
        field: P_DURING.into(),
        set: SetConst::ParamSet(ParamId(0)),
        dense: false,
    }];

    let params = [Const::WordSet(vec![w(-4), w(10)])];
    assert_eq!(
        sorted_ids(&view_apply(&image, &predicates, &params, Vec::new())),
        [2, 3]
    );

    let empty = [Const::WordSet(Vec::new())];
    assert!(view_apply(&image, &predicates, &empty, Vec::new()).is_empty());
}

#[test]
fn same_atom_interval_shapes_evaluate_their_fixed_compositions() {
    let image = interval_image();
    let run =
        |predicate: FilterPredicate| sorted_ids(&view_apply(&image, &[predicate], &[], Vec::new()));

    assert_eq!(
        run(FilterPredicate::FieldsAllen {
            left: P_DURING.into(),
            right: P_REVIEW.into(),
            mask: AllenMask::INTERSECTS,
        }),
        [1, 2, 3, 5]
    );

    assert_eq!(
        run(FilterPredicate::FieldsAllen {
            left: P_DURING.into(),
            right: P_REVIEW.into(),
            mask: AllenMask::COVERS,
        }),
        [1, 2, 5]
    );

    assert_eq!(
        run(FilterPredicate::FieldsAllen {
            left: P_DURING.into(),
            right: P_REVIEW.into(),
            mask: AllenMask::EQUALS,
        }),
        [5]
    );

    assert_eq!(
        run(FilterPredicate::FieldsPointIn {
            interval: P_DURING.into(),
            point: P_AT.into(),
            dense: false,
        }),
        [1, 2, 5]
    );

    assert_eq!(
        run(FilterPredicate::FieldsCompare {
            left: P_DURING.into(),
            right: P_REVIEW.into(),
            op: WordCmp::Eq,
        }),
        [5]
    );
    assert_eq!(
        run(FilterPredicate::FieldsCompare {
            left: P_DURING.into(),
            right: P_REVIEW.into(),
            op: WordCmp::Ne,
        }),
        [1, 2, 3, 4]
    );
}

#[test]
fn field_within_is_scalar_membership_in_the_constant_interval() {
    let image = interval_image();

    let scalar_within = vec![FilterPredicate::FieldWithin {
        field: P_AT.into(),
        outer: IntervalConst::Interval {
            start: w(2),
            end: w(9),
        },
        dense: false,
    }];
    assert_eq!(
        sorted_ids(&view_apply(&image, &scalar_within, &[], Vec::new())),
        [1, 3, 4]
    );
}

#[test]
fn interval_constants_compare_pairwise_under_eq() {
    let image = interval_image();
    let predicates = vec![FilterPredicate::Compare {
        field: P_DURING.into(),
        op: WordCmp::Eq,
        value: Const::Interval {
            start: w(2),
            end: w(9),
        },
    }];
    assert_eq!(
        sorted_ids(&view_apply(&image, &predicates, &[], Vec::new())),
        [1]
    );
}

#[test]
fn param_set_eq_matches_any_element_over_a_scalar_column() {
    let image = interval_image();
    let predicates = vec![FilterPredicate::Compare {
        field: P_ID.into(),
        op: WordCmp::Eq,
        value: Const::ParamSet(ParamId(0)),
    }];
    let params = [Const::WordSet(vec![1u64, 3])];
    assert_eq!(
        sorted_ids(&view_apply(&image, &predicates, &params, Vec::new())),
        [1, 3]
    );
}
