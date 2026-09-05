//! Corrupt stored bytes refuse at the one canonical walker — typed
//! corruption, never a panic and never a normalized value. The storage
//! side's raw-byte corruption drills live with the store's own tests
//! (P02); these exercise the walker the build/probe/fallback paths share.
use super::schema;
use crate::error::Error;
use crate::image::canon::{TextWords, row_words};
use crate::image::intern::TextInterner;
use crate::ir::Value;

fn canonical_bytes(values: &[Value]) -> Vec<u8> {
    let schema = schema();
    let work = crate::api::prepared::source::unbounded_work().expect("ledger");
    crate::canonical::CanonicalRow::encode(schema.relation(super::R).fields(), values, &work)
        .expect("fixture rows are canonical")
        .as_bytes()
        .to_vec()
}

fn walk(bytes: &[u8]) -> Result<Vec<u64>, Error> {
    let schema = schema();
    let interner = TextInterner::default();
    let mut text = TextWords::Lookup(&interner);
    let mut out = Vec::new();
    row_words(
        schema.relation(super::R).fields(),
        bytes,
        &mut text,
        &mut out,
    )?
    .expect_ready("lookup never spills");
    Ok(out)
}

#[test]
fn truncated_rows_refuse_as_corruption() {
    let healthy = canonical_bytes(&[
        Value::U64(1),
        Value::Bool(true),
        Value::Bool(false),
        Value::I64(-3),
    ]);
    assert!(walk(&healthy).is_ok(), "the healthy row walks");
    for cut in [1usize, 3, healthy.len() - 1] {
        let err = walk(&healthy[..cut]).expect_err("truncation refuses");
        assert!(matches!(err, Error::Corruption(_)), "{err:?}");
    }
    let mut trailing = healthy.clone();
    trailing.push(0);
    let err = walk(&trailing).expect_err("trailing bytes refuse");
    assert!(matches!(err, Error::Corruption(_)), "{err:?}");
}

#[test]
fn wrong_tags_bad_bools_and_arity_refuse() {
    let healthy = canonical_bytes(&[
        Value::U64(1),
        Value::Bool(true),
        Value::Bool(false),
        Value::I64(-3),
    ]);
    // Flip the first field's tag (offset 2 after the u16 arity).
    let mut wrong_tag = healthy.clone();
    wrong_tag[2] = 0xEE;
    assert!(matches!(walk(&wrong_tag), Err(Error::Corruption(_))));
    // A bool payload of 2 refuses (never truthy normalization).
    let mut bad_bool = healthy.clone();
    // layout: arity(2) + [tag u64(1) + 8] + [tag bool(1) + 1] ...
    bad_bool[2 + 9 + 1] = 2;
    assert!(matches!(walk(&bad_bool), Err(Error::Corruption(_))));
    // A wrong arity header refuses.
    let mut bad_arity = healthy;
    bad_arity[1] = 9;
    assert!(matches!(walk(&bad_arity), Err(Error::Corruption(_))));
}

#[test]
fn noncanonical_floats_and_inverted_intervals_refuse() {
    let float_schema =
        crate::schema::ValidateDescriptor::validate(bumbledb_theory::schema::SchemaDescriptor {
            relations: vec![bumbledb_theory::schema::RelationDescriptor {
                extension: None,
                name: "F".into(),
                fields: vec![
                    bumbledb_theory::schema::FieldDescriptor {
                        name: "x".into(),
                        value_type: bumbledb_theory::schema::ValueType::F64,
                    },
                    bumbledb_theory::schema::FieldDescriptor {
                        name: "span".into(),
                        value_type: bumbledb_theory::schema::ValueType::Interval {
                            element: bumbledb_theory::schema::IntervalElement::U64,
                        },
                    },
                ],
            }],
            statements: vec![],
        })
        .expect("valid fixture");
    let work = crate::api::prepared::source::unbounded_work().expect("ledger");
    let healthy = crate::canonical::CanonicalRow::encode(
        float_schema.relation(super::R).fields(),
        &[
            Value::F64(bumbledb_theory::F64::from(1.5)),
            Value::IntervalU64(bumbledb_theory::Interval::<u64>::new(3, 7).expect("nonempty")),
        ],
        &work,
    )
    .expect("canonical")
    .as_bytes()
    .to_vec();

    let walk_f = |bytes: &[u8]| -> Result<Vec<u64>, Error> {
        let interner = TextInterner::default();
        let mut text = TextWords::Lookup(&interner);
        let mut out = Vec::new();
        row_words(
            float_schema.relation(super::R).fields(),
            bytes,
            &mut text,
            &mut out,
        )?
        .expect_ready("lookup never spills");
        Ok(out)
    };
    assert!(walk_f(&healthy).is_ok());

    // A negative-zero payload is a noncanonical stored float: refuse.
    let mut neg_zero = healthy.clone();
    neg_zero[3..11].copy_from_slice(&(-0.0f64).to_bits().to_be_bytes());
    assert!(matches!(walk_f(&neg_zero), Err(Error::Corruption(_))));

    // Inverted interval endpoints refuse (never a silent empty range).
    let mut inverted = healthy;
    // layout: arity(2) + [tag f64(1) + 8] + [tag interval(1) + 8 + 8]
    inverted[12..20].copy_from_slice(&9u64.to_be_bytes());
    inverted[20..28].copy_from_slice(&3u64.to_be_bytes());
    assert!(matches!(walk_f(&inverted), Err(Error::Corruption(_))));
}

#[test]
fn f64_interval_with_nan_endpoint_refuses_like_strict_decode() {
    let float_schema =
        crate::schema::ValidateDescriptor::validate(bumbledb_theory::schema::SchemaDescriptor {
            relations: vec![bumbledb_theory::schema::RelationDescriptor {
                extension: None,
                name: "F".into(),
                fields: vec![bumbledb_theory::schema::FieldDescriptor {
                    name: "span".into(),
                    value_type: bumbledb_theory::schema::ValueType::Interval {
                        element: bumbledb_theory::schema::IntervalElement::F64,
                    },
                }],
            }],
            statements: vec![],
        })
        .expect("valid fixture");
    let work = crate::api::prepared::source::unbounded_work().expect("ledger");
    let nan = bumbledb_theory::F64::NAN;
    let finite = bumbledb_theory::F64::from(1.0);
    let mut bytes = vec![0, 1, 9];
    bytes.extend_from_slice(&finite.to_be_bytes());
    bytes.extend_from_slice(&nan.to_be_bytes());
    assert!(matches!(
        crate::canonical::decode(float_schema.relation(super::R).fields(), &bytes, &work),
        Err(crate::canonical::RowError::InvalidInterval { field: 0 })
    ));
    let interner = TextInterner::default();
    let mut text = TextWords::Lookup(&interner);
    let mut out = Vec::new();
    assert!(row_words(
        float_schema.relation(super::R).fields(),
        &bytes,
        &mut text,
        &mut out
    )
    .is_err());
}
