//! F3 bounded-execution regressions (audit-core #2/#4/#5): the bounded
//! restart fires from JOIN growth, interior stage sinks spill under
//! genuine pressure and agree bit-exactly with the resident run, and
//! result construction charges as it grows — a tiny result budget refuses
//! before whole-set materialization and a beyond-allowance result streams
//! through the scratch backing. Gate anchors: Q-BUDGET, Q-DISK, Q-ATOMIC,
//! QRY-002/003, chapter 12 §5/§7.
use super::*;
use crate::ir::FoldOp;

use bumbledb_theory::schema::ValueType as TheoryValueType;

const METRIC: RelationId = RelationId(0);

/// A text-free relation so the resident image slab charge is exact and
/// the fallback interns nothing.
fn metric_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Metric".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: TheoryValueType::U64,
                },
                FieldDescriptor {
                    name: "bucket".into(),
                    value_type: TheoryValueType::U64,
                },
                FieldDescriptor {
                    name: "amount".into(),
                    value_type: TheoryValueType::I64,
                },
            ],
        }],
        statements: vec![],
    }
}

fn metric_store(name: &'static str, rows: u64) -> StoreFix {
    let fix = StoreFix::store(name, metric_descriptor());
    let facts: Vec<Vec<Value>> = (0..rows)
        .map(|i| {
            vec![
                Value::U64(i),
                Value::U64(i % 3),
                Value::I64(i.cast_signed() - 64),
            ]
        })
        .collect();
    fix.insert_dyn(METRIC, &facts);
    fix
}

fn bounded_policy(working_bytes: u64, result_bytes: u64) -> crate::work::ExecutionPolicy {
    crate::work::ExecutionPolicy {
        input_bytes: u64::MAX,
        working_bytes,
        scratch_bytes: u64::MAX,
        result_bytes,
        rows: u64::MAX,
        work_units: u64::MAX,
        timeout: std::time::Duration::from_secs(3600),
    }
}

/// The two-atom self-join on `id`: the sibling occurrence forces a level
/// map over every id key, so the executor's COLT pools dwarf the (smaller,
/// transient) image slab charge — the working refusal comes from JOIN
/// growth, not the build.
fn self_join() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(METRIC),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(METRIC),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn bucket_amounts(answers: &Answers) -> Vec<(u64, i64)> {
    let mut rows: Vec<(u64, i64)> = (0..answers.len())
        .map(|row| {
            let (AnswerValue::U64(bucket), AnswerValue::I64(amount)) =
                (answers.get(row, 0), answers.get(row, 1))
            else {
                panic!("typed metric answers")
            };
            (bucket, amount)
        })
        .collect();
    rows.sort_unstable();
    rows
}

/// Audit-core #2 (end to end): a working budget above the transient image
/// slab but below the join's COLT footprint refuses DURING the join, and
/// the one bounded restart through the cursor fallback delivers the exact
/// resident answers on the same pinned snapshot.
#[test]
fn a_tiny_working_budget_restarts_once_from_join_growth() {
    // 640 rows: slab = 640 × 3 words × 8 B = 15 KiB (transient, fits);
    // the forced id map alone owns ≥ 32 KiB of pool capacity (crosses).
    let fix = metric_store("budget-join-restart", 640);
    let query = self_join();

    let mut resident = fix.prepare(&query).expect("prepare");
    let expected = bucket_amounts(
        &fix.execute(&mut resident, &[] as &[BindValue])
            .expect("resident"),
    );
    assert_eq!(expected.len(), 640, "one (bucket, amount) pair per id");

    let mut restarted = fix.prepare(&query).expect("prepare");
    fix.db
        .read(|instance| {
            let work = bounded_policy(24 << 10, u64::MAX)
                .start()
                .expect("bounded ledger");
            let source =
                crate::api::prepared::source::QuerySource::store(instance.snapshot(), &work);
            let mut out = Answers::new();
            restarted.execute_source(&source, &[] as &[BindValue], &mut out)?;
            assert_eq!(
                bucket_amounts(&out),
                expected,
                "the restarted path is the query"
            );
            assert_eq!(
                work.used(crate::work::Resource::WorkingBytes),
                0,
                "every growth reservation was refunded"
            );
            Ok(())
        })
        .expect("restarted execute");
}

fn interior_amounts(answers: &Answers) -> Vec<(u64, i64)> {
    bucket_amounts(answers)
}

/// Audit-core #4: interior stage sinks now run under the per-execution
/// allowance — under forced pressure (zero RAM allowance) the stage's
/// dedup state continues in scratch and the sealed stage agrees bit-exactly
/// with the resident run; small stages under the default allowance never
/// spill (the threshold governs).
#[test]
fn a_projection_interior_under_pressure_spills_and_agrees_bit_exactly() {
    let rows: &[(u64, u64, &str, i64)] = &[
        (1, 3, "a", 10),
        (2, 3, "b", 10),
        (3, 7, "c", 25),
        (4, 7, "d", 25),
        (5, 9, "e", -5),
        (6, 9, "f", 40),
        (7, 9, "g", 40),
    ];
    let fix = postings(rows);
    // Interior: distinct (account, amount) pairs; main reads the stage.
    let query = Query {
        interiors: vec![Interior {
            rules: vec![
                crate::ir::ProjectionRule {
                    finds: vec![VarId(0), VarId(1)],
                    atoms: vec![Atom {
                        source: AtomSource::Edb(POSTING),
                        bindings: vec![
                            (FieldId(1), Term::Var(VarId(0))),
                            (FieldId(3), Term::Var(VarId(1))),
                        ],
                    }],
                    negated: vec![],
                    conditions: vec![],
                }
                .to_rule(),
            ],
        }],
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        }],
        rec: None,
    };

    let mut resident = fix.prepare(&query).expect("prepare");
    let expected = interior_amounts(
        &fix.execute(&mut resident, &[] as &[BindValue])
            .expect("resident"),
    );
    assert_eq!(expected.len(), 4, "distinct (account, amount) pairs");

    let mut spilled = fix.prepare(&query).expect("prepare");
    spilled.set_sink_ram(0);
    let got = interior_amounts(
        &fix.execute(&mut spilled, &[] as &[BindValue])
            .expect("spilled"),
    );
    assert_eq!(got, expected, "the spilled interior stage is the stage");

    // Success → success reuse on the same spilled plan (clean reset).
    let again = interior_amounts(
        &fix.execute(&mut spilled, &[] as &[BindValue])
            .expect("re-execute"),
    );
    assert_eq!(again, expected);
}

/// Audit-core #4 (aggregate stage half): an aggregate interior's dedup and
/// group state judged against a zero allowance continues in scratch and
/// finalizes into the identical sealed stage.
#[test]
fn an_aggregate_interior_under_pressure_spills_and_agrees_bit_exactly() {
    let rows: &[(u64, u64, &str, i64)] = &[
        (1, 3, "a", 10),
        (2, 3, "b", 25),
        (3, 7, "c", 25),
        (4, 7, "d", 40),
        (5, 9, "e", -5),
        (6, 9, "f", 40),
    ];
    let fix = postings(rows);
    // Interior: per-account exact Sum of amounts; main reads the stage.
    let query = Query {
        interiors: vec![Interior {
            rules: vec![Rule {
                finds: vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: FoldOp::Sum,
                        over: VarId(1),
                    },
                ],
                atoms: vec![Atom {
                    source: AtomSource::Edb(POSTING),
                    bindings: vec![
                        (FieldId(1), Term::Var(VarId(0))),
                        (FieldId(3), Term::Var(VarId(1))),
                    ],
                }],
                negated: vec![],
                conditions: vec![],
            }],
        }],
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        }],
        rec: None,
    };

    let mut resident = fix.prepare(&query).expect("prepare");
    let expected = interior_amounts(
        &fix.execute(&mut resident, &[] as &[BindValue])
            .expect("resident"),
    );
    assert_eq!(
        expected,
        vec![(3, 35), (7, 65), (9, 35)],
        "per-account sums"
    );

    let mut spilled = fix.prepare(&query).expect("prepare");
    spilled.set_sink_ram(0);
    let got = interior_amounts(
        &fix.execute(&mut spilled, &[] as &[BindValue])
            .expect("spilled"),
    );
    assert_eq!(got, expected, "the spilled aggregate stage is the stage");
}

/// Audit-core #5: result bytes charge DURING construction — a tiny result
/// budget refuses while the set is still being built (never after a full
/// RAM materialization), the refusal leaves no partial answer (Q-ATOMIC),
/// and a beyond-allowance result streams into the scratch backing as rows
/// land, sealing bit-exactly.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end charged-construction scenario: refusal + streaming"
)]
fn a_result_budget_refuses_before_whole_set_materialization() {
    let fix = metric_store("budget-result-charge", 4096);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1)), FindTerm::Var(VarId(2))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(METRIC),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
                (FieldId(2), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });

    let mut resident = fix.prepare(&query).expect("prepare");
    let expected = bucket_amounts(
        &fix.execute(&mut resident, &[] as &[BindValue])
            .expect("resident"),
    );
    assert_eq!(expected.len(), 4096);

    // (a) A 2 KiB result budget: the bounded-quantum charge inside
    // finalize refuses long before 4096 rows exist; the construction path
    // itself errors (never a post-hoc seal refusal over an already-built
    // set) and the carrier holds nothing.
    let mut refused = fix.prepare(&query).expect("prepare");
    fix.db
        .read(|instance| {
            let work = bounded_policy(u64::MAX, 2 << 10)
                .start()
                .expect("bounded ledger");
            let source =
                crate::api::prepared::source::QuerySource::store(instance.snapshot(), &work);
            let mut charge = crate::api::prepared::result::ResultCharge::new(
                &work,
                crate::api::prepared::result::RESULT_RAM_BYTES,
            );
            let mut out = Answers::new();
            let result =
                refused.execute_source_charged(&source, &[] as &[BindValue], &mut out, Some(&mut charge));
            assert!(
                matches!(
                    &result,
                    Err(Error::Store(store)) if matches!(
                        &**store,
                        crate::storage::store::StoreError::Work(
                            crate::work::WorkError::Exhausted {
                                resource: crate::work::Resource::ResultBytes,
                                ..
                            }
                        )
                    )
                ),
                "typed result-byte exhaustion during construction, got {result:?}"
            );
            assert_eq!(out.len(), 0, "no partial answer survives (Q-ATOMIC)");
            assert!(
                !charge.spilled(),
                "the refusal came from the budget, not the allowance"
            );
            Ok(())
        })
        .expect("refused execute");

    // (b) A 1 KiB RAM allowance under an unbounded budget: rows route into
    // the scratch backing DURING construction (spilled before any seal),
    // and the sealed beyond-allowance result is the exact answer set.
    let mut streamed = fix.prepare(&query).expect("prepare");
    let work = bounded_policy(u64::MAX, u64::MAX)
        .start()
        .expect("ledger");
    let sealed = fix
        .db
        .read(|instance| {
            let source =
                crate::api::prepared::source::QuerySource::store(instance.snapshot(), &work);
            let mut charge = crate::api::prepared::result::ResultCharge::new(&work, 1 << 10);
            let mut out = Answers::new();
            streamed.execute_source_charged(
                &source,
                &[] as &[BindValue],
                &mut out,
                Some(&mut charge),
            )?;
            assert!(
                charge.spilled(),
                "past the allowance the construction streams into scratch"
            );
            assert!(
                out.is_empty(),
                "streamed rows never accumulate in the RAM carrier"
            );
            let identity = crate::api::prepared::result::ResultIdentity {
                source: crate::api::prepared::source::PinnedSource::Store(
                    instance.snapshot().identity(),
                ),
                generation: Some(instance.snapshot().generation()),
            };
            charge.seal(out, identity)
        })
        .expect("streamed execute");
    assert_eq!(sealed.len(), 4096, "the complete set sealed");
    assert!(
        sealed.byte_len() > 0,
        "the sealed rows hold their result-byte charge"
    );
    let mut sealed = sealed;
    let collected = sealed.collect(u64::MAX).expect("collect");
    assert_eq!(
        bucket_amounts(&collected),
        expected,
        "the streamed backing is the answer set, bit-exactly"
    );
}
