use super::HeapStage;
use crate::encoding::{ValueRef, encode_fact};
use crate::schema::ValidateDescriptor as _;
use crate::schema::{Schema, SchemaDescriptor};
use crate::storage::delta::{DeltaEffect, Disposition};
use crate::storage::keys;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, ValueType,
};
use std::num::NonZeroU64;

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
                    name: "amount".into(),
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
const ID: FieldId = FieldId(0);

fn fact(schema: &Schema, id: u64, amount: i64) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode_fact(
        &[ValueRef::U64(id), ValueRef::I64(amount)],
        schema.relation(R).layout(),
        &mut bytes,
    );
    bytes
}

#[test]
fn insert_then_contains_then_cancel() {
    let schema = schema();
    let mut stage = HeapStage::new(&schema);
    let f = fact(&schema, 1, 100);
    assert_eq!(
        stage.apply(&schema, R, &f, Disposition::Insert),
        DeltaEffect::Recorded
    );
    assert!(stage.contains(R, &f));
    assert_eq!(stage.fact_bytes(stage.fact_refs()[0]), f.as_slice());
    assert_eq!(
        stage.apply(&schema, R, &f, Disposition::Insert),
        DeltaEffect::NoOp
    );
    assert_eq!(
        stage.apply(&schema, R, &f, Disposition::Delete),
        DeltaEffect::Cancelled
    );
    assert!(!stage.contains(R, &f));
    assert_eq!(
        stage.apply(&schema, R, &f, Disposition::Delete),
        DeltaEffect::NoOp
    );
}

#[test]
fn identity_index_grows_and_cancels_a_middle_fact() {
    let schema = schema();
    let mut stage = HeapStage::new(&schema);
    let rows: Vec<_> = (0..200)
        .map(|i| fact(&schema, i, i.cast_signed()))
        .collect();
    for row in &rows {
        assert_eq!(
            stage.apply(&schema, R, row, Disposition::Insert),
            DeltaEffect::Recorded
        );
    }
    assert_eq!(stage.fact_refs().len(), 200);
    let mid = &rows[17];
    assert!(stage.contains(R, mid));
    assert_eq!(
        stage.apply(&schema, R, mid, Disposition::Delete),
        DeltaEffect::Cancelled
    );
    assert!(!stage.contains(R, mid));
    for (i, row) in rows.iter().enumerate() {
        if i == 17 {
            continue;
        }
        assert!(stage.contains(R, row), "surviving fact {i}");
    }
}

#[test]
fn intern_round_trips_and_does_not_duplicate() {
    let schema = schema();
    let mut stage = HeapStage::new(&schema);
    let a = stage.intern_str("ada");
    let b = stage.intern_str("ada");
    assert_eq!(a, b);
    assert_eq!(stage.intern_count(), 1);
    assert_eq!(stage.resolve_str("ada"), Some(a));
    assert!(stage.resolve_str("ghost").is_none());
    assert_eq!(stage.pending_raw(a), Some(b"ada".as_slice()));
}

#[test]
fn reserve_advances_the_dense_floor() {
    let schema = schema();
    let mut stage = HeapStage::new(&schema);
    let start = stage
        .reserve(&schema, R, ID, NonZeroU64::new(3).unwrap())
        .expect("reserve");
    assert_eq!(start, 0);
    let again = stage
        .reserve(&schema, R, ID, NonZeroU64::new(1).unwrap())
        .expect("reserve");
    assert_eq!(again, 3);
    let f = fact(&schema, 10, 1);
    stage.apply(&schema, R, &f, Disposition::Insert);
    let after = stage
        .reserve(&schema, R, ID, NonZeroU64::new(1).unwrap())
        .expect("reserve after insert");
    assert_eq!(after, 11);
}

#[test]
fn overlay_last_wins_survives_swap_remove_of_an_earlier_fact() {
    let schema = schema();
    let mut stage = HeapStage::new(&schema);
    let first = fact(&schema, 1, 1);
    let second = fact(&schema, 1, 2);
    let third = fact(&schema, 1, 3);
    for row in [&first, &second, &third] {
        assert_eq!(
            stage.apply(&schema, R, row, Disposition::Insert),
            DeltaEffect::Recorded
        );
    }
    assert_eq!(
        stage.apply(&schema, R, &first, Disposition::Delete),
        DeltaEffect::Cancelled
    );
    let key = schema.relation(R).keys()[0];
    let statement = schema.key(key);
    let mut scratch = keys::DeterminantImage::scratch();
    keys::determinant_image(
        schema.relation(R).layout().encoded(&third),
        &statement.projection,
        &mut scratch,
    );
    assert_eq!(
        stage.overlay_fact(&schema, R, key, scratch.as_bytes()),
        Some(third.as_slice()),
        "deleting the first same-key fact must not promote the middle one"
    );
}
