use super::*;
use crate::schema::{Generation, IntervalElement};
use crate::work::{ExecutionPolicy, Resource};
use std::time::Duration;

fn work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 1_000_000,
        working_bytes: 1_000_000,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 1000,
        work_units: 1_000_000,
        timeout: Duration::from_secs(60),
    }
    .start()
    .unwrap()
}

fn fields(types: &[ValueType]) -> Vec<FieldDescriptor> {
    types
        .iter()
        .map(|value_type| FieldDescriptor {
            name: "v".into(),
            value_type: *value_type,
            generation: Generation::None,
        })
        .collect()
}

#[test]
fn independent_all_scalar_golden_and_every_truncation() {
    let fields = fields(&[
        ValueType::Bool,
        ValueType::U64,
        ValueType::I64,
        ValueType::F64,
        ValueType::String,
        ValueType::FixedBytes { len: 3 },
        ValueType::Interval {
            element: IntervalElement::U64,
        },
        ValueType::FixedInterval {
            element: IntervalElement::I64,
            width: 2,
        },
    ]);
    let values = [
        Value::Bool(true),
        Value::U64(42),
        Value::I64(-2),
        Value::F64(F64::NAN),
        Value::String("é".into()),
        Value::FixedBytes([1, 2, 3].into()),
        Value::IntervalU64(Interval::new(3, 7).unwrap()),
        Value::IntervalI64(Interval::new(-3, -1).unwrap()),
    ];
    let expected: &[u8] = &[
        0, 8, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 42, 2, 255, 255, 255, 255, 255, 255, 255, 254, 3, 127,
        248, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 2, 195, 169, 5, 0, 0, 0, 0, 0, 0, 0, 3, 1,
        2, 3, 6, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 7, 7, 255, 255, 255, 255, 255, 255,
        255, 253, 255, 255, 255, 255, 255, 255, 255, 255,
    ];
    let ctx = work();
    let row = CanonicalRow::encode(&fields, &values, &ctx).unwrap();
    assert_eq!(row.as_bytes(), expected);
    let parsed = CanonicalRow::parse(&fields, expected, &ctx).unwrap();
    assert_eq!(parsed.as_bytes(), expected);
    let decoded = decode(&fields, expected, &ctx).unwrap();
    assert_eq!(decoded.values, values);
    drop(decoded);
    assert_eq!(
        ctx.used(Resource::WorkingBytes),
        (2 * expected.len()) as u64
    );
    for end in 0..expected.len() {
        assert!(
            CanonicalRow::parse(&fields, &expected[..end], &ctx).is_err(),
            "prefix {end}"
        );
    }
    drop(parsed);
    drop(row);
    assert_eq!(ctx.used(Resource::WorkingBytes), 0);
    let mut trailing = expected.to_vec();
    trailing.push(0);
    assert!(matches!(
        CanonicalRow::parse(&fields, &trailing, &ctx),
        Err(RowError::TrailingBytes)
    ));
}

#[test]
fn malformed_bool_float_interval_width_and_utf8_refuse() {
    let ctx = work();
    assert!(matches!(
        CanonicalRow::parse(&fields(&[ValueType::Bool]), &[0, 1, 0, 2], &ctx),
        Err(RowError::InvalidBool { field: 0 })
    ));
    for bits in [
        0x8000_0000_0000_0000u64,
        0x7ff0_0000_0000_0001,
        0xfff8_0000_0000_0000,
    ] {
        let mut bytes = vec![0, 1, 3];
        bytes.extend_from_slice(&bits.to_be_bytes());
        assert!(matches!(
            CanonicalRow::parse(&fields(&[ValueType::F64]), &bytes, &ctx),
            Err(RowError::NonCanonicalFloat { field: 0 })
        ));
    }
    let mut bad_interval = vec![0, 1, 6];
    bad_interval.extend_from_slice(&[0; 16]);
    assert!(matches!(
        CanonicalRow::parse(
            &fields(&[ValueType::Interval {
                element: IntervalElement::U64
            }]),
            &bad_interval,
            &ctx
        ),
        Err(RowError::InvalidInterval { field: 0 })
    ));
    assert!(matches!(
        CanonicalRow::parse(
            &fields(&[ValueType::String]),
            &[0, 1, 4, 0, 0, 0, 0, 0, 0, 0, 1, 255],
            &ctx
        ),
        Err(RowError::InvalidUtf8 { field: 0 })
    ));
    assert!(matches!(
        CanonicalRow::parse(
            &fields(&[ValueType::FixedBytes { len: 2 }]),
            &[0, 1, 5, 0, 0, 0, 0, 0, 0, 0, 1, 0],
            &ctx
        ),
        Err(RowError::Type { field: 0 })
    ));
    assert_eq!(ctx.used(Resource::WorkingBytes), 0);
}

#[test]
fn utf8_crossing_poll_boundaries_is_checked_without_full_copy() {
    let mut text = "a".repeat(COPY_QUANTUM - 1);
    text.push_str("🦀é");
    text.push_str(&"z".repeat(COPY_QUANTUM));
    let ctx = work();
    let fields = fields(&[ValueType::String]);
    let row = CanonicalRow::encode(&fields, &[Value::String(text.into())], &ctx).unwrap();
    let parsed = CanonicalRow::parse(&fields, row.as_bytes(), &ctx).unwrap();
    assert_eq!(parsed.as_bytes(), row.as_bytes());
    let mut invalid = row.as_bytes().to_vec();
    invalid[11 + COPY_QUANTUM] = 0xff;
    assert!(matches!(
        CanonicalRow::parse(&fields, &invalid, &ctx),
        Err(RowError::InvalidUtf8 { field: 0 })
    ));
}

#[test]
fn shape_and_budget_errors_leave_no_owned_bytes() {
    let fields = fields(&[ValueType::U64]);
    let ctx = work();
    assert!(matches!(
        CanonicalRow::encode(&fields, &[], &ctx),
        Err(RowError::Arity)
    ));
    assert!(matches!(
        CanonicalRow::encode(&fields, &[Value::Bool(false)], &ctx),
        Err(RowError::Type { field: 0 })
    ));
    assert_eq!(ctx.used(Resource::WorkingBytes), 0);
    let tiny = ExecutionPolicy {
        input_bytes: 100,
        working_bytes: 10,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 1,
        work_units: 100,
        timeout: Duration::from_secs(60),
    }
    .start()
    .unwrap();
    assert!(matches!(
        CanonicalRow::encode(&fields, &[Value::U64(0)], &tiny),
        Err(RowError::Work(WorkError::Exhausted {
            resource: Resource::WorkingBytes,
            ..
        }))
    ));
    assert_eq!(tiny.used(Resource::WorkingBytes), 0);
    let row = CanonicalRow::encode(&[], &[], &ctx).unwrap();
    assert_eq!(row.as_bytes(), &[0, 0]);
    assert_eq!(
        CanonicalRow::parse(&[], &[0, 0], &ctx).unwrap().as_bytes(),
        &[0, 0]
    );
}
