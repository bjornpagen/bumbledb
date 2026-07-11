use super::*;
use crate::allen::AllenMask;
use crate::encoding::{decode_field, encode_fact, encode_i64, encode_u64, ValueRef};
use crate::error::Result as DbResult;
use crate::image::build;
use crate::ir::ParamId;
use crate::schema::{
    FieldDescriptor, Generation, IntervalElement, RelationDescriptor, RelationId, Schema,
    SchemaDescriptor, ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::env::Environment;
use crate::storage::read;
use crate::testutil::TempDir;

/// R(id u64, flag bool, a i64, b i64).
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
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
    commit(delta, &env).expect("commit");
    env
}

/// The naive oracle: per-row decode via the fact codec, no images.
fn oracle(
    env: &Environment,
    schema: &Schema,
    keep: impl Fn(u64, bool, i64, i64) -> bool,
) -> Vec<u64> {
    let txn = env.read_txn().expect("txn");
    let layout = schema.relation(R).layout();
    read::scan(&txn, schema, R)
        .expect("scan")
        .map(|entry| {
            let (_, bytes) = entry.expect("ok");
            let id = match decode_field(bytes, layout, 0).expect("decode") {
                crate::encoding::ValueRef::U64(v) => v,
                other => panic!("{other:?}"),
            };
            let flag = match decode_field(bytes, layout, 1).expect("decode") {
                crate::encoding::ValueRef::Bool(v) => v,
                other => panic!("{other:?}"),
            };
            let a = match decode_field(bytes, layout, 2).expect("decode") {
                crate::encoding::ValueRef::I64(v) => v,
                other => panic!("{other:?}"),
            };
            let b = match decode_field(bytes, layout, 3).expect("decode") {
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
        .map(|p| view.image().column_words(0)[p as usize])
        .collect()
}

#[test]
fn conjunction_over_mixed_width_fields_matches_the_naive_oracle() {
    let dir = TempDir::new("view-conjunction");
    let schema = schema();
    let env = populated(&dir, &schema);
    let txn = env.read_txn().expect("txn");
    let image = build(&txn, &schema, R).expect("build");

    // flag == true AND a >= -10 AND a < 15
    let predicates = vec![
        FilterPredicate::Compare {
            field: FieldId(1),
            op: CmpOp::Eq,
            value: Const::Byte(1),
        },
        FilterPredicate::Compare {
            field: FieldId(2),
            op: CmpOp::Ge,
            value: Const::Word(u64::from_be_bytes(encode_i64(-10))),
        },
        FilterPredicate::Compare {
            field: FieldId(2),
            op: CmpOp::Lt,
            value: Const::Word(u64::from_be_bytes(encode_i64(15))),
        },
    ];
    let view = apply(&image, &predicates, &[], Vec::new()).expect("no measure filters");
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
    let image = build(&txn, &schema, R).expect("build");
    let predicates = vec![FilterPredicate::FieldsCompare {
        left: FieldId(2),
        right: FieldId(3),
        op: CmpOp::Eq,
    }];
    let view = apply(&image, &predicates, &[], Vec::new()).expect("no measure filters");
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
    let image = build(&txn, &schema, R).expect("build");
    let predicates = vec![FilterPredicate::Compare {
        field: FieldId(0),
        op: CmpOp::Eq,
        value: Const::Word(u64::MAX),
    }];
    let view = apply(&image, &predicates, &[], Vec::new()).expect("no measure filters");
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
    let image = build(&txn, &schema, R).expect("build");
    let view = apply(&image, &[], &[], Vec::new()).expect("no measure filters");
    assert!(matches!(view, View::All(_)));
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
        field: FieldId(0),
        op: CmpOp::Ge,
        value: Const::Word(u64::from_be_bytes(encode_u64(40))),
    }];

    let (image, view) = build_with_filters(&txn, &schema, R, &predicates, &[], Vec::new())?;
    let reference = build(&txn, &schema, R)?;
    // Byte-identical columns (addresses differ; contents must not).
    assert_eq!(image.row_count(), reference.row_count());
    for field in 0..4 {
        assert_eq!(image.column(field), reference.column(field));
    }
    // ...and the view equals apply() over that image.
    let reapplied = apply(&image, &predicates, &[], Vec::new()).expect("no measure filters");
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
            name: "P".into(),
            fields: vec![
                field("id", ValueType::U64),
                field("during", interval_i64.clone()),
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
                ValueRef::IntervalI64(during.0, during.1),
                ValueRef::IntervalI64(review.0, review.1),
                ValueRef::I64(at),
            ],
            schema.relation(P).layout(),
            &mut bytes,
        );
        delta.insert(&view, P, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    let txn = env.read_txn().expect("txn");
    build(&txn, &schema, P).expect("build")
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
        field: P_DURING,
        point: ResolvedWordSource::Word(w(9)),
    }];
    assert_eq!(
        sorted_ids(&apply(&image, &at_nine, &[], Vec::new()).expect("no measure filters")),
        [2]
    );

    // 2 == start of [2,9) (survives), == end of [-5,2) (dies), and an
    // interior point of [1,3).
    let at_two = vec![FilterPredicate::PointIn {
        field: P_DURING,
        point: ResolvedWordSource::Word(w(2)),
    }];
    assert_eq!(
        sorted_ids(&apply(&image, &at_two, &[], Vec::new()).expect("no measure filters")),
        [1, 4, 5]
    );

    // The same point through the bind-time param slice.
    let via_param = vec![FilterPredicate::PointIn {
        field: P_DURING,
        point: ResolvedWordSource::Param(ParamId(0)),
    }];
    assert_eq!(
        sorted_ids(
            &apply(&image, &via_param, &[Const::Word(w(9))], Vec::new())
                .expect("no measure filters")
        ),
        [2]
    );
}

#[test]
fn any_point_in_matches_any_element_of_the_bound_set() {
    let dir = TempDir::new("view-any-point-in");
    let image = interval_image(&dir);
    let predicates = vec![FilterPredicate::AnyPointIn {
        field: P_DURING,
        set: Const::ParamSet(ParamId(0)),
    }];

    // {-4, 10}: -4 lies in [-5,2) (row 3), 10 in [9,12) (row 2).
    let params = [Const::WordSet(vec![w(-4), w(10)])];
    assert_eq!(
        sorted_ids(&apply(&image, &predicates, &params, Vec::new()).expect("no measure filters")),
        [2, 3]
    );

    // The empty set lies in no interval.
    let empty = [Const::WordSet(Vec::new())];
    assert!(apply(&image, &predicates, &empty, Vec::new())
        .expect("no measure filters")
        .is_empty());
}

#[test]
fn same_atom_interval_shapes_evaluate_their_fixed_compositions() {
    let dir = TempDir::new("view-interval-shapes");
    let image = interval_image(&dir);
    let run = |predicate: FilterPredicate| {
        sorted_ids(&apply(&image, &[predicate], &[], Vec::new()).expect("no measure filters"))
    };

    // INTERSECTS: the point-sets share a point (the 9-bit composite).
    assert_eq!(
        run(FilterPredicate::FieldsAllen {
            left: P_DURING,
            right: P_REVIEW,
            mask: MaskConst::Mask(AllenMask::INTERSECTS),
        }),
        [1, 2, 3, 5]
    );
    // COVERS (⊇): equals ∪ contains ∪ started-by ∪ finished-by.
    assert_eq!(
        run(FilterPredicate::FieldsAllen {
            left: P_DURING,
            right: P_REVIEW,
            mask: MaskConst::Mask(AllenMask::COVERS),
        }),
        [1, 2, 5]
    );
    // A singleton basic: exact equality through the algebra.
    assert_eq!(
        run(FilterPredicate::FieldsAllen {
            left: P_DURING,
            right: P_REVIEW,
            mask: MaskConst::Mask(AllenMask::EQUALS),
        }),
        [5]
    );
    // Point membership as a same-fact composition, half-open on both
    // fixture boundaries (rows 1 and 2 sit at start, rows 3 and 4 at end).
    assert_eq!(
        run(FilterPredicate::FieldsContainPoint {
            interval: P_DURING,
            point: P_AT,
        }),
        [1, 2, 5]
    );
    // Interval fields compare pairwise over their two-word spans.
    assert_eq!(
        run(FilterPredicate::FieldsCompare {
            left: P_DURING,
            right: P_REVIEW,
            op: CmpOp::Eq,
        }),
        [5]
    );
    assert_eq!(
        run(FilterPredicate::FieldsCompare {
            left: P_DURING,
            right: P_REVIEW,
            op: CmpOp::Ne,
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
        field: P_AT,
        outer: Const::Interval {
            start: w(2),
            end: w(9),
        },
    }];
    assert_eq!(
        sorted_ids(&apply(&image, &scalar_within, &[], Vec::new()).expect("no measure filters")),
        [1, 3, 4]
    );
}

#[test]
fn field_allen_classifies_against_the_constant_interval() {
    let dir = TempDir::new("view-field-allen");
    let image = interval_image(&dir);
    let run = |mask: MaskConst, start: i64, end: i64, params: &[Const]| {
        let predicates = vec![FilterPredicate::FieldAllen {
            field: P_DURING,
            other: if params.is_empty() || matches!(params[0], Const::Word(_)) {
                Const::Interval {
                    start: w(start),
                    end: w(end),
                }
            } else {
                Const::Param(ParamId(0))
            },
            mask,
        }];
        sorted_ids(&apply(&image, &predicates, params, Vec::new()).expect("no measure filters"))
    };

    // Value equality and its complement — the Eq/Ne derived facts.
    assert_eq!(run(MaskConst::Mask(AllenMask::EQUALS), 2, 9, &[]), [1]);
    assert_eq!(
        run(MaskConst::Mask(AllenMask::EQUALS.complement()), 2, 9, &[]),
        [2, 3, 4, 5]
    );
    // The intersection composite, literal and param-bound constant.
    assert_eq!(
        run(MaskConst::Mask(AllenMask::INTERSECTS), 3, 10, &[]),
        [1, 2, 4]
    );
    let bound = [Const::Interval {
        start: w(3),
        end: w(10),
    }];
    assert_eq!(
        run(MaskConst::Mask(AllenMask::INTERSECTS), 0, 0, &bound),
        [1, 2, 4]
    );
    // The field's interval covers the constant.
    assert_eq!(run(MaskConst::Mask(AllenMask::COVERS), 3, 4, &[]), [1, 4]);
    // COVERED_BY: the field within [0,10) — the old reversed containment,
    // now a mask like everything else.
    assert_eq!(
        run(MaskConst::Mask(AllenMask::COVERED_BY), 0, 10, &[]),
        [1, 4, 5]
    );
    // A param mask resolves through the slice as its 13-bit word; the
    // mirrored form (`ConversedParam`) converses after resolution —
    // COVERED_BY via a COVERS param proves the involution end to end.
    let mask_param = [Const::Word(u64::from(AllenMask::COVERS.bits()))];
    assert_eq!(
        run(MaskConst::ConversedParam(ParamId(0)), 0, 10, &mask_param),
        [1, 4, 5]
    );
    assert_eq!(run(MaskConst::Param(ParamId(0)), 3, 4, &mask_param), [1, 4]);
}

#[test]
fn interval_constants_compare_pairwise_under_eq() {
    let dir = TempDir::new("view-interval-const");
    let image = interval_image(&dir);
    let predicates = vec![FilterPredicate::Compare {
        field: P_DURING,
        op: CmpOp::Eq,
        value: Const::Interval {
            start: w(2),
            end: w(9),
        },
    }];
    assert_eq!(
        sorted_ids(&apply(&image, &predicates, &[], Vec::new()).expect("no measure filters")),
        [1]
    );
}

#[test]
fn param_set_eq_matches_any_element_over_a_scalar_column() {
    let dir = TempDir::new("view-param-set");
    let image = interval_image(&dir);
    let predicates = vec![FilterPredicate::Compare {
        field: P_ID,
        op: CmpOp::Eq,
        value: Const::ParamSet(ParamId(0)),
    }];
    let params = [Const::WordSet(vec![1u64, 3])];
    assert_eq!(
        sorted_ids(&apply(&image, &predicates, &params, Vec::new()).expect("no measure filters")),
        [1, 3]
    );
}
