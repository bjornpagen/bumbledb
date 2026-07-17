use super::*;

use crate::ir::{AggOp, ParamId};
use crate::storage::dict;
use bumbledb_theory::schema::{IntervalElement, StatementDescriptor};

/// The key-probe fast lane — hit, miss, and a
/// param-type error, with an interned find exercising the resolving
/// column beside the word blits.
#[test]
fn key_probe_fast_lane_hits_misses_and_type_errors() {
    let dir = TempDir::new("prepared-key_probe-lane");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "memo-a", 41), (2, 8, "memo-b", 42)]);
    // Q(account, memo, amount) :- Posting(id = ?0, account, memo, amount).
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
                (FieldId(3), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let txn = env.read_txn().expect("txn");
    let cache = crate::image::cache::ImageCache::new(&schema);
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepares");
    assert!(
        matches!(
            prepared.program.rules(),
            [PreparedRule::KeyProbe(KeyProbeRule {
                key_probe_finds: Some(_),
                ..
            })]
        ),
        "plain-variable key_probe takes the fast lane"
    );
    let mut out = Answers::new();
    // Hit: every cell decoded straight from the fact.
    prepared
        .execute(&txn, &cache, &[BindValue::U64(2)], &mut out)
        .expect("hit");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(8));
    assert_eq!(out.get(0, 1), AnswerValue::String("memo-b"));
    assert_eq!(out.get(0, 2), AnswerValue::I64(42));
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

/// The key-probe lane is stats-free end to end: a
/// key-probe preparation + execute builds NO image — and the lazy distinct
/// counts live on images, so no image means no stats walk, ever.
/// This is the isolation gate in its strongest form.
#[test]
fn a_key_probe_prepare_and_execute_build_no_image() {
    let dir = TempDir::new("prepared-key_probe-statsfree");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "memo-a", 41)]);
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
                (FieldId(3), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let txn = env.read_txn().expect("txn");
    let cache = crate::image::cache::ImageCache::new(&schema);
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepares");
    assert!(
        matches!(
            prepared.program.rules(),
            [PreparedRule::KeyProbe(KeyProbeRule {
                key_probe_finds: Some(_),
                ..
            })]
        ),
        "the fast lane classified"
    );
    let mut out = Answers::new();
    prepared
        .execute(&txn, &cache, &[BindValue::U64(1)], &mut out)
        .expect("hit");
    assert_eq!(out.len(), 1);
    #[cfg(feature = "trace")]
    assert_eq!(
        cache.resident(),
        (0, 0),
        "a key-probe execution must not build images (and so never walks stats)"
    );
}

#[test]
fn key_probe_queries_flow_through_the_same_surface() {
    let dir = TempDir::new("prepared-key_probe");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(5, 7, "found", 42)]);
    let cache = ImageCache::new(&schema);
    // Q(amount) :- Posting(id = 5, amount) — the fresh key: key probe.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(5))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(matches!(
        prepared.program.rules(),
        [PreparedRule::KeyProbe(_)]
    ));
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::I64(42));

    // introspection reports the classification alongside the rows.
    let (answers, report) = prepared.introspect(&txn, &cache, &[]).expect("introspect");
    assert_eq!(answers.len(), 1);
    assert!(report.contains("key probe"));
}

// ---------- PRD 19 criteria: statement-derived point lookups ----------

/// Booking(room u64, span interval<u64>, label u64) with the declared
/// pointwise key `Booking(room, span) -> Booking` (statement 0 — no
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

/// Commits `(room, [start, end), label)` bookings.
fn insert_bookings(env: &Environment, schema: &Schema, rows: &[(u64, (u64, u64), u64)]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (room, (start, end), label) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*room),
                ValueRef::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(*start, *end).expect("nonempty interval"),
                ),
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
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(1))),
                (FieldId(1), span_term),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// A point lookup through the pointwise key with the interval bound **by
/// value**: one `U` get on the exact `scalar ‖ 16-byte interval` determinant,
/// answered post-commit cold — no image build (cache-state inspection).
#[test]
fn pointwise_key_point_lookup_uses_key_probe_and_is_image_free() {
    let dir = TempDir::new("prepared-key_probe-pointwise");
    let schema = booking_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_bookings(
        &env,
        &schema,
        &[(1, (5, 10), 100), (1, (20, 30), 200), (2, (5, 10), 300)],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let query = booking_query(Term::Literal(Value::IntervalU64(
        bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
    )));
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(matches!(
        prepared.program.rules(),
        [PreparedRule::KeyProbe(_)]
    ));

    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(100));
    #[cfg(feature = "trace")]
    assert_eq!(
        cache.resident(),
        (0, 0),
        "post-commit cold: the key-probe path builds no image"
    );

    // The classification is observable through profile stats, and the
    // 16-byte determinant is exact: a one-off interval misses.
    let (answers, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert_eq!(answers.len(), 1);
    assert_eq!(
        stats.rules[0].key_probe,
        Some(crate::api::stats::KeyProbeStats { hit: true })
    );
    let near = booking_query(Term::Literal(Value::IntervalU64(
        bumbledb_theory::Interval::<u64>::new(5, 11).expect("nonempty interval"),
    )));
    let mut near = prepare(&txn, &cache, &schema, &near).expect("prepare");
    let (answers, stats) = near.profile(&txn, &cache, &[]).expect("profile");
    assert_eq!(answers.len(), 0);
    assert_eq!(
        stats.rules[0].key_probe,
        Some(crate::api::stats::KeyProbeStats { hit: false })
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
    let dir = TempDir::new("prepared-key_probe-membership");
    let schema = booking_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_bookings(
        &env,
        &schema,
        &[(1, (5, 10), 100), (1, (20, 30), 200), (2, (5, 10), 300)],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    // span ∋ 7 — a U64 *point* literal on the interval field is a
    // membership binding (validation's typing, by the literal's shape),
    // not a key cover.
    let query = booking_query(Term::Literal(Value::U64(7)));
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        matches!(prepared.program.rules(), [PreparedRule::FreeJoin(_)]),
        "membership binding is not a key cover"
    );

    let (answers, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert!(
        stats.rules[0].key_probe.is_none(),
        "the scan+filter path, not the key_probe"
    );
    assert!(!stats.rules[0].nodes.is_empty());
    assert_eq!(answers.len(), 1);
    assert_eq!(answers.get(0, 0), AnswerValue::U64(100));

    // Correct across points: 25 hits the other booking, 15 hits none.
    let query = booking_query(Term::Literal(Value::U64(25)));
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(200));
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
    let dir = TempDir::new("prepared-key_probe-m-interval");
    // Stay(owner u64, span interval<u64>), no statements — no keys.
    let schema = SchemaDescriptor {
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
    .expect("valid fixture");
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let mut bytes = Vec::new();
    encode_fact(
        &[
            ValueRef::U64(2),
            ValueRef::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
            ),
        ],
        schema.relation(RelationId(0)).layout(),
        &mut bytes,
    );
    delta.insert(&view, RelationId(0), &bytes).expect("insert");
    drop(view);
    commit(delta, &env).expect("commit");

    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    // Q(count()) :- Stay(owner = 2, span = [5, 10)) — the existence shape.
    let count_stay = |span: (u64, u64)| {
        Query::single(Rule {
            finds: vec![FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            }],
            atoms: vec![Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(0), Term::Literal(Value::U64(2))),
                    (
                        FieldId(1),
                        Term::Literal(Value::IntervalU64(
                            bumbledb_theory::Interval::<u64>::new(span.0, span.1)
                                .expect("nonempty interval"),
                        )),
                    ),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        })
    };
    let mut prepared = prepare(&txn, &cache, &schema, &count_stay((5, 10))).expect("prepare");
    assert!(matches!(
        prepared.program.rules(),
        [PreparedRule::KeyProbe(_)]
    ));
    let (_, report) = prepared.introspect(&txn, &cache, &[]).expect("introspect");
    assert!(report.contains("full-fact membership probe"), "{report}");

    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(1));
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
    let dir = TempDir::new("prepared-key_probe-intern-miss");
    // Doc(name str, val u64) with the declared key `Doc(name) -> Doc`.
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
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

    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    // Q(val) :- Doc(name = ?0, val).
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(matches!(
        prepared.program.rules(),
        [PreparedRule::KeyProbe(_)]
    ));

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
    assert_eq!(out.get(0, 0), AnswerValue::U64(7));
}

/// A hand-corrupted fixed-width start through the key-probe path is a
/// CORRUPTION conviction — never a panic, never a classification. Both
/// corrupt shapes convict: the at-bound start (`start + w = MAX_END`,
/// whose derived end is the ray sentinel, unconstructible in the fixed
/// family) and the overflowing start. Same conviction, same decoder as
/// the image lane (`encoding::decode_fixed_interval_start`).
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one fixture, two corrupt shapes: the schema, the healthy hit, and both convictions read as one story"
)]
fn a_corrupt_fixed_width_start_through_the_key_probe_is_corruption_not_a_panic() {
    use crate::error::CorruptionError;
    use crate::storage::keys::{self, KeyBuf, MAX_KEY};
    use crate::storage::read;

    // Slot(room u64, span interval<u64, 5>, label u64) keyed by room.
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Slot".into(),
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
                        width: Some(5),
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
            projection: Box::new([FieldId(0)]),
        }],
    }
    .validate()
    .expect("valid fixture");
    let dir = TempDir::new("prepared-key_probe-fixed-corrupt");
    let env = Environment::create(dir.path(), &schema).expect("create");
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(1),
                ValueRef::FixedIntervalU64(
                    bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
                ),
                ValueRef::U64(100),
            ],
            schema.relation(RelationId(0)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
        drop(view);
        commit(delta, &env).expect("commit");
    }

    // Q(span, label) :- Slot(room = 1, span, label) — the key-probe fast lane.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(1))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let cache = ImageCache::new(&schema);
    {
        // Sanity: the healthy fact answers through the fast lane.
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
        assert!(matches!(
            prepared.program.rules(),
            [PreparedRule::KeyProbe(_)]
        ));
        let out = prepared.execute_collect(&txn, &cache, &[]).expect("hit");
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(0, 1), AnswerValue::U64(100));
    }

    // Hand-corrupt the stored F value's span start (one stored word) —
    // the `U` determinant key is untouched, so the probe still HITS and
    // the fetched fact carries the corrupt start. Both corrupt shapes
    // must convict as Corruption.
    let layout = schema.relation(RelationId(0)).layout();
    let offset = layout.field_offset(1);
    let victim = {
        let txn = env.read_txn().expect("txn");
        read::scan(&txn, &schema, RelationId(0))
            .expect("scan")
            .map(|e| e.expect("ok").0)
            .next()
            .expect("nonempty")
    };
    let healthy = {
        let txn = env.read_txn().expect("txn");
        read::fetch(&txn, &schema, RelationId(0), victim)
            .expect("fetch")
            .to_vec()
    };
    // At-bound (`u64::MAX - 5 + 5 = u64::MAX`, the ray sentinel) and
    // past-bound (overflowing) starts.
    for corrupt_start in [u64::MAX - 5, u64::MAX] {
        let mut corrupt = healthy.clone();
        corrupt[offset..offset + 8].copy_from_slice(&corrupt_start.to_be_bytes());
        {
            let mut wtxn = env.write_txn().expect("txn");
            let mut key: KeyBuf = [0; MAX_KEY];
            let len = keys::fact_key(&mut key, RelationId(0), victim);
            env.data()
                .put(wtxn.raw_mut(), &key[..len], &corrupt)
                .expect("put");
            wtxn.commit().expect("commit");
        }
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
        let err = prepared
            .execute_collect(&txn, &cache, &[])
            .expect_err("corrupt stored start convicts");
        assert!(
            matches!(
                err,
                Error::Corruption(CorruptionError::InvalidFixedIntervalStart(bytes))
                    if bytes == corrupt_start.to_be_bytes()
            ),
            "{err:?}"
        );
    }

    // The IMAGE lane's twin: the same corrupt bytes driven through the
    // Free Join path (no key coverage — the image build decodes every
    // stored fact). Same decoder, same conviction, different lane —
    // this pins the routing, since the shared decoder's boundary
    // behavior is already unit-pinned in `encoding/tests.rs`.
    let scan = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
                (FieldId(2), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &scan).expect("prepare");
    assert!(
        !matches!(prepared.program.rules(), [PreparedRule::KeyProbe(_)]),
        "the all-vars scan must not take the key-probe lane"
    );
    let err = prepared
        .execute_collect(&txn, &cache, &[])
        .expect_err("the image build convicts the corrupt start");
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::InvalidFixedIntervalStart(_))
        ),
        "{err:?}"
    );
}
