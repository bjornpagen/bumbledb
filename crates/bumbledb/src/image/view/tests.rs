use super::*;
use crate::encoding::{ValueRef, decode_field, encode_fact, encode_i64, encode_u64};
use crate::error::Result as DbResult;
use crate::image::build;
use crate::ir::ParamId;
use crate::ir::{OrderCmp, WordCmp};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::read;
use crate::testutil::TempDir;
use bumbledb_theory::allen::AllenMask;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, RelationId,
    SchemaDescriptor, ValueType,
};

/// R(id u64, flag bool, a i64, b i64).
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "R".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "flag".into(),
                    value_type: ValueType::Bool,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "a".into(),
                    value_type: ValueType::I64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "b".into(),
                    value_type: ValueType::I64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const R: RelationId = RelationId(0);

fn fact(schema: &Schema, id: u64, flag: bool, a: i64, b: i64) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode_fact(
        &[
            ValueRef::U64(id),
            ValueRef::Bool(flag),
            ValueRef::I64(a),
            ValueRef::I64(b),
        ],
        schema.relation(R).layout(),
        &mut bytes,
    );
    bytes
}

fn populated(dir: &TempDir, schema: &Schema) -> Environment {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for i in 0..50i64 {
        let id = i.cast_unsigned();
        // Every fifth row has a == b so the equality filter has matches.
        let b = if i % 5 == 0 { i - 25 } else { (i % 7) - 3 };
        delta
            .insert(&view, R, &fact(schema, id, i % 2 == 0, i - 25, b))
            .expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit").expect("admitted");
    env
}

/// The naive oracle: per-row decode via the fact codec, no images.
fn oracle(
    env: &Environment,
    schema: &Schema,
    keep: impl Fn(u64, bool, i64, i64) -> bool,
) -> Vec<u64> {
    let txn = env.read_txn().expect("txn");
    read::scan(&txn, schema, R)
        .expect("scan")
        .map(|entry| {
            let (_, bytes) = entry.expect("ok");
            let id = match decode_field(bytes, 0).expect("decode") {
                crate::encoding::ValueRef::U64(v) => v,
                other => panic!("{other:?}"),
            };
            let flag = match decode_field(bytes, 1).expect("decode") {
                crate::encoding::ValueRef::Bool(v) => v,
                other => panic!("{other:?}"),
            };
            let a = match decode_field(bytes, 2).expect("decode") {
                crate::encoding::ValueRef::I64(v) => v,
                other => panic!("{other:?}"),
            };
            let b = match decode_field(bytes, 3).expect("decode") {
                crate::encoding::ValueRef::I64(v) => v,
                other => panic!("{other:?}"),
            };
            (id, flag, a, b)
        })
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
    let dir = TempDir::new("view-conjunction");
    let schema = schema();
    let env = populated(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let image = build(&txn.catalog(), &schema, R).expect("build");

    // flag == true AND a >= -10 AND a < 15
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
    let view = apply(&image, &predicates, &[], Vec::new());
    let expected = oracle(&env, &schema, |_, flag, a, _| {
        flag && (-10..15).contains(&a)
    });
    assert_eq!(survivor_ids(&view), expected);
    assert!(!expected.is_empty(), "fixture exercises the filter");
}

#[test]
fn same_fact_field_equality_pairs_work() {
    let dir = TempDir::new("view-fields-equal");
    let schema = schema();
    let env = populated(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let image = build(&txn.catalog(), &schema, R).expect("build");
    let predicates = vec![FilterPredicate::FieldsCompare {
        left: FieldId(2).into(),
        right: FieldId(3).into(),
        op: WordCmp::Eq,
    }];
    let view = apply(&image, &predicates, &[], Vec::new());
    let expected = oracle(&env, &schema, |_, _, a, b| a == b);
    assert_eq!(survivor_ids(&view), expected);
    assert!(!expected.is_empty(), "fixture exercises the equality");
}

#[test]
fn unsatisfiable_filter_yields_an_empty_survivor_set() {
    let dir = TempDir::new("view-empty");
    let schema = schema();
    let env = populated(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let image = build(&txn.catalog(), &schema, R).expect("build");
    let predicates = vec![FilterPredicate::Compare {
        field: FieldId(0).into(),
        op: WordCmp::Eq,
        value: Const::Word(u64::MAX),
    }];
    let view = apply(&image, &predicates, &[], Vec::new());
    assert_eq!(view.len(), 0);
    assert!(view.is_empty());
    assert_eq!(view.positions().count(), 0);
}

#[test]
fn no_predicates_yield_the_all_variant() {
    let dir = TempDir::new("view-all");
    let schema = schema();
    let env = populated(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let image = build(&txn.catalog(), &schema, R).expect("build");
    let view = apply(&image, &[], &[], Vec::new());
    assert!(matches!(view, View::Bound(BoundView::All(_))));
    assert_eq!(view.len(), 50);
    let positions: Vec<u32> = view.positions().collect();
    assert_eq!(positions, (0..50).collect::<Vec<u32>>());
}

#[test]
fn cold_dual_output_matches_separate_build_and_apply() -> DbResult<()> {
    let dir = TempDir::new("view-dual-output");
    let schema = schema();
    let env = populated(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let predicates = vec![FilterPredicate::Compare {
        field: FieldId(0).into(),
        op: WordCmp::Ge,
        value: Const::Word(u64::from_be_bytes(encode_u64(40))),
    }];

    let (image, view) = build_with_filters(&txn, &schema, R, &predicates, &[], Vec::new())?;
    let reference = build(&txn.catalog(), &schema, R)?;
    // Byte-identical columns (addresses differ; contents must not).
    assert_eq!(image.row_count(), reference.row_count());
    for field in 0..4 {
        assert_eq!(image.column(field), reference.column(field));
    }
    // ...and the view equals apply() over that image.
    let reapplied = apply(&image, &predicates, &[], Vec::new());
    assert_eq!(
        view.positions().collect::<Vec<_>>(),
        reapplied.positions().collect::<Vec<_>>()
    );
    assert_eq!(view.len(), 10);
    Ok(())
}

// --- the interval filter kinds (PRD 14, scalar path) ------------------------

/// P(id u64, during interval<i64>, review interval<i64>, at i64) — columns
/// 0, (1, 2), (3, 4), 5.
fn interval_schema() -> Schema {
    let interval_i64 = ValueType::Interval {
        element: IntervalElement::I64,
    };
    let field = |name: &str, ty: ValueType| FieldDescriptor {
        name: name.into(),
        value_type: ty,
        generation: Generation::None,
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

/// One fixture row: `(id, during, review, at)`.
type PRow = (u64, (i64, i64), (i64, i64), i64);

/// The rows, chosen so every interval shape and both membership
/// boundaries discriminate.
const P_ROWS: [PRow; 5] = [
    (1, (2, 9), (2, 5), 2),
    (2, (9, 12), (9, 10), 9),
    (3, (-5, 2), (-6, 1), 2),
    (4, (0, 4), (4, 8), 4),
    (5, (1, 3), (1, 3), 1),
];

/// The biased I64 column word.
fn w(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

/// Survivor ids in ascending id order (scan order is content-hash order,
/// so set comparisons sort).
fn sorted_ids(view: &View) -> Vec<u64> {
    let mut ids = survivor_ids(view);
    ids.sort_unstable();
    ids
}

fn interval_image(dir: &TempDir) -> std::sync::Arc<crate::image::RelationImage> {
    let schema = interval_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    for (id, during, review, at) in P_ROWS {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(id),
                ValueRef::IntervalI64(
                    bumbledb_theory::Interval::<i64>::new(during.0, during.1)
                        .expect("nonempty interval"),
                ),
                ValueRef::IntervalI64(
                    bumbledb_theory::Interval::<i64>::new(review.0, review.1)
                        .expect("nonempty interval"),
                ),
                ValueRef::I64(at),
            ],
            schema.relation(P).layout(),
            &mut bytes,
        );
        delta.insert(&view, P, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit").expect("admitted");
    let txn = env.read_txn().expect("txn");
    build(&txn.catalog(), &schema, P).expect("build")
}

/// PRD 14 criterion: `PointIn` survives exactly the rows whose interval
/// contains the point — `point == start` survives, `point == end` does
/// not (the half-open boundary).
#[test]
fn point_in_keeps_start_boundary_and_drops_end_boundary() {
    let dir = TempDir::new("view-point-in");
    let image = interval_image(&dir);

    // 9 == start of [9,12) (row 2, survives) and == end of [2,9)
    // (row 1, dies).
    let at_nine = vec![FilterPredicate::PointIn {
        field: P_DURING.into(),
        point: ViewWordSource::Word(w(9)),
    }];
    assert_eq!(sorted_ids(&apply(&image, &at_nine, &[], Vec::new())), [2]);

    // 2 == start of [2,9) (survives), == end of [-5,2) (dies), and an
    // interior point of [1,3).
    let at_two = vec![FilterPredicate::PointIn {
        field: P_DURING.into(),
        point: ViewWordSource::Word(w(2)),
    }];
    assert_eq!(
        sorted_ids(&apply(&image, &at_two, &[], Vec::new())),
        [1, 4, 5]
    );

    // The same point through the bind-time param slice.
    let via_param = vec![FilterPredicate::PointIn {
        field: P_DURING.into(),
        point: ViewWordSource::Param(ParamId(0)),
    }];
    assert_eq!(
        sorted_ids(&apply(&image, &via_param, &[Const::Word(w(9))], Vec::new())),
        [2]
    );
}

#[test]
fn any_point_in_matches_any_element_of_the_bound_set() {
    let dir = TempDir::new("view-any-point-in");
    let image = interval_image(&dir);
    let predicates = vec![FilterPredicate::AnyPointIn {
        field: P_DURING.into(),
        set: SetConst::ParamSet(ParamId(0)),
    }];

    // {-4, 10}: -4 lies in [-5,2) (row 3), 10 in [9,12) (row 2).
    let params = [Const::WordSet(vec![w(-4), w(10)])];
    assert_eq!(
        sorted_ids(&apply(&image, &predicates, &params, Vec::new())),
        [2, 3]
    );

    // The empty set lies in no interval.
    let empty = [Const::WordSet(Vec::new())];
    assert!(apply(&image, &predicates, &empty, Vec::new()).is_empty());
}

#[test]
fn same_atom_interval_shapes_evaluate_their_fixed_compositions() {
    let dir = TempDir::new("view-interval-shapes");
    let image = interval_image(&dir);
    let run =
        |predicate: FilterPredicate| sorted_ids(&apply(&image, &[predicate], &[], Vec::new()));

    // INTERSECTS: the point-sets share a point (the 9-bit composite).
    assert_eq!(
        run(FilterPredicate::FieldsAllen {
            left: P_DURING.into(),
            right: P_REVIEW.into(),
            mask: AllenMask::INTERSECTS,
        }),
        [1, 2, 3, 5]
    );
    // COVERS (⊇): equals ∪ contains ∪ started-by ∪ finished-by.
    assert_eq!(
        run(FilterPredicate::FieldsAllen {
            left: P_DURING.into(),
            right: P_REVIEW.into(),
            mask: AllenMask::COVERS,
        }),
        [1, 2, 5]
    );
    // A singleton basic: exact equality through the algebra.
    assert_eq!(
        run(FilterPredicate::FieldsAllen {
            left: P_DURING.into(),
            right: P_REVIEW.into(),
            mask: AllenMask::EQUALS,
        }),
        [5]
    );
    // Point membership as a same-fact composition, half-open on both
    // fixture boundaries (rows 1 and 2 sit at start, rows 3 and 4 at end).
    assert_eq!(
        run(FilterPredicate::FieldsPointIn {
            interval: P_DURING.into(),
            point: P_AT.into(),
        }),
        [1, 2, 5]
    );
    // Interval fields compare pairwise over their two-word spans.
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
    let dir = TempDir::new("view-field-within");
    let image = interval_image(&dir);

    // Scalar field within [2,9): membership with the half-open boundary
    // (at == 2 survives, at == 9 dies).
    let scalar_within = vec![FilterPredicate::FieldWithin {
        field: P_AT.into(),
        outer: IntervalConst::Interval {
            start: w(2),
            end: w(9),
        },
    }];
    assert_eq!(
        sorted_ids(&apply(&image, &scalar_within, &[], Vec::new())),
        [1, 3, 4]
    );
}

#[test]
fn interval_constants_compare_pairwise_under_eq() {
    let dir = TempDir::new("view-interval-const");
    let image = interval_image(&dir);
    let predicates = vec![FilterPredicate::Compare {
        field: P_DURING.into(),
        op: WordCmp::Eq,
        value: Const::Interval {
            start: w(2),
            end: w(9),
        },
    }];
    assert_eq!(
        sorted_ids(&apply(&image, &predicates, &[], Vec::new())),
        [1]
    );
}

#[test]
fn param_set_eq_matches_any_element_over_a_scalar_column() {
    let dir = TempDir::new("view-param-set");
    let image = interval_image(&dir);
    let predicates = vec![FilterPredicate::Compare {
        field: P_ID.into(),
        op: WordCmp::Eq,
        value: Const::ParamSet(ParamId(0)),
    }];
    let params = [Const::WordSet(vec![1u64, 3])];
    assert_eq!(
        sorted_ids(&apply(&image, &predicates, &params, Vec::new())),
        [1, 3]
    );
}

/// The measure path (findings 051/115): the pooled survivor buffer
/// round-trips through an all-measure apply instead of being dropped,
/// the mixed list runs infallible-first over the SAME borrowed slice
/// (no partition, no predicate clones), and the fields-measure arm
/// dispatches on the View variant directly.
#[test]
fn measure_filters_keep_the_pooled_buffer_and_refine_in_order() {
    let dir = TempDir::new("view-measure-path");
    let image = interval_image(&dir);
    // Durations of P_ROWS' `during`: 7, 3, 7, 4, 2 (by id).
    let dur_lt = |bound: u64| FilterPredicate::DurationCompare {
        field: P_DURING.into(),
        op: OrderCmp::Lt,
        value: WordOrParam::Word(bound),
    };

    // All-measure: the caller's pooled buffer seeds the survivors (051).
    let buf = Vec::with_capacity(64);
    let ptr = buf.as_ptr();
    let view = apply(&image, &[dur_lt(4)], &[], buf);
    assert_eq!(sorted_ids(&view), [2, 5]);
    let recycled = view.recycle();
    assert_eq!(
        recycled.as_ptr(),
        ptr,
        "the pooled survivor buffer round-trips through the all-measure path"
    );

    // Mixed list: the infallible predicate runs first over the same
    // borrowed slice, the measure refines its survivors (order law).
    let mixed = vec![
        FilterPredicate::Compare {
            field: P_ID.into(),
            op: WordCmp::Ge,
            value: Const::Word(3),
        },
        dur_lt(5),
    ];
    assert_eq!(sorted_ids(&apply(&image, &mixed, &[], recycled)), [4, 5]);

    // The fields measure through both live variant arms (115): the
    // identity extension (an All input) and the in-place survivor
    // refinement (a mixed list). Durations vs the u64 id column:
    // only id 5 (duration 2) sits strictly under its id.
    let fields_lt = FilterPredicate::DurationFieldsCompare {
        interval: P_DURING.into(),
        op: OrderCmp::Lt,
        scalar: P_ID.into(),
    };
    assert_eq!(
        sorted_ids(&apply(
            &image,
            std::slice::from_ref(&fields_lt),
            &[],
            Vec::new()
        )),
        [5]
    );
    let mixed = vec![
        FilterPredicate::Compare {
            field: P_ID.into(),
            op: WordCmp::Ge,
            value: Const::Word(4),
        },
        fields_lt,
    ];
    assert_eq!(sorted_ids(&apply(&image, &mixed, &[], Vec::new())), [5]);
}
