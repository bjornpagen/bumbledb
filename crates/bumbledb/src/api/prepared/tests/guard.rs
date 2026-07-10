use super::*;

use crate::ir::{AggOp, ParamId};
use crate::schema::{IntervalElement, StatementDescriptor};
use crate::storage::dict;

/// The guard fast lane — hit, miss, and a
/// param-type error, with an interned find exercising the resolving
/// column beside the word blits.
#[test]
fn guard_fast_lane_hits_misses_and_type_errors() {
    let dir = TempDir::new("prepared-guard-lane");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "memo-a", 41), (2, 8, "memo-b", 42)]);
    // Q(account, memo, amount) :- Posting(id = ?0, account, memo, amount).
    let query = Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
                (FieldId(3), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let txn = env.read_txn().expect("txn");
    let cache = crate::image::cache::ImageCache::new();
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepares");
    assert!(
        prepared.guard_finds.is_some(),
        "plain-variable guard takes the fast lane"
    );
    let mut out = ResultBuffer::new();
    // Hit: every cell decoded straight from the fact.
    prepared
        .execute(&txn, &cache, &[BindValue::U64(2)], &mut out)
        .expect("hit");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::U64(8));
    assert_eq!(out.get(0, 1), ResultValue::String("memo-b"));
    assert_eq!(out.get(0, 2), ResultValue::I64(42));
    // Miss: clean empty buffer.
    prepared
        .execute(&txn, &cache, &[BindValue::U64(999)], &mut out)
        .expect("miss is empty, not an error");
    assert_eq!(out.len(), 0);
    // Param-type error: typed, before any probe.
    let err = prepared
        .execute(&txn, &cache, &[BindValue::Bool(true)], &mut out)
        .expect_err("type mismatch");
    assert!(matches!(err, Error::ParamTypeMismatch { .. }), "{err:?}");
}

/// The guard lane is stats-free end to end: a
/// guard prepare + execute builds NO image — and the lazy distinct
/// counts live on images, so no image means no stats walk, ever.
/// This is the isolation gate in its strongest form.
#[test]
fn a_guard_prepare_and_execute_build_no_image() {
    let dir = TempDir::new("prepared-guard-statsfree");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "memo-a", 41)]);
    let query = Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
                (FieldId(3), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let txn = env.read_txn().expect("txn");
    let cache = crate::image::cache::ImageCache::new();
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepares");
    assert!(prepared.guard_finds.is_some(), "the fast lane classified");
    let mut out = ResultBuffer::new();
    prepared
        .execute(&txn, &cache, &[BindValue::U64(1)], &mut out)
        .expect("hit");
    assert_eq!(out.len(), 1);
    #[cfg(feature = "trace")]
    assert_eq!(
        cache.resident(),
        (0, 0),
        "a guard execute must not build images (and so never walks stats)"
    );
}

#[test]
fn guard_probe_queries_flow_through_the_same_surface() {
    let dir = TempDir::new("prepared-guard");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(5, 7, "found", 42)]);
    let cache = ImageCache::new();
    // Q(amount) :- Posting(id = 5, amount) — the fresh key: guard probe.
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(5))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(matches!(prepared.plan, ExecPlan::GuardProbe(_)));
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::I64(42));

    // EXPLAIN reports the classification alongside the rows.
    let (rows, report) = prepared.explain(&txn, &cache, &[]).expect("explain");
    assert_eq!(rows.len(), 1);
    assert!(report.contains("guard probe"));
}

// ---------- PRD 19 criteria: statement-derived point lookups ----------

/// Booking(room u64, span interval<u64>, label u64) with the declared
/// pointwise key `Booking(room, span) -> Booking` (statement 0 — no
/// fresh ids exist).
fn booking_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
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

/// Commits `(room, [start, end), label)` bookings.
fn insert_bookings(env: &Environment, schema: &Schema, rows: &[(u64, (u64, u64), u64)]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (room, (start, end), label) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*room),
                ValueRef::IntervalU64(*start, *end),
                ValueRef::U64(*label),
            ],
            schema.relation(RelationId(0)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

/// Q(label) :- Booking(room = 1, span <op> ?span-term) with the span term
/// supplied by the caller — the by-value/membership twin queries.
fn booking_query(span_term: Term) -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: RelationId(0),
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(1))),
                (FieldId(1), span_term),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    }
}

/// A point lookup through the pointwise key with the interval bound **by
/// value**: one `U` get on the exact `scalar ‖ 16-byte interval` guard,
/// answered post-commit cold — no image build (cache-state inspection).
#[test]
fn pointwise_key_point_lookup_is_guarded_and_image_free() {
    let dir = TempDir::new("prepared-guard-pointwise");
    let schema = booking_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_bookings(
        &env,
        &schema,
        &[(1, (5, 10), 100), (1, (20, 30), 200), (2, (5, 10), 300)],
    );
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let query = booking_query(Term::Literal(Value::IntervalU64(5, 10)));
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(matches!(prepared.plan, ExecPlan::GuardProbe(_)));

    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::U64(100));
    #[cfg(feature = "trace")]
    assert_eq!(
        cache.resident(),
        (0, 0),
        "post-commit cold: the guard path builds no image"
    );

    // The classification is observable through profile stats, and the
    // 16-byte guard is exact: a one-off interval misses.
    let (rows, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert_eq!(rows.len(), 1);
    assert_eq!(
        stats.guard,
        Some(crate::api::stats::GuardStats { hit: true })
    );
    let near = booking_query(Term::Literal(Value::IntervalU64(5, 11)));
    let mut near = prepare(&txn, &cache, &schema, &near).expect("prepare");
    let (rows, stats) = near.profile(&txn, &cache, &[]).expect("profile");
    assert_eq!(rows.len(), 0);
    assert_eq!(
        stats.guard,
        Some(crate::api::stats::GuardStats { hit: false })
    );
    #[cfg(feature = "trace")]
    assert_eq!(cache.resident(), (0, 0));
}

/// A membership-bound single-atom query does NOT take the fast path — the
/// span binding is a point, not the key's interval value (validation's
/// typing, consumed through the lowered filter kind) — and executes
/// correctly via scan+filter; the path is asserted via profile stats.
#[test]
fn a_membership_bound_single_atom_query_stays_free_join() {
    let dir = TempDir::new("prepared-guard-membership");
    let schema = booking_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_bookings(
        &env,
        &schema,
        &[(1, (5, 10), 100), (1, (20, 30), 200), (2, (5, 10), 300)],
    );
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    // span ∋ 7 — a U64 *point* literal on the interval field is a
    // membership binding (validation's typing, by the literal's shape),
    // not a key cover.
    let query = booking_query(Term::Literal(Value::U64(7)));
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        matches!(prepared.plan, ExecPlan::FreeJoin(_)),
        "membership binding is not a key cover"
    );

    let (rows, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert!(stats.guard.is_none(), "the scan+filter path, not the guard");
    assert!(!stats.nodes.is_empty());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows.get(0, 0), ResultValue::U64(100));

    // Correct across points: 25 hits the other booking, 15 hits none.
    let query = booking_query(Term::Literal(Value::U64(25)));
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::U64(200));
    let query = booking_query(Term::Literal(Value::U64(15)));
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 0);
}

/// The full-fact `M` lookup with an interval field: no key statement
/// exists, every field is bound by value, and the existence check runs as
/// one membership get — post-commit cold, no image build.
#[test]
fn full_fact_membership_lookup_with_an_interval_field_is_image_free() {
    let dir = TempDir::new("prepared-guard-m-interval");
    // Stay(owner u64, span interval<u64>), no statements — no keys.
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
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
                    },
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture");
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let mut bytes = Vec::new();
    encode_fact(
        &[ValueRef::U64(2), ValueRef::IntervalU64(5, 10)],
        schema.relation(RelationId(0)).layout(),
        &mut bytes,
    );
    delta.insert(&view, RelationId(0), &bytes).expect("insert");
    drop(view);
    commit(delta, &env).expect("commit");

    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    // Q(count()) :- Stay(owner = 2, span = [5, 10)) — the existence shape.
    let count_stay = |span: (u64, u64)| Query {
        finds: vec![FindTerm::Aggregate {
            op: AggOp::Count,
            over: None,
        }],
        atoms: vec![Atom {
            relation: RelationId(0),
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(2))),
                (
                    FieldId(1),
                    Term::Literal(Value::IntervalU64(span.0, span.1)),
                ),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &count_stay((5, 10))).expect("prepare");
    assert!(matches!(prepared.plan, ExecPlan::GuardProbe(_)));
    let (_, report) = prepared.explain(&txn, &cache, &[]).expect("explain");
    assert!(report.contains("full-fact membership probe"), "{report}");

    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::U64(1));
    #[cfg(feature = "trace")]
    assert_eq!(
        cache.resident(),
        (0, 0),
        "post-commit cold: the M path builds no image"
    );

    // A different interval value is a different fact: the empty set (a
    // global aggregate over nothing emits no row, 20-query-ir).
    let mut absent = prepare(&txn, &cache, &schema, &count_stay((5, 11))).expect("prepare");
    let out = absent.execute_collect(&txn, &cache, &[]).expect("execute");
    assert_eq!(out.len(), 0);
}

/// An intern-miss param on the fast path: the key resolves to the
/// never-minted sentinel id, the `U` probe misses, the result is empty —
/// no error, and nothing is interned by the read path.
#[test]
fn intern_miss_param_on_the_fast_path_is_empty_not_an_error() {
    let dir = TempDir::new("prepared-guard-intern-miss");
    // Doc(name str, val u64) with the declared key `Doc(name) -> Doc`.
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Doc".into(),
            fields: vec![
                FieldDescriptor {
                    name: "name".into(),
                    value_type: ValueType::String,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "val".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }],
    }
    .validate()
    .expect("valid fixture");
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let name_id = delta.intern_str(&view, "alice").expect("intern");
    let mut bytes = Vec::new();
    encode_fact(
        &[ValueRef::String(name_id), ValueRef::U64(7)],
        schema.relation(RelationId(0)).layout(),
        &mut bytes,
    );
    delta.insert(&view, RelationId(0), &bytes).expect("insert");
    drop(view);
    commit(delta, &env).expect("commit");

    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    // Q(val) :- Doc(name = ?0, val).
    let query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: RelationId(0),
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(matches!(prepared.plan, ExecPlan::GuardProbe(_)));

    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::Str("ghost")])
        .expect("an intern miss is empty, not an error");
    assert_eq!(out.len(), 0);
    assert_eq!(
        dict::lookup_str(&txn, "ghost").expect("lookup"),
        None,
        "the read path never interns"
    );

    // The same prepared query hits once the key resolves.
    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::Str("alice")])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), ResultValue::U64(7));
}
