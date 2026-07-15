use super::*;
use crate::encoding::{ValueRef, encode_fact};
use crate::exec::run::Bindings;
use crate::exec::sink::{AggregateSink, FindSpec, FoldOp, ProjectionSink};
use crate::image::view::ResolvedWordSource;
use crate::ir::normalize::{NormalizedQuery, OccId, Occurrence, PlacedComparison, Role, SlotWidth};
use crate::ir::{CmpOp, ParamId, VarId};
use crate::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, RelationId, Schema,
    SchemaDescriptor, StatementDescriptor, StatementId, ValueType,
};
use crate::storage::commit::commit;
use crate::storage::delta::WriteDelta;
use crate::storage::dict;
use crate::storage::env::Environment;
use crate::testutil::TempDir;

/// Account(id fresh u64, holder u64, name string): statement 0 is the
/// fresh auto-key on `id`.
fn account_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Account".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "holder".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "name".into(),
                    value_type: ValueType::String,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

/// Booking(room u64, span interval<u64>, label u64) with the declared
/// pointwise key `Booking(room, span) -> Booking` — statement 0 (no
/// fresh ids exist).
fn booking_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Booking".into(),
            fields: vec![
                FieldDescriptor {
                    name: "room".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::U64,
                        width: None,
                    },
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "label".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
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

/// Stay(owner u64, span interval<u64>) with no statements: no key exists,
/// so only the full-fact `M` path can serve a point lookup.
fn stay_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Stay".into(),
            fields: vec![
                FieldDescriptor {
                    name: "owner".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::U64,
                        width: None,
                    },
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

/// Shift(id fresh u64, span interval<u64>): the fresh auto-key plus an
/// interval field to decode as a two-slot variable.
fn shift_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Shift".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::U64,
                        width: None,
                    },
                    generation: Generation::None,
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
        source: crate::ir::AtomSource::Edb(REL),
        role: Role::Positive,
        vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
        filters,
    }
}

fn eq_filter(field: u16, value: Const) -> FilterPredicate {
    FilterPredicate::Compare {
        field: FieldId(field),
        op: CmpOp::Eq,
        value,
    }
}

/// Wraps one occurrence with the given per-var slot widths (`(var, two)`).
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
        duration_residuals: Vec::new(),
        anti_probes: vec![],
        slot_widths,
    }
}

fn single(occurrence: Occurrence) -> NormalizedQuery {
    single_with_widths(occurrence, &[])
}

/// Commits accounts (id, holder, name) and returns the environment.
fn populated_accounts(dir: &TempDir, schema: &Schema, rows: &[(u64, u64, &str)]) -> Environment {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, holder, name) in rows {
        let name_id = delta.intern_str(&view, name).expect("intern");
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*holder),
                ValueRef::String(name_id),
            ],
            schema.relation(REL).layout(),
            &mut bytes,
        );
        delta.insert(&view, REL, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    env
}

/// Commits facts of pre-encoded values and returns the environment.
fn populated(dir: &TempDir, schema: &Schema, rows: &[Vec<ValueRef>]) -> Environment {
    let env = Environment::create(dir.path(), schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for values in rows {
        let mut bytes = Vec::new();
        encode_fact(values, schema.relation(REL).layout(), &mut bytes);
        delta.insert(&view, REL, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, &env).expect("commit");
    env
}

// ---------- classification ----------

#[test]
fn fully_key_bound_single_atom_classifies_as_key_probe() {
    let schema = account_schema();
    let normalized = single(occurrence(
        &[(1, 0), (2, 1)],
        vec![eq_filter(0, Const::Word(5))], // id = 5, the fresh auto-key
    ));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert_eq!(plan.statement, Some(StatementId(0)));
    assert_eq!(plan.key, vec![(FieldId(0), Const::Word(5))]);
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
        duration_residuals: Vec::new(),
        anti_probes: vec![],
        slot_widths: [(VarId(0), SlotWidth::ONE)].into_iter().collect(),
    };
    assert!(classify(&two_atoms, &schema).is_none());

    let mut with_residual = single(occurrence(
        &[(1, 0), (2, 1)],
        vec![eq_filter(0, Const::Word(5))],
    ));
    with_residual.residuals.push(PlacedComparison {
        op: CmpOp::Lt,
        lhs: VarId(0),
        rhs: VarId(1),
    });
    assert!(classify(&with_residual, &schema).is_none());
}

/// The closed Currency { `minor_units` } = { Usd(2), Eur(0) }: statement 0
/// is the closed auto-key on the synthetic `id`.
fn currency_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: Some(Box::new([
                crate::schema::Row {
                    handle: "Usd".into(),
                    values: Box::new([crate::ir::Value::U64(2)]),
                },
                crate::schema::Row {
                    handle: "Eur".into(),
                    values: Box::new([crate::ir::Value::U64(0)]),
                },
            ])),
            name: "Currency".into(),
            fields: vec![FieldDescriptor {
                name: "minor_units".into(),
                value_type: ValueType::U64,
                generation: Generation::None,
            }],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

/// A closed relation never takes the key-probe path: no `U` determinants and no `M`
/// entries exist — its storage is the theory — so even a single atom
/// fully binding the auto-key (or every field) classifies as Free Join
/// and hits the virtual image.
#[test]
fn a_closed_relation_stays_free_join_even_fully_bound() {
    let schema = currency_schema();
    // id = 1 covers the closed auto-key's whole projection.
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
    // Only a non-key field is constant: no key coverage, not full-fact.
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
        vec![
            eq_filter(0, Const::Word(5)),
            eq_filter(1, Const::Word(7)), // outside the key's projection
        ],
    ));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert_eq!(plan.remaining_filters, vec![eq_filter(1, Const::Word(7))]);
}

#[test]
fn a_pointwise_key_covered_by_value_classifies_with_its_statement() {
    let schema = booking_schema();
    // room = 1, span = [5, 10) — the interval bound by an interval-typed
    // term (an Eq Compare against an Interval constant).
    let normalized = single(occurrence(
        &[(2, 0)],
        vec![
            eq_filter(0, Const::Word(1)),
            eq_filter(1, Const::Interval { start: 5, end: 10 }),
        ],
    ));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert_eq!(plan.statement, Some(StatementId(0)));
    // Key constants in statement projection order.
    assert_eq!(
        plan.key,
        vec![
            (FieldId(0), Const::Word(1)),
            (FieldId(1), Const::Interval { start: 5, end: 10 }),
        ]
    );
    assert!(plan.remaining_filters.is_empty());
}

#[test]
fn a_membership_binding_is_not_a_key_cover() {
    let schema = booking_schema();
    // room = 1, span ∋ 7: lowering typed the span binding as membership
    // (`PointIn`), so the pointwise key is NOT covered — the dispatch
    // reads the filter kind and never re-infers membership vs equality.
    let normalized = single(occurrence(
        &[(2, 0)],
        vec![
            eq_filter(0, Const::Word(1)),
            FilterPredicate::PointIn {
                field: FieldId(1),
                point: ResolvedWordSource::Word(7),
            },
        ],
    ));
    assert!(classify(&normalized, &schema).is_none());
}

#[test]
fn a_param_set_bound_field_disqualifies_the_fast_path() {
    let schema = account_schema();
    // The key itself is set-bound: k gets would serve it, but v0 routes
    // sets to the selection-level path (the classify decision comment).
    let on_key = single(occurrence(
        &[(1, 0)],
        vec![eq_filter(0, Const::ParamSet(ParamId(0)))],
    ));
    assert!(classify(&on_key, &schema).is_none());
    // A set beside a covered key disqualifies too.
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
    assert_eq!(plan.statement, None);
    assert_eq!(plan.key.len(), 2, "every field, declaration order");
    assert!(plan.remaining_filters.is_empty());

    // A membership binding does not bind the interval field's value:
    // not full-fact either → Free Join.
    let membership = single(occurrence(
        &[],
        vec![
            eq_filter(0, Const::Word(2)),
            FilterPredicate::PointIn {
                field: FieldId(1),
                point: ResolvedWordSource::Word(7),
            },
        ],
    ));
    assert!(classify(&membership, &schema).is_none());
}

// ---------- execution ----------

fn run_key_probe(
    plan: &KeyProbePlan,
    env: &Environment,
    schema: &Schema,
    params: &[Const],
) -> Vec<Vec<u64>> {
    let txn = env.read_txn().expect("txn");
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = ProjectionSink::new((0..plan.slot_count()).collect());
    let mut key = Vec::new();
    execute_key_probe(
        plan,
        &txn,
        schema,
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
    let dir = TempDir::new("key_probe-hit-miss");
    let schema = account_schema();
    let env = populated_accounts(&dir, &schema, &[(5, 7, "alice"), (6, 8, "bob")]);
    let normalized = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(5))]));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert_eq!(run_key_probe(&plan, &env, &schema, &[]), vec![vec![7]]);

    // Miss: no such id.
    let missing = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(99))]));
    let plan = classify(&missing, &schema).expect("key probe");
    assert!(run_key_probe(&plan, &env, &schema, &[]).is_empty());

    // Hit, but a remaining filter rejects the fetched fact.
    let rejected = single(occurrence(
        &[(1, 0)],
        vec![
            eq_filter(0, Const::Word(5)),
            eq_filter(1, Const::Word(999)), // holder is 7, not 999
        ],
    ));
    let plan = classify(&rejected, &schema).expect("key probe");
    assert!(run_key_probe(&plan, &env, &schema, &[]).is_empty());
}

#[test]
fn param_driven_keys_resolve_at_bind_time() {
    let dir = TempDir::new("key_probe-param");
    let schema = account_schema();
    let env = populated_accounts(&dir, &schema, &[(5, 7, "alice")]);
    let normalized = single(occurrence(
        &[(1, 0)],
        vec![eq_filter(0, Const::Param(ParamId(0)))],
    ));
    let plan = classify(&normalized, &schema).expect("key probe");
    assert_eq!(
        run_key_probe(&plan, &env, &schema, &[Const::Word(5)]),
        vec![vec![7]]
    );
    assert!(run_key_probe(&plan, &env, &schema, &[Const::Word(6)]).is_empty());
}

#[test]
fn pending_intern_miss_is_empty_and_never_interns() {
    let dir = TempDir::new("key_probe-intern-miss");
    let schema = account_schema();
    let env = populated_accounts(&dir, &schema, &[(5, 7, "alice")]);
    // Probe by id, filter on a never-interned name.
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
    assert!(run_key_probe(&plan, &env, &schema, &[]).is_empty());
    // The read path never interned the ghost string.
    let txn = env.read_txn().expect("txn");
    assert_eq!(dict::lookup_str(&txn, "ghost").expect("lookup"), None);
}

/// The pointwise `U` hit: the determinant key carries the interval's exact
/// 16-byte encoding — byte-identical to what the write-side slicer
/// ([`crate::storage::keys::determinant_image`]) derived from the stored fact.
#[test]
fn pointwise_key_probe_hit_is_byte_exact() {
    let dir = TempDir::new("key_probe-pointwise");
    let schema = booking_schema();
    let env = populated(
        &dir,
        &schema,
        &[
            vec![
                ValueRef::U64(1),
                ValueRef::IntervalU64(
                    crate::Interval::<u64>::new(5, 10).expect("nonempty interval"),
                ),
                ValueRef::U64(100),
            ],
            vec![
                ValueRef::U64(1),
                ValueRef::IntervalU64(
                    crate::Interval::<u64>::new(20, 30).expect("nonempty interval"),
                ),
                ValueRef::U64(200),
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
    assert_eq!(plan.statement, Some(StatementId(0)));

    let txn = env.read_txn().expect("txn");
    let mut key = Vec::new();
    let fact = key_probe_fact(&plan, &txn, &schema, &[], &mut key)
        .expect("probe")
        .expect("hit");
    // The probe key equals the shared slicer's determinant bytes for the fact.
    let mut expected = crate::storage::keys::DeterminantImage::scratch();
    crate::storage::keys::determinant_image(
        schema.relation(REL).layout(),
        &[FieldId(0), FieldId(1)],
        fact,
        &mut expected,
    );
    assert_eq!(key, expected.as_bytes());
    assert_eq!(key.len(), 8 + 16, "scalar word + whole 16-byte interval");
    assert_eq!(run_key_probe(&plan, &env, &schema, &[]), vec![vec![100]]);

    // The 16 bytes are exact: a one-off end misses.
    let near = single(occurrence(
        &[(2, 0)],
        vec![
            eq_filter(0, Const::Word(1)),
            eq_filter(1, Const::Interval { start: 5, end: 11 }),
        ],
    ));
    let plan = classify(&near, &schema).expect("key probe");
    assert!(run_key_probe(&plan, &env, &schema, &[]).is_empty());
}

#[test]
fn full_fact_membership_lookup_with_an_interval_field() {
    let dir = TempDir::new("key_probe-m-interval");
    let schema = stay_schema();
    let env = populated(
        &dir,
        &schema,
        &[vec![
            ValueRef::U64(2),
            ValueRef::IntervalU64(crate::Interval::<u64>::new(5, 10).expect("nonempty interval")),
        ]],
    );
    let exact = single(occurrence(
        &[],
        vec![
            eq_filter(0, Const::Word(2)),
            eq_filter(1, Const::Interval { start: 5, end: 10 }),
        ],
    ));
    let plan = classify(&exact, &schema).expect("key probe");
    assert_eq!(plan.statement, None, "the M path");
    let txn = env.read_txn().expect("txn");
    let mut key = Vec::new();
    assert!(
        key_probe_fact(&plan, &txn, &schema, &[], &mut key)
            .expect("probe")
            .is_some()
    );

    // A different interval value is a different fact: miss.
    let other = single(occurrence(
        &[],
        vec![
            eq_filter(0, Const::Word(2)),
            eq_filter(1, Const::Interval { start: 5, end: 11 }),
        ],
    ));
    let plan = classify(&other, &schema).expect("key probe");
    let mut key = Vec::new();
    assert!(
        key_probe_fact(&plan, &txn, &schema, &[], &mut key)
            .expect("probe")
            .is_none()
    );
}

#[test]
fn an_interval_variable_decodes_into_its_two_slot_span() {
    let dir = TempDir::new("key_probe-interval-var");
    let schema = shift_schema();
    let env = populated(
        &dir,
        &schema,
        &[vec![
            ValueRef::U64(1),
            ValueRef::IntervalU64(crate::Interval::<u64>::new(5, 10).expect("nonempty interval")),
        ]],
    );
    // Q(span) :- Shift(id = 1, span) — span is a two-word variable.
    let normalized = single_with_widths(
        occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(1))]),
        &[0],
    );
    let plan = classify(&normalized, &schema).expect("key probe");
    assert_eq!(plan.slot_count(), 2);
    assert_eq!(
        run_key_probe(&plan, &env, &schema, &[]),
        vec![vec![5, 10]],
        "start and end words in the SlotWidth layout"
    );
}

#[test]
fn aggregate_over_a_point_lookup_folds_one_binding() {
    let dir = TempDir::new("key_probe-aggregate");
    let schema = account_schema();
    let env = populated_accounts(&dir, &schema, &[(5, 7, "alice")]);
    let normalized = single(occurrence(&[(1, 0)], vec![eq_filter(0, Const::Word(5))]));
    let plan = classify(&normalized, &schema).expect("key probe");
    let txn = env.read_txn().expect("txn");
    let mut bindings = Bindings::new(1);
    let mut sink = AggregateSink::new(
        vec![FindSpec::Agg {
            op: FoldOp::Count,
            over_slot: None,
            over_width: 1,
            signed: false,
        }],
        1,
    );
    let mut key = Vec::new();
    execute_key_probe(
        &plan,
        &txn,
        &schema,
        &[],
        &mut key,
        &mut bindings,
        &mut sink,
        &mut crate::exec::run::NoopCounters,
    )
    .expect("execute");
    assert_eq!(sink.into_answers().expect("rows"), vec![vec![1]]);
}

// No image build can occur on the key-probe path: `execute_key_probe` takes no
// image, view, or cache argument — the property holds by API shape, on
// a cold database in every test above.
