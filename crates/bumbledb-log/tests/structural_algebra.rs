use bumbledb::schema::{
    FieldDescriptor, IntervalElement, RelationDescriptor, RelationId, SchemaDescriptor, ValueType,
};
use bumbledb::{Interval, Rounding, ScalarEvaluator, Value, WorkContext};
use bumbledb_log::migration::{
    compile::compile,
    plan::{
        FieldMap, Operation, Plan, PlanExpr as P, StepLabel, canonical_plan_bytes, decode_plan,
        parse_plan, render_plan,
    },
    state::{MigrationState, StateError},
};
use bumbledb_log::schema_file::{self, schema_id};

fn source() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Slice".into(),
            extension: None,
            fields: vec![
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::I64,
                    },
                },
                FieldDescriptor {
                    name: "amount".into(),
                    value_type: ValueType::U64,
                },
            ],
        }],
        statements: vec![],
    }
}
fn plan(from: &SchemaDescriptor, to: &SchemaDescriptor, mut ops: Vec<Operation>) -> Plan {
    let to_schema = schema_id(to).unwrap();
    ops.push(Operation::ValidateSchema { schema: to_schema });
    Plan {
        sequence: 0,
        label: StepLabel::new("0000-structural").unwrap(),
        from_schema: schema_id(from).unwrap(),
        to_schema,
        operations: ops,
        destructive: vec![],
    }
}
fn populated(row: Box<[Value]>) -> MigrationState {
    let from = SchemaDescriptor {
        relations: vec![],
        statements: vec![],
    };
    let to = source();
    let p = plan(
        &from,
        &to,
        vec![
            Operation::EmptyRelation {
                target: "Slice".into(),
            },
            Operation::Seed {
                target: "Slice".into(),
                rows: vec![row],
            },
        ],
    );
    MigrationState::empty()
        .apply(
            &compile(&p, &from, &to).unwrap(),
            &ScalarEvaluator::new().unwrap(),
            &WorkContext::new(),
        )
        .unwrap()
}

#[test]
fn old_unbounded_capacity_snapshots_keep_their_identity() {
    let old = r#"{"relations":[{"name":"Counted","fields":[{"name":"id","type":"u64"}]}],
        "statements":[{"functionality":{"relation":0,"projection":[0]}},
        {"capacity":{"target":{"relation":0,"projection":[0]},
        "weight":"unit","lo":"0","source":{"relation":0,"projection":[0]}}}]}"#;
    let descriptor = schema_file::parse(old).unwrap();
    let canonical = schema_file::render(&descriptor);
    assert!(canonical.contains("\"hi\":null"));
    assert_eq!(schema_file::parse(&canonical).unwrap(), descriptor);
    assert_eq!(
        schema_id(&descriptor).unwrap(),
        schema_id(&schema_file::parse(&canonical).unwrap()).unwrap()
    );
    assert!(
        schema_file::parse(&old.replace("\"lo\":\"0\"", "\"lo\":\"0\",\"future\":true")).is_err()
    );
}

fn measured_plan(rounding: Rounding) -> Plan {
    let from = source();
    let to = source();
    let expression = P::MulDiv {
        a: Box::new(P::Measure(Box::new(P::Field("span".into())))),
        b: Box::new(P::Literal(Value::U64(3))),
        divisor: Box::new(P::Literal(Value::U64(2))),
        rounding,
    };
    let mut p = plan(
        &from,
        &to,
        vec![Operation::MapRelation {
            source: "Slice".into(),
            target: "Slice".into(),
            fields: vec![
                FieldMap {
                    target: "span".into(),
                    expression: P::Field("span".into()),
                },
                FieldMap {
                    target: "amount".into(),
                    expression,
                },
            ],
        }],
    );
    p.destructive.push(bumbledb_log::migration::plan::Loss {
        relation: "Slice".into(),
        field: Some("amount".into()),
    });
    p
}

#[test]
fn scalar_plan_codec_preserves_rounding_and_refuses_malformed_frames() {
    let p = measured_plan(Rounding::NearestTiesAwayFromZero);
    let bytes = canonical_plan_bytes(&p, 1 << 20).unwrap();
    assert_eq!(decode_plan(&bytes, 1 << 20).unwrap(), p);
    for end in 0..bytes.len() {
        assert!(
            decode_plan(&bytes[..end], 1 << 20).is_err(),
            "truncated new scalar frame at {end}"
        );
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_plan(&trailing, 1 << 20).is_err());
    assert!(decode_plan(&bytes, bytes.len() - 1).is_err());
    let mut even = p.clone();
    let Operation::MapRelation { fields, .. } = &mut even.operations[0] else {
        unreachable!()
    };
    let P::MulDiv { rounding, .. } = &mut fields[1].expression else {
        unreachable!()
    };
    *rounding = Rounding::NearestTiesToEven;
    let even_bytes = canonical_plan_bytes(&even, 1 << 20).unwrap();
    let different: Vec<_> = bytes
        .iter()
        .zip(&even_bytes)
        .enumerate()
        .filter_map(|(i, (a, b))| (a != b).then_some(i))
        .collect();
    assert_eq!(different.len(), 1, "only the explicit rounding tag changes");
    let mut unknown = bytes.clone();
    unknown[different[0]] = 255;
    assert!(decode_plan(&unknown, 1 << 20).is_err());
    let text = render_plan(&p);
    assert_eq!(parse_plan(&text).unwrap(), p);
    assert!(
        parse_plan(&text.replace(
            "\"kind\":\"measure\"",
            "\"kind\":\"measure\",\"ignored\":true"
        ))
        .is_err()
    );
    assert!(parse_plan(&text.replace("nearestTiesAwayFromZero", "nearestSomething")).is_err());
}

#[test]
fn migrated_measure_and_all_rounding_modes_execute_and_preserve_failures() {
    let from = source();
    let to = source();
    for (rounding, expected) in [
        (Rounding::TowardZero, 7),
        (Rounding::NearestTiesAwayFromZero, 8),
        (Rounding::NearestTiesToEven, 8),
    ] {
        let p = measured_plan(rounding);
        let compiled = compile(&p, &from, &to).unwrap();
        let evaluator = ScalarEvaluator::new().unwrap();
        let work = WorkContext::new();
        let state = populated(Box::new([
            Value::IntervalI64(Interval::new(-2, 3).unwrap()),
            Value::U64(99),
        ]));
        let result = state.apply(&compiled, &evaluator, &work).unwrap();
        let mut count = 0;
        result
            .visit_canonical(RelationId(0), &mut |row| {
                assert_eq!(
                    bumbledb::canonical::decode(&to.relations[0].fields, row, &work)
                        .unwrap()
                        .values(),
                    &[
                        Value::IntervalI64(Interval::new(-2, 3).unwrap()),
                        Value::U64(expected)
                    ]
                );
                count += 1;
                Ok(true)
            })
            .unwrap();
        assert_eq!(count, 1);
        let ray = populated(Box::new([
            Value::IntervalI64(Interval::new(-2, i64::MAX).unwrap()),
            Value::U64(0),
        ]));
        assert!(matches!(
            ray.apply(&compiled, &evaluator, &work),
            Err(StateError::Scalar {
                error: bumbledb::ScalarError::UnboundedMeasure,
                ..
            })
        ));
    }
}
