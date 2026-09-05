use super::*;
use crate::exec::run::Bindings;
use crate::exec::sink::{AggSpec, AggregateSink, FindSpec, ProjectionSink};
use crate::image::intern::InternerHandle;
use crate::image::testsupport::TestSource;
use crate::image::view::{FilterPredicate, OperandAddr, ViewWordSource};
use crate::ir::Value;
use crate::ir::WordCmp;
use crate::ir::normalize::{NormalizedQuery, OccBind, OccId, Occurrence, Role, SlotWidth};
use crate::ir::{ParamId, VarId};
use crate::schema::Schema;
use crate::schema::ValidateDescriptor as _;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, IntervalElement, RelationDescriptor, RelationId, SchemaDescriptor,
    StatementDescriptor, StatementId, ValueType,
};

fn account_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Account".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "holder".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "name".into(),
                    value_type: ValueType::String,
                },
            ],
        }],
        // The declared id key (the deleted fresh auto-key's position).
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }],
    }
    .validate()
    .expect("valid fixture")
}

fn booking_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Booking".into(),
            fields: vec![
                FieldDescriptor {
                    name: "room".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                },
                FieldDescriptor {
                    name: "label".into(),
                    value_type: ValueType::U64,
                },
            ],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0), FieldId(1)]),
        }],
    }
    .validate()
    .expect("valid fixture")
}

fn stay_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Stay".into(),
            fields: vec![
                FieldDescriptor {
                    name: "owner".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

fn shift_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Shift".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const REL: RelationId = RelationId(0);

fn occurrence(vars: &[(u16, u16)], filters: Vec<FilterPredicate>) -> Occurrence {
    Occurrence {
        occ_id: OccId(0),
        bind: OccBind::Edb(REL),
        role: Role::Positive,
        vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
        filters,
        point_vars: vec![],
    }
}

fn account_source(rows: &[(u64, u64, &str)]) -> TestSource {
    let facts: Vec<Vec<Value>> = rows
        .iter()
        .map(|(id, holder, name)| {
            vec![
                Value::U64(*id),
                Value::U64(*holder),
                Value::String((*name).into()),
            ]
        })
        .collect();
    TestSource::new(&account_schema(), &[(REL, facts)])
}

fn value_source(schema: &Schema, rows: &[Vec<Value>]) -> TestSource {
    TestSource::new(schema, &[(REL, rows.to_vec())])
}

fn eq_filter(field: u16, value: Const) -> FilterPredicate {
    FilterPredicate::Compare {
        field: FieldId(field).into(),
        op: WordCmp::Eq,
        value,
    }
}

fn single_with_widths(occurrence: Occurrence, wide_vars: &[u16]) -> NormalizedQuery {
    let slot_widths = occurrence
        .vars
        .iter()
        .map(|(_, var)| {
            let width = if wide_vars.contains(&var.0) {
                SlotWidth::TWO
            } else {
                SlotWidth::ONE
            };
            (*var, width)
        })
        .collect();
    NormalizedQuery {
        dead: None,
        occurrences: vec![occurrence],
        residuals: vec![],
        word_residuals: vec![],
        allen_residuals: Vec::new(),
        anti_probes: vec![],
        slot_widths,
    }
}

fn single(occurrence: Occurrence) -> NormalizedQuery {
    single_with_widths(occurrence, &[])
}

#[test]
fn fully_key_bound_single_atom_classifies_as_key_probe() {
    let schema = account_schema();
    let normalized = single(occurrence(
        &[(1, 0), (2, 1)],
        vec![eq_filter(0, Const::Word(5))],
    ));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert!(matches!(
        &plan.kind,
        KeyProbeKind::Uniqueness {
            statement: StatementId(0),
            ..
        }
    ));
    assert_eq!(plan.kind.key(), &[(FieldId(0), Const::Word(5))]);
    assert!(plan.remaining_filters.is_empty());
    assert_eq!(plan.slot_count(), 2);
}

#[test]
fn a_second_atom_or_a_residual_stays_free_join() {
    let schema = account_schema();
    let occ = occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(5))]);
    let two_atoms = NormalizedQuery {
        dead: None,
        occurrences: vec![occ.clone(), occ],
        residuals: vec![],
        word_residuals: vec![],
        allen_residuals: Vec::new(),
        anti_probes: vec![],
        slot_widths: [(VarId(0), SlotWidth::ONE)].into_iter().collect(),
    };
    assert!(classify(&two_atoms, &schema).is_none());

    let mut with_residual = single(occurrence(
        &[(1, 0), (2, 1)],
        vec![eq_filter(0, Const::Word(5))],
    ));
    with_residual
        .residuals
        .push(FilterPredicate::FieldsCompare {
            left: OperandAddr::from(VarId(0)),
            right: OperandAddr::from(VarId(1)),
            op: WordCmp::Lt,
        });
    assert!(classify(&with_residual, &schema).is_none());
}

fn currency_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: Some(Box::new([
                bumbledb_theory::schema::Row {
                    handle: "Usd".into(),
                    values: Box::new([crate::ir::Value::U64(2)]),
                },
                bumbledb_theory::schema::Row {
                    handle: "Eur".into(),
                    values: Box::new([crate::ir::Value::U64(0)]),
                },
            ])),
            name: "Currency".into(),
            fields: vec![FieldDescriptor {
                name: "minor_units".into(),
                value_type: ValueType::U64,
            }],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

#[test]
fn a_closed_relation_stays_free_join_even_fully_bound() {
    let schema = currency_schema();

    let key_bound = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(1))]));
    assert!(classify(&key_bound, &schema).is_none());
    // Every field bound by value: the full-fact `M` path is refused too.
    let fully_bound = single(occurrence(
        &[],
        vec![eq_filter(0, Const::Word(1)), eq_filter(1, Const::Word(0))],
    ));
    assert!(classify(&fully_bound, &schema).is_none());
}

#[test]
fn a_partially_bound_key_stays_free_join() {
    let schema = account_schema();

    let normalized = single(occurrence(
        &[(0, 0), (2, 1)],
        vec![eq_filter(1, Const::Word(9))],
    ));
    assert!(classify(&normalized, &schema).is_none());
}

#[test]
fn extra_filters_survive_as_remaining() {
    let schema = account_schema();
    let normalized = single(occurrence(
        &[(2, 0)],
        vec![eq_filter(0, Const::Word(5)), eq_filter(1, Const::Word(7))],
    ));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert_eq!(plan.remaining_filters, vec![eq_filter(1, Const::Word(7))]);
}

#[test]
fn a_pointwise_key_covered_by_value_classifies_with_its_statement() {
    let schema = booking_schema();

    let normalized = single(occurrence(
        &[(2, 0)],
        vec![
            eq_filter(0, Const::Word(1)),
            eq_filter(1, Const::Interval { start: 5, end: 10 }),
        ],
    ));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert!(matches!(
        &plan.kind,
        KeyProbeKind::Uniqueness {
            statement: StatementId(0),
            ..
        }
    ));

    assert_eq!(
        plan.kind.key(),
        &[
            (FieldId(0), Const::Word(1)),
            (FieldId(1), Const::Interval { start: 5, end: 10 }),
        ]
    );
    assert!(plan.remaining_filters.is_empty());
}

#[test]
fn a_membership_binding_is_not_a_key_cover() {
    let schema = booking_schema();

    let normalized = single(occurrence(
        &[(2, 0)],
        vec![
            eq_filter(0, Const::Word(1)),
            FilterPredicate::PointIn {
                field: FieldId(1).into(),
                point: ViewWordSource::Word(7),
                dense: false,
            },
        ],
    ));
    assert!(classify(&normalized, &schema).is_none());
}

#[test]
fn a_param_set_bound_field_disqualifies_the_fast_path() {
    let schema = account_schema();

    let on_key = single(occurrence(
        &[(1, 0)],
        vec![eq_filter(0, Const::ParamSet(ParamId(0)))],
    ));
    assert!(classify(&on_key, &schema).is_none());

    let beside_key = single(occurrence(
        &[(2, 0)],
        vec![
            eq_filter(0, Const::Word(5)),
            eq_filter(1, Const::ParamSet(ParamId(0))),
        ],
    ));
    assert!(classify(&beside_key, &schema).is_none());
}

#[test]
fn full_fact_binding_takes_the_membership_path() {
    let schema = stay_schema();
    // No key statements exist; every field bound by value → `M` probe.
    let normalized = single(occurrence(
        &[],
        vec![
            eq_filter(0, Const::Word(2)),
            eq_filter(1, Const::Interval { start: 5, end: 10 }),
        ],
    ));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert!(matches!(plan.kind, KeyProbeKind::Membership { .. }));
    assert_eq!(plan.kind.key().len(), 2, "every field, declaration order");
    assert!(plan.remaining_filters.is_empty());

    // not full-fact either → Free Join.
    let membership = single(occurrence(
        &[],
        vec![
            eq_filter(0, Const::Word(2)),
            FilterPredicate::PointIn {
                field: FieldId(1).into(),
                point: ViewWordSource::Word(7),
                dense: false,
            },
        ],
    ));
    assert!(classify(&membership, &schema).is_none());
}

fn run_key_probe(
    plan: &KeyProbePlan,
    fixture: &TestSource,
    schema: &Schema,
    params: &[Const],
) -> Vec<Vec<u64>> {
    let cache = crate::image::cache::ImageCache::new(schema);
    let source = fixture.source();
    let interner = InternerHandle::new(cache.interner(), source.work());
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = ProjectionSink::new((0..plan.slot_count()).collect());
    let mut key = Vec::new();
    execute_key_probe(
        plan,
        &source,
        schema,
        &interner,
        params,
        &mut key,
        &mut bindings,
        &mut sink,
        &mut crate::exec::run::NoopCounters,
    )
    .expect("execute");
    sink.answers().map(<[u64]>::to_vec).collect()
}

#[test]
fn hit_miss_and_filter_rejection() {
    let schema = account_schema();
    let fixture = account_source(&[(5, 7, "alice"), (6, 8, "bob")]);
    let normalized = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(5))]));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert_eq!(run_key_probe(&plan, &fixture, &schema, &[]), vec![vec![7]]);

    let missing = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(99))]));
    let plan = classify(&missing, &schema).expect("key probe");
    assert!(run_key_probe(&plan, &fixture, &schema, &[]).is_empty());

    let rejected = single(occurrence(
        &[(1, 0)],
        vec![eq_filter(0, Const::Word(5)), eq_filter(1, Const::Word(999))],
    ));
    let plan = classify(&rejected, &schema).expect("key probe");
    assert!(run_key_probe(&plan, &fixture, &schema, &[]).is_empty());
}

#[test]
fn param_driven_keys_resolve_at_bind_time() {
    let schema = account_schema();
    let fixture = account_source(&[(5, 7, "alice")]);
    let normalized = single(occurrence(
        &[(1, 0)],
        vec![eq_filter(0, Const::Param(ParamId(0)))],
    ));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert_eq!(
        run_key_probe(&plan, &fixture, &schema, &[Const::Word(5)]),
        vec![vec![7]]
    );
    assert!(run_key_probe(&plan, &fixture, &schema, &[Const::Word(6)]).is_empty());
}

#[test]
fn an_unstored_pending_literal_is_empty_not_an_error() {
    let schema = account_schema();
    let fixture = account_source(&[(5, 7, "alice")]);

    // The pending literal latches to a fresh interner token; no stored
    // text equals it, so the residual filter rejects the matched row.
    let normalized = single(occurrence(
        &[(1, 0)],
        vec![
            eq_filter(0, Const::Word(5)),
            eq_filter(
                2,
                Const::PendingIntern {
                    bytes: Box::from(&b"ghost"[..]),
                },
            ),
        ],
    ));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert!(run_key_probe(&plan, &fixture, &schema, &[]).is_empty());
}

#[test]
fn pointwise_key_probe_hit_is_byte_exact() {
    let schema = booking_schema();
    let fixture = value_source(
        &schema,
        &[
            vec![
                Value::U64(1),
                Value::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
                ),
                Value::U64(100),
            ],
            vec![
                Value::U64(1),
                Value::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(20, 30).expect("nonempty interval"),
                ),
                Value::U64(200),
            ],
        ],
    );
    let normalized = single(occurrence(
        &[(2, 0)],
        vec![
            eq_filter(0, Const::Word(1)),
            eq_filter(1, Const::Interval { start: 5, end: 10 }),
        ],
    ));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert!(matches!(
        &plan.kind,
        KeyProbeKind::Uniqueness {
            statement: StatementId(0),
            ..
        }
    ));
    assert_eq!(
        run_key_probe(&plan, &fixture, &schema, &[]),
        vec![vec![100]]
    );

    // The exact-bound twin one past the end misses: interval keys compare
    // by both endpoint words, never a prefix.
    let near = single(occurrence(
        &[(2, 0)],
        vec![
            eq_filter(0, Const::Word(1)),
            eq_filter(1, Const::Interval { start: 5, end: 11 }),
        ],
    ));
    let plan = classify(&near, &schema).expect("key probe");
    assert!(run_key_probe(&plan, &fixture, &schema, &[]).is_empty());
}

#[test]
fn full_fact_membership_lookup_with_an_interval_field() {
    let schema = stay_schema();
    let fixture = value_source(
        &schema,
        &[vec![
            Value::U64(2),
            Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
            ),
        ]],
    );
    let probe = |span: (u64, u64)| {
        let normalized = single(occurrence(
            &[],
            vec![
                eq_filter(0, Const::Word(2)),
                eq_filter(
                    1,
                    Const::Interval {
                        start: span.0,
                        end: span.1,
                    },
                ),
            ],
        ));
        let plan = classify(&normalized, &schema).expect("key probe");
        assert!(
            matches!(plan.kind, KeyProbeKind::Membership { .. }),
            "the M path"
        );
        let cache = crate::image::cache::ImageCache::new(&schema);
        let source = fixture.source();
        let interner = InternerHandle::new(cache.interner(), source.work());
        let field_types: Vec<ValueType> = schema
            .relation(REL)
            .fields()
            .iter()
            .map(|f| f.value_type)
            .collect();
        let mut row = crate::image::canon::RowWords::new(&field_types);
        let mut key = Vec::new();
        key_probe_row(&plan, &source, &schema, &interner, &[], &mut row, &mut key).expect("probe")
    };
    assert!(probe((5, 10)), "exact membership hits");
    assert!(!probe((5, 11)), "one past the end misses");
}

#[test]
fn an_interval_variable_decodes_into_its_two_slot_span() {
    let schema = shift_schema();
    let fixture = value_source(
        &schema,
        &[vec![
            Value::U64(1),
            Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
            ),
        ]],
    );

    let normalized = single_with_widths(
        occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(1))]),
        &[0],
    );
    let plan = classify(&normalized, &schema).expect("key probe");
    assert_eq!(plan.slot_count(), 2);
    assert_eq!(
        run_key_probe(&plan, &fixture, &schema, &[]),
        vec![vec![5, 10]],
        "start and end words in the SlotWidth layout"
    );
}

#[test]
fn aggregate_over_a_point_lookup_folds_one_binding() {
    let schema = account_schema();
    let fixture = account_source(&[(5, 7, "alice")]);
    let normalized = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(5))]));
    let plan = classify(&normalized, &schema).expect("key probe");
    let cache = crate::image::cache::ImageCache::new(&schema);
    let source = fixture.source();
    let interner = InternerHandle::new(cache.interner(), source.work());
    let mut bindings = Bindings::new(1);
    let mut sink = AggregateSink::new(vec![FindSpec::Agg(AggSpec::Count)], 1);
    let mut key = Vec::new();
    execute_key_probe(
        &plan,
        &source,
        &schema,
        &interner,
        &[],
        &mut key,
        &mut bindings,
        &mut sink,
        &mut crate::exec::run::NoopCounters,
    )
    .expect("execute");
    assert_eq!(sink.into_answers().expect("rows"), vec![vec![1]]);
}
