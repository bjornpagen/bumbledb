use super::*;
use crate::Interval;
use crate::schema::{FixedIntervalElement, IntervalElement};
use crate::work::WorkContext;

fn work() -> WorkContext {
    WorkContext::new()
}

fn fields(types: &[ValueType]) -> Vec<FieldDescriptor> {
    types
        .iter()
        .map(|value_type| FieldDescriptor {
            name: "v".into(),
            value_type: *value_type,
        })
        .collect()
}

fn exact_projection(
    fields: &[FieldDescriptor],
    selected: &[u16],
) -> crate::schema::CompiledProjection {
    use crate::schema::{CompiledProjection, FieldId, KeyEncoding, ProjectionId, RelationId};

    let scalar_fields: Box<[_]> = selected
        .iter()
        .map(|&field| fields[usize::from(field)].clone())
        .collect();
    let width: usize = scalar_fields
        .iter()
        .map(|field| exact_scalar_width(&field.value_type).unwrap())
        .sum();
    CompiledProjection {
        id: ProjectionId(0),
        relation: RelationId(0),
        projection: selected.iter().copied().map(FieldId).collect(),
        scalar_positions: (0..selected.len()).collect(),
        scalar_fields,
        encoding: KeyEncoding::ExactBounded {
            scalar_width: u8::try_from(width).unwrap(),
        },
        interval_position: None,
        interval_type: None,
    }
}

#[test]
fn scalar_routes_borrow_payloads_and_preserve_nonleading_and_composite_order() {
    let fields = fields(&[
        ValueType::String,
        ValueType::Bool,
        ValueType::U64,
        ValueType::I64,
        ValueType::F64,
        ValueType::Uuid,
        ValueType::FixedBytes { len: 3 },
        ValueType::FixedBytes { len: 16 },
        ValueType::Interval {
            element: IntervalElement::U64,
        },
    ]);
    let mut text = "a".repeat(COPY_QUANTUM - 1);
    text.push('🦀');
    text.push_str(&"é".repeat(COPY_QUANTUM));
    let values = [
        Value::String(text.into()),
        Value::Bool(true),
        Value::U64(u64::MAX),
        Value::I64(-42),
        Value::F64(F64::from(-1.25)),
        Value::Uuid(Uuid::from_bytes([0xaa; 16])),
        Value::FixedBytes(Box::new([1, 2, 3])),
        Value::FixedBytes(Box::new([0xbb; 16])),
        Value::IntervalU64(Interval::new(1, 5).unwrap()),
    ];
    let donor = work();
    let row = CanonicalRow::encode(&fields, &values, &donor).unwrap();
    let cases: &[&[u16]] = &[
        &[1],
        &[2],
        &[3],
        &[4],
        &[5],
        &[6],
        &[7],
        &[6, 3, 1],
        &[4, 2],
        &[],
    ];
    for selected in cases {
        let projection = exact_projection(&fields, selected);
        let context = work();
        let mut route = [0; crate::schema::MAX_EXACT_SCALAR_BYTES];
        let mut expected = [0; crate::schema::MAX_EXACT_SCALAR_BYTES];
        assert_eq!(
            exact_scalar_projection(&fields, &row, &projection, &context, &mut route).unwrap(),
            projection.encode_scalar_row(&values, &mut expected),
            "projection {selected:?}"
        );
        let validation = work();
        validate(&fields, &row, &validation).unwrap();
    }
}

#[test]
fn scalar_route_validates_every_field_and_all_row_framing() {
    let fields = fields(&[ValueType::U64, ValueType::String]);
    let projection = exact_projection(&fields, &[0]);
    let donor = work();
    let row = CanonicalRow::encode(
        &fields,
        &[Value::U64(7), Value::String("tail".into())],
        &donor,
    )
    .unwrap();
    let context = work();
    let mut route = [0; crate::schema::MAX_EXACT_SCALAR_BYTES];
    for end in 0..row.len() {
        assert_eq!(
            exact_scalar_projection(&fields, &row[..end], &projection, &context, &mut route),
            Err(RowError::Truncated),
            "prefix {end} cannot return a route before the tail validates"
        );
    }
    let mut malformed = row.as_bytes().to_vec();
    malformed.push(0);
    assert_eq!(
        exact_scalar_projection(&fields, &malformed, &projection, &context, &mut route),
        Err(RowError::TrailingBytes)
    );
    malformed.truncate(row.len());
    malformed[1] = 1;
    assert_eq!(
        exact_scalar_projection(&fields, &malformed, &projection, &context, &mut route),
        Err(RowError::Arity)
    );
    malformed[1] = 2;
    *malformed.last_mut().unwrap() = 0xff;
    assert_eq!(
        exact_scalar_projection(&fields, &malformed, &projection, &context, &mut route),
        Err(RowError::InvalidUtf8 { field: 1 })
    );
    malformed[11] = 255;
    assert_eq!(
        exact_scalar_projection(&fields, &malformed, &projection, &context, &mut route),
        Err(RowError::InvalidTag { field: 1 })
    );
}

#[test]
fn scalar_route_float_order_and_noncanonical_unselected_values() {
    let fields = fields(&[ValueType::Bool, ValueType::F64]);
    let projection = exact_projection(&fields, &[1]);
    let key_only = exact_projection(&fields, &[0]);
    let donor = work();
    let mut previous = None;
    for value in [
        F64::NEG_INFINITY,
        F64::from(-1.0),
        F64::from(0.0),
        F64::from(1.0),
        F64::INFINITY,
        F64::NAN,
    ] {
        let values = [Value::Bool(true), Value::F64(value)];
        let row = CanonicalRow::encode(&fields, &values, &donor).unwrap();
        let context = work();
        let mut route = [0; crate::schema::MAX_EXACT_SCALAR_BYTES];
        let actual = exact_scalar_projection(&fields, &row, &projection, &context, &mut route)
            .unwrap()
            .unwrap();
        assert_eq!(actual, crate::encoding::encode_f64(value));
        let actual: [u8; 8] = actual.try_into().unwrap();
        if let Some(previous) = previous {
            assert!(previous < actual);
        }
        previous = Some(actual);
    }
    let positive = CanonicalRow::encode(
        &fields,
        &[Value::Bool(true), Value::F64(F64::from(0.0))],
        &donor,
    )
    .unwrap();
    let negative = CanonicalRow::encode(
        &fields,
        &[Value::Bool(true), Value::F64(F64::from(-0.0))],
        &donor,
    )
    .unwrap();
    assert_eq!(positive.as_bytes(), negative.as_bytes());
    for bits in [
        0x8000_0000_0000_0000u64,
        0x7ff0_0000_0000_0001,
        0xfff8_0000_0000_0000,
    ] {
        let mut malformed = vec![0, 2, 0, 1, 3];
        malformed.extend_from_slice(&bits.to_be_bytes());
        for selected in [&projection, &key_only] {
            let mut route = [0; crate::schema::MAX_EXACT_SCALAR_BYTES];
            assert_eq!(
                exact_scalar_projection(&fields, &malformed, selected, &work(), &mut route),
                Err(RowError::NonCanonicalFloat { field: 1 })
            );
        }
    }
}

#[test]
fn scalar_route_rejects_unselected_interval_and_fixed_bytes_malformed_values() {
    let cases: &[(ValueType, &[u8], RowError)] = &[
        (ValueType::Bool, &[0, 2], RowError::InvalidBool { field: 1 }),
        (
            ValueType::Bool,
            &[1, 0, 0, 0, 0, 0, 0, 0, 1],
            RowError::Type { field: 1 },
        ),
        (
            ValueType::FixedBytes { len: 3 },
            &[5, 0, 0, 0, 0, 0, 0, 0, 2, 1, 2],
            RowError::Type { field: 1 },
        ),
        (
            ValueType::FixedInterval {
                element: FixedIntervalElement::U64,
                width: 3,
            },
            &[6, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 5],
            RowError::Type { field: 1 },
        ),
        (
            ValueType::Interval {
                element: IntervalElement::U64,
            },
            &[6, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1],
            RowError::InvalidInterval { field: 1 },
        ),
    ];
    for (value_type, suffix, error) in cases {
        let fields = fields(&[ValueType::Bool, *value_type]);
        let projection = exact_projection(&fields, &[0]);
        let mut malformed = vec![0, 2, 0, 1];
        malformed.extend_from_slice(suffix);
        let mut route = [0; crate::schema::MAX_EXACT_SCALAR_BYTES];
        assert_eq!(
            exact_scalar_projection(&fields, &malformed, &projection, &work(), &mut route),
            Err(*error)
        );
    }
}

#[test]
fn scalar_route_checks_cancellation_including_empty_projections() {
    let fields = fields(&[ValueType::U64, ValueType::String]);
    let projection = exact_projection(&fields, &[0]);
    let row = CanonicalRow::encode(
        &fields,
        &[
            Value::U64(7),
            Value::String("x".repeat(COPY_QUANTUM + 1).into()),
        ],
        &work(),
    )
    .unwrap();
    let context = work();
    let mut route = [0; crate::schema::MAX_EXACT_SCALAR_BYTES];
    assert_eq!(
        exact_scalar_projection(&fields, &row, &projection, &context, &mut route).unwrap(),
        Some(7u64.to_be_bytes().as_slice())
    );
    context.cancel();
    assert_eq!(
        exact_scalar_projection(&fields, &row, &projection, &context, &mut route),
        Err(RowError::Work(WorkError::Cancelled))
    );
    let empty_projection = exact_projection(&[], &[]);
    assert_eq!(
        exact_scalar_projection(&[], &[0, 0], &empty_projection, &context, &mut route),
        Err(RowError::Work(WorkError::Cancelled))
    );
}

#[test]
fn decode_scratch_reuses_capacity_and_drops_payloads_after_each_visit() {
    let fields = fields(&[
        ValueType::U64,
        ValueType::String,
        ValueType::FixedBytes { len: 64 },
    ]);
    let values = [
        Value::U64(7),
        Value::String("payload".into()),
        Value::FixedBytes(Box::new([3; 64])),
    ];
    let context = work();
    let encoded = CanonicalRow::encode(&fields, &values, &context).unwrap();
    let mut scratch = DecodeScratch::new(&context);
    let mut allocation = None;
    for _ in 0..3 {
        scratch
            .with_decoded(&fields, &encoded, |actual| {
                assert_eq!(actual, values);
                Ok::<_, RowError>(())
            })
            .unwrap();
        assert!(
            scratch.values.is_empty(),
            "visitor payloads drop before returning"
        );
        let current = (scratch.values.as_ptr(), scratch.values.capacity());
        if let Some(previous) = allocation {
            assert_eq!(current, previous, "equal-width rows reuse the same vector");
        }
        allocation = Some(current);
    }
    let narrow = CanonicalRow::encode(&fields[..1], &values[..1], &context).unwrap();
    scratch
        .with_decoded(&fields[..1], &narrow, |actual| {
            assert_eq!(actual, &values[..1]);
            Ok::<_, RowError>(())
        })
        .unwrap();
    assert!(scratch.values.is_empty());
    assert_eq!(
        allocation,
        Some((scratch.values.as_ptr(), scratch.values.capacity()))
    );
}

#[test]
fn decode_scratch_cancellation_prevents_growth_or_visit() {
    let context = work();
    let mut scratch = DecodeScratch::new(&context);
    scratch.prepare(1).unwrap();
    let allocation = (scratch.values.as_ptr(), scratch.values.capacity());
    let fields = fields(&[ValueType::U64; 8]);
    let wide = CanonicalRow::encode(&fields, &vec![Value::U64(0); 8], &work()).unwrap();
    context.cancel();
    assert_eq!(
        scratch.with_decoded(&fields, &wide, |_| -> Result<(), RowError> {
            panic!("cancelled decode cannot reach the visitor")
        }),
        Err(RowError::Work(WorkError::Cancelled))
    );
    assert!(scratch.values.is_empty());
    assert_eq!(
        (scratch.values.as_ptr(), scratch.values.capacity()),
        allocation
    );
}

#[test]
fn decode_scratch_capacity_overflow_preserves_existing_allocation() {
    let context = work();
    let mut scratch = DecodeScratch::new(&context);
    scratch.prepare(1).unwrap();
    let allocation = (scratch.values.as_ptr(), scratch.values.capacity());
    // Deterministic capacity overflow, not an attempt to exhaust host memory.
    let impossible = usize::try_from(isize::MAX).unwrap() / std::mem::size_of::<Value>() + 1;
    assert_eq!(scratch.prepare(impossible), Err(RowError::Allocation));
    assert_eq!(
        (scratch.values.as_ptr(), scratch.values.capacity()),
        allocation
    );
    scratch.prepare(1).unwrap();
    assert!(scratch.values.is_empty());
}

#[test]
fn decode_scratch_clears_partial_values_on_decode_visitor_and_cancellation_errors() {
    let fields = fields(&[ValueType::String, ValueType::U64]);
    let context = work();
    let encoded = CanonicalRow::encode(
        &fields,
        &[Value::String("x".repeat(5000).into()), Value::U64(3)],
        &context,
    )
    .unwrap();
    let mut scratch = DecodeScratch::new(&context);
    assert_eq!(
        scratch.with_decoded(
            &fields,
            &encoded[..encoded.len() - 1],
            |_| -> Result<(), RowError> { panic!("malformed row cannot reach visitor") }
        ),
        Err(RowError::Truncated)
    );
    assert!(scratch.values.is_empty());
    let allocation = (scratch.values.as_ptr(), scratch.values.capacity());
    assert_eq!(
        scratch.with_decoded(&fields, &encoded, |_| Err::<(), _>(RowError::Allocation)),
        Err(RowError::Allocation)
    );
    assert!(scratch.values.is_empty());
    assert_eq!(
        scratch.with_decoded(&fields, &encoded, |_| {
            context.cancel();
            Err::<(), _>(RowError::Work(WorkError::Cancelled))
        }),
        Err(RowError::Work(WorkError::Cancelled))
    );
    assert!(scratch.values.is_empty());
    assert_eq!(
        scratch.with_decoded(&fields, &encoded, |_| -> Result<(), RowError> {
            panic!("cancelled workspace cannot visit")
        }),
        Err(RowError::Work(WorkError::Cancelled))
    );
    assert_eq!(
        (scratch.values.as_ptr(), scratch.values.capacity()),
        allocation
    );
}

#[test]
fn decode_scratch_remains_reusable_after_visitor_unwinds() {
    let fields = fields(&[ValueType::String]);
    let context = work();
    let encoded =
        CanonicalRow::encode(&fields, &[Value::String("payload".into())], &context).unwrap();
    let mut scratch = DecodeScratch::new(&context);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: Result<(), RowError> =
            scratch.with_decoded(&fields, &encoded, |_| panic!("visitor"));
    }));
    assert!(panic.is_err());
    assert_eq!(scratch.values, [Value::String("payload".into())]);
    scratch
        .with_decoded(&fields, &encoded, |actual| {
            assert_eq!(actual, &[Value::String("payload".into())]);
            Ok::<_, RowError>(())
        })
        .unwrap();
    assert!(scratch.values.is_empty());
}

#[test]
fn field_batches_preserve_wire_roundtrips() {
    for width in [
        0,
        1,
        FIELD_QUANTUM - 1,
        FIELD_QUANTUM,
        FIELD_QUANTUM + 1,
        129,
    ] {
        let fields = fields(&vec![ValueType::U64; width]);
        let values: Vec<_> = (0..width).map(|value| Value::U64(value as u64)).collect();
        let ctx = work();
        let row = CanonicalRow::encode(&fields, &values, &ctx).unwrap();
        let ctx = work();
        let parsed = CanonicalRow::parse(&fields, row.as_bytes(), &ctx).unwrap();
        assert_eq!(parsed.as_bytes(), row.as_bytes());
        let ctx = work();
        let decoded = decode(&fields, row.as_bytes(), &ctx).unwrap();
        assert_eq!(decoded.values(), values);
        drop(decoded);
    }
}

#[test]
fn field_batches_keep_variable_width_roundtrips() {
    let mut types = vec![ValueType::U64; FIELD_QUANTUM];
    types.push(ValueType::String);
    let fields = fields(&types);
    let mut values = vec![Value::U64(1); FIELD_QUANTUM];
    values.push(Value::String("x".repeat(COPY_QUANTUM + 1).into_boxed_str()));
    let ctx = work();
    let row = CanonicalRow::encode(&fields, &values, &ctx).unwrap();
    let ctx = work();
    let decoded = decode(&fields, row.as_bytes(), &ctx).unwrap();
    assert_eq!(decoded.values(), values);
}

#[test]
fn field_batches_preserve_global_error_indices_and_cancellation() {
    let fields = fields(&vec![ValueType::U64; FIELD_QUANTUM + 1]);
    let mut values = vec![Value::U64(1); fields.len()];
    let row = CanonicalRow::encode(&fields, &values, &work()).unwrap();
    let mut malformed = row.as_bytes().to_vec();
    malformed[2 + 9 * FIELD_QUANTUM] = 255;
    assert_eq!(
        validate(&fields, &malformed, &work()),
        Err(RowError::InvalidTag {
            field: FIELD_QUANTUM
        })
    );
    assert!(matches!(
        decode(&fields, &malformed, &work()),
        Err(RowError::InvalidTag {
            field: FIELD_QUANTUM
        })
    ));
    values[FIELD_QUANTUM] = Value::Bool(true);
    assert!(matches!(
        CanonicalRow::encode(&fields, &values, &work()),
        Err(RowError::Type {
            field: FIELD_QUANTUM
        })
    ));
    let context = work();
    context.cancel();
    assert!(matches!(
        CanonicalRow::encode(&fields, &values, &context),
        Err(RowError::Work(WorkError::Cancelled))
    ));
    assert!(matches!(
        CanonicalRow::encode(&[], &[], &context),
        Err(RowError::Work(WorkError::Cancelled))
    ));
    assert!(matches!(
        CanonicalRow::parse(&[], &[0, 0], &context),
        Err(RowError::Work(WorkError::Cancelled))
    ));
    assert_eq!(
        decode(&[], &[0, 0], &context),
        Err(RowError::Work(WorkError::Cancelled))
    );
    assert_eq!(
        validate(&[], &[0, 0], &context),
        Err(RowError::Work(WorkError::Cancelled))
    );
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
            element: FixedIntervalElement::I64,
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
    assert_eq!(decoded.values(), values);
    drop(decoded);
    for end in 0..expected.len() {
        assert!(
            CanonicalRow::parse(&fields, &expected[..end], &ctx).is_err(),
            "prefix {end}"
        );
    }
    drop(parsed);
    drop(row);
    let mut trailing = expected.to_vec();
    trailing.push(0);
    assert!(matches!(
        CanonicalRow::parse(&fields, &trailing, &ctx),
        Err(RowError::TrailingBytes)
    ));
}

/// Uuid (tag 8) and the dense float interval (tag 9) have exact golden
/// wire bytes: endpoints are canonical binary64 payload bits, big endian,
/// never index order keys (E-CODEC, F-WIRE, F-INTERVAL).
#[test]
fn uuid_and_dense_interval_golden_roundtrip() {
    let fields = fields(&[
        ValueType::Uuid,
        ValueType::Interval {
            element: IntervalElement::F64,
        },
    ]);
    let id = crate::Uuid::from_bytes([0xaa; 16]);
    let span = Interval::<F64>::new(F64::NEG_INFINITY, F64::from(1.0)).unwrap();
    let values = [Value::Uuid(id), Value::IntervalF64(span)];
    let mut expected: Vec<u8> = vec![0, 2, 8];
    expected.extend_from_slice(&[0xaa; 16]);
    expected.push(9);
    expected.extend_from_slice(&0xfff0_0000_0000_0000u64.to_be_bytes());
    expected.extend_from_slice(&0x3ff0_0000_0000_0000u64.to_be_bytes());
    let ctx = work();
    let row = CanonicalRow::encode(&fields, &values, &ctx).unwrap();
    assert_eq!(row.as_bytes(), &expected[..]);
    let parsed = CanonicalRow::parse(&fields, &expected, &ctx).unwrap();
    assert_eq!(parsed.as_bytes(), &expected[..]);
    let decoded = decode(&fields, &expected, &ctx).unwrap();
    assert_eq!(decoded.values(), values);
    for end in 0..expected.len() {
        assert!(
            CanonicalRow::parse(&fields, &expected[..end], &ctx).is_err(),
            "prefix {end}"
        );
    }
}

/// A forged dense-interval wire value cannot parse into a successful
/// canonical row: noncanonical endpoint bits, NaN endpoints, equal or
/// inverted bounds each refuse with their own diagnostic (E-CODEC).
#[test]
fn forged_dense_interval_bytes_refuse() {
    let ctx = work();
    let fields = fields(&[ValueType::Interval {
        element: IntervalElement::F64,
    }]);
    let framed = |start: u64, end: u64| {
        let mut bytes = vec![0, 1, 9];
        bytes.extend_from_slice(&start.to_be_bytes());
        bytes.extend_from_slice(&end.to_be_bytes());
        bytes
    };
    // Noncanonical endpoints: negative zero and a signaling NaN payload.
    for (start, end) in [
        (0x8000_0000_0000_0000u64, 0x3ff0_0000_0000_0000u64),
        (0x0000_0000_0000_0000, 0x7ff0_0000_0000_0001),
    ] {
        assert!(matches!(
            CanonicalRow::parse(&fields, &framed(start, end), &ctx),
            Err(RowError::NonCanonicalFloat { field: 0 })
        ));
    }
    // Canonical NaN is refused as an ENDPOINT, empty and inverted bounds
    // as an INTERVAL; the parser never normalizes any of them.
    for (start, end) in [
        (0x7ff8_0000_0000_0000u64, 0x7ff8_0000_0000_0000u64), // NaN..NaN
        (0x0000_0000_0000_0000, 0x7ff8_0000_0000_0000),       // 0..NaN
        (0x3ff0_0000_0000_0000, 0x3ff0_0000_0000_0000),       // [1,1)
        (0x4000_0000_0000_0000, 0x3ff0_0000_0000_0000),       // [2,1)
        (0x7ff0_0000_0000_0000, 0x7ff0_0000_0000_0000),       // [inf,inf)
    ] {
        assert!(
            matches!(
                CanonicalRow::parse(&fields, &framed(start, end), &ctx),
                Err(RowError::InvalidInterval { field: 0 })
            ),
            "{start:#x}..{end:#x}"
        );
    }
    // The typed encoder cannot be tricked either: value/type cross-checks
    // refuse an integer interval at a dense position and vice versa.
    let integer = Value::IntervalU64(Interval::new(0, 1).unwrap());
    assert!(matches!(
        CanonicalRow::encode(&fields, &[integer], &ctx),
        Err(RowError::Type { field: 0 })
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
fn shape_errors_refuse_and_zero_field_rows_roundtrip() {
    let fields = fields(&[ValueType::U64]);
    let context = work();
    assert!(matches!(
        CanonicalRow::encode(&fields, &[], &context),
        Err(RowError::Arity)
    ));
    assert!(matches!(
        CanonicalRow::encode(&fields, &[Value::Bool(false)], &context),
        Err(RowError::Type { field: 0 })
    ));
    let row = CanonicalRow::encode(&[], &[], &context).unwrap();
    assert_eq!(row.as_bytes(), &[0, 0]);
    assert_eq!(
        CanonicalRow::parse(&[], &[0, 0], &context)
            .unwrap()
            .as_bytes(),
        &[0, 0]
    );
}

#[test]
fn image_walker_and_strict_decode_agree_on_interval_laws() {
    use crate::image::canon::{TextWords, row_words};
    use crate::image::intern::TextInterner;

    let fields = fields(&[ValueType::FixedInterval {
        element: FixedIntervalElement::U64,
        width: 2,
    }]);
    let ctx = work();
    let healthy = CanonicalRow::encode(
        &fields,
        &[Value::IntervalU64(Interval::new(3, 5).expect("width 2"))],
        &ctx,
    )
    .expect("canonical");
    let interner = TextInterner::default();
    let mut text = TextWords::Lookup(&interner);
    let mut words = Vec::new();
    row_words(
        &fields,
        healthy.as_bytes(),
        &mut text,
        &mut words,
        &mut Vec::new(),
    )
    .expect("image walk");
    assert_eq!(words, [3, 5]);

    // Wrong fixed width: strict decode and image walk both refuse.
    assert!(matches!(
        CanonicalRow::encode(
            &fields,
            &[Value::IntervalU64(Interval::new(3, 4).expect("width 1"))],
            &ctx,
        ),
        Err(RowError::Type { field: 0 })
    ));
    // Corrupt raw bytes, not a checked CanonicalRow constructor.
    let mut bad = healthy.as_bytes().to_vec();
    bad[11..19].copy_from_slice(&4u64.to_be_bytes());
    assert!(matches!(
        decode(&fields, &bad, &ctx),
        Err(RowError::Type { field: 0 })
    ));
    let mut words = Vec::new();
    assert!(row_words(&fields, &bad, &mut text, &mut words, &mut Vec::new()).is_err());
}
