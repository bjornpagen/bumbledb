use super::*;
use crate::encoding::{decode_field, encode_fact, encode_i64, encode_u64, ValueRef};
use crate::error::Result as DbResult;
use crate::image::build;
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
    ValueType,
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
                    generation: Generation::Serial,
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
            constraints: vec![],
        }],
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
    let image = build(&txn, &schema, R).expect("build");
    let predicates = vec![FilterPredicate::FieldsCompare {
        left: FieldId(2),
        right: FieldId(3),
        op: CmpOp::Eq,
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
    let image = build(&txn, &schema, R).expect("build");
    let predicates = vec![FilterPredicate::Compare {
        field: FieldId(0),
        op: CmpOp::Eq,
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
    let image = build(&txn, &schema, R).expect("build");
    let view = apply(&image, &[], &[], Vec::new());
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
    let reapplied = apply(&image, &predicates, &[], Vec::new());
    assert_eq!(
        view.positions().collect::<Vec<_>>(),
        reapplied.positions().collect::<Vec<_>>()
    );
    assert_eq!(view.len(), 10);
    Ok(())
}
