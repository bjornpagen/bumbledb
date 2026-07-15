//! The `docs/architecture/70-api.md` integration tests: the usage shapes
//! end to end through the public surface — create → write{alloc+insert} →
//! read{point lookup, join, aggregate} → mutate via delete+insert → read
//! again; the write-closure abort contracts; the threading contract; the
//! commit-time statement judgments with their rendered diagnostics; and
//! the export → `bulk_load` ETL round trip.

use bumbledb::ir::{AggOp, Atom, FindTerm, ParamId, Query, Rule, Term, Value, VarId};
use bumbledb::schema::FieldId;
use bumbledb::{AnswerValue, Answers, BindValue, Db, Direction, Fact, StatementId, Theory};

mod common;

/// The validated ledger schema, for diagnostics rendering
/// (`display_with`) — the engine itself takes [`Ledger`].
fn ledger_schema() -> bumbledb::Schema {
    Ledger
        .descriptor()
        .validate()
        .expect("the test schema is valid")
}

bumbledb::schema! {
    pub Ledger;

    relation Holder {
        id: u64 as HolderId, fresh,
        name: str,
    }
    relation Account {
        id: u64 as AccountId, fresh,
        holder: u64 as HolderId,
        balance: i64,
    }

    Account(holder) <= Holder(id);
}

/// Q(name, balance) :- Account(holder = h, balance), Holder(id = h, name).
fn join_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(Account::RELATION),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(Holder::RELATION),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

/// Q(name, Sum(balance)) :- Account(holder = h, balance), Holder(id = h,
/// name).
fn aggregate_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(Account::RELATION),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(Holder::RELATION),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

/// Q(balance) :- Account(id = ?0, balance) — the point-lookup (key probe) shape.
fn point_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: bumbledb::AtomSource::Edb(Account::RELATION),
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// Collects a two-column (String, I64) result buffer into a sorted vec —
/// results are sets; the host sorts.
fn name_amount_answers(out: &Answers) -> Vec<(String, i64)> {
    let mut answers: Vec<(String, i64)> = (0..out.len())
        .map(|answer| {
            let AnswerValue::String(name) = out.get(answer, 0) else {
                panic!("column 0 is a string");
            };
            let AnswerValue::I64(amount) = out.get(answer, 1) else {
                panic!("column 1 is an i64");
            };
            (name.to_owned(), amount)
        })
        .collect();
    answers.sort();
    answers
}

#[test]
fn usage_shapes_end_to_end() {
    let dir = common::TempDir::new("api-usage");
    let db = Db::create(dir.path(), Ledger).expect("create");

    // Write: fresh minting + typed inserts.
    let accounts = db
        .write(|tx| {
            let alice: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id: alice,
                name: "alice",
            })?;
            let bob: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id: bob,
                name: "bob",
            })?;
            let mut accounts = Vec::new();
            for (holder, balance) in [(alice, 100), (alice, -25), (bob, 40)] {
                let id: AccountId = tx.alloc()?;
                tx.insert(&Account {
                    id,
                    holder,
                    balance,
                })?;
                accounts.push(Account {
                    id,
                    holder,
                    balance,
                });
            }
            Ok(accounts)
        })
        .expect("write");

    // Read: point lookup (key probe), join, aggregate.
    let mut point = db.prepare(&point_query()).expect("prepare point");
    let mut join = db.prepare(&join_query()).expect("prepare join");
    let mut aggregate = db.prepare(&aggregate_query()).expect("prepare agg");
    db.read(|snap| {
        let answers = snap.execute_collect(&mut point, &[BindValue::U64(accounts[2].id.0)])?;
        assert_eq!(answers.len(), 1);
        assert_eq!(answers.get(0, 0), AnswerValue::I64(40));

        let answers = snap.execute_collect(&mut join, &[])?;
        assert_eq!(
            name_amount_answers(&answers),
            vec![
                ("alice".to_owned(), -25),
                ("alice".to_owned(), 100),
                ("bob".to_owned(), 40),
            ]
        );

        let answers = snap.execute_collect(&mut aggregate, &[])?;
        assert_eq!(
            name_amount_answers(&answers),
            vec![("alice".to_owned(), 75), ("bob".to_owned(), 40)]
        );
        Ok(())
    })
    .expect("read");

    // Mutate: delete(old) + insert(new) — here in the *other* order, which
    // is equally blessed (the delta is set arithmetic).
    let old = accounts[0].clone();
    db.write(|tx| {
        tx.insert(&Account {
            balance: 90,
            ..old.clone()
        })?;
        tx.delete(&old)?;
        Ok(())
    })
    .expect("mutate");

    db.read(|snap| {
        let answers = snap.execute_collect(&mut join, &[])?;
        assert_eq!(
            name_amount_answers(&answers),
            vec![
                ("alice".to_owned(), -25),
                ("alice".to_owned(), 90),
                ("bob".to_owned(), 40),
            ]
        );
        let (answers, report) = snap.introspect(&mut join, &[])?;
        assert_eq!(answers.len(), 3);
        assert!(!report.is_empty(), "introspect renders a report");
        Ok(())
    })
    .expect("read after mutate");
}

#[test]
fn aborted_writes_leave_prior_state_intact() {
    let dir = common::TempDir::new("api-abort");
    let db = Db::create(dir.path(), Ledger).expect("create");
    db.write(|tx| {
        let id: HolderId = tx.alloc()?;
        tx.insert(&Holder { id, name: "keep" })
    })
    .expect("seed");

    // A panicking closure: the delta dies in the unwind, LMDB untouched.
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: bumbledb::Result<()> = db.write(|tx| {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id,
                name: "doomed-by-panic",
            })?;
            panic!("boom");
        });
    }));
    assert!(panicked.is_err());

    // An `Err` closure aborts the same way.
    let failed: bumbledb::Result<()> = db.write(|tx| {
        let id: HolderId = tx.alloc()?;
        tx.insert(&Holder {
            id,
            name: "doomed-by-error",
        })?;
        Err(bumbledb::Error::Overflow(
            bumbledb::OverflowKind::Aggregate { find: 0 },
        ))
    });
    assert!(failed.is_err());

    // The writer mutex is released and prior state intact.
    db.write(|tx| {
        let id: HolderId = tx.alloc()?;
        tx.insert(&Holder { id, name: "after" })
    })
    .expect("mutex usable after a panic");

    let names = db
        .read(|snap| {
            let mut names = Vec::new();
            for fact in snap.scan(Holder::RELATION)? {
                let fact = fact?;
                let Value::String(raw) = &fact[1] else {
                    panic!("field 1 is the name");
                };
                names.push(String::from_utf8(raw.to_vec()).expect("utf-8"));
            }
            names.sort();
            Ok(names)
        })
        .expect("scan");
    assert_eq!(names, vec!["after".to_owned(), "keep".to_owned()]);
}

#[test]
fn concurrent_readers_while_writing() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Db<Ledger>>();

    let dir = common::TempDir::new("api-threads");
    let db = Db::create(dir.path(), Ledger).expect("create");
    // Seed one pair so readers always see data.
    db.write(|tx| {
        let holder: HolderId = tx.alloc()?;
        tx.insert(&Holder {
            id: holder,
            name: "seed",
        })?;
        let id: AccountId = tx.alloc()?;
        tx.insert(&Account {
            id,
            holder,
            balance: 1,
        })
    })
    .expect("seed");

    // The writer commits (Holder, Account) pairs; every reader snapshot
    // must observe them atomically: equal counts, always.
    std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            for round in 0..20 {
                db.write(|tx| {
                    let holder: HolderId = tx.alloc()?;
                    tx.insert(&Holder {
                        id: holder,
                        name: &format!("holder-{round}"),
                    })?;
                    let id: AccountId = tx.alloc()?;
                    tx.insert(&Account {
                        id,
                        holder,
                        balance: round,
                    })
                })
                .expect("paired write");
            }
        });
        for _ in 0..2 {
            scope.spawn(|| {
                for _ in 0..50 {
                    db.read(|snap| {
                        let holders = snap.scan(Holder::RELATION)?.count();
                        let accounts = snap.scan(Account::RELATION)?.count();
                        assert_eq!(
                            holders, accounts,
                            "a snapshot saw a torn pair: {holders} holders, {accounts} accounts"
                        );
                        Ok(())
                    })
                    .expect("consistent read");
                }
            });
        }
        writer.join().expect("writer thread");
    });
}

#[test]
fn export_scan_bulk_loads_into_a_fresh_database() {
    let dir_old = common::TempDir::new("api-etl-old");
    let dir_new = common::TempDir::new("api-etl-new");
    let old = Db::create(dir_old.path(), Ledger).expect("create old");

    let max_holder = old
        .write(|tx| {
            let mut max = 0;
            for (name, balance) in [("alice", 100i64), ("bob", -7), ("carol", 40)] {
                let holder: HolderId = tx.alloc()?;
                tx.insert(&Holder { id: holder, name })?;
                let id: AccountId = tx.alloc()?;
                tx.insert(&Account {
                    id,
                    holder,
                    balance,
                })?;
                max = max.max(holder.0);
            }
            Ok(max)
        })
        .expect("seed");

    // Export: full-relation scans in row_id order, decoded dynamic facts.
    let (holders, accounts) = old
        .read(|snap| {
            let holders: Vec<Vec<Value>> =
                snap.scan(Holder::RELATION)?.collect::<Result<_, _>>()?;
            let accounts: Vec<Vec<Value>> =
                snap.scan(Account::RELATION)?.collect::<Result<_, _>>()?;
            Ok((holders, accounts))
        })
        .expect("export");

    // Import: containment targets first; explicit fresh values preserve
    // identity.
    let new = Db::create(dir_new.path(), Ledger).expect("create new");
    let loaded = new
        .bulk_load(Holder::RELATION, holders)
        .expect("load holders");
    assert_eq!(loaded, 3);
    let loaded = new
        .bulk_load(Account::RELATION, accounts)
        .expect("load accounts");
    assert_eq!(loaded, 3);

    // Identity: both databases answer the join identically.
    let mut join_old = old.prepare(&join_query()).expect("prepare");
    let answers_old = old
        .read(|snap| snap.execute_collect(&mut join_old, &[]))
        .expect("query old");
    let mut join_new = new.prepare(&join_query()).expect("prepare");
    let answers_new = new
        .read(|snap| snap.execute_collect(&mut join_new, &[]))
        .expect("query new");
    assert_eq!(
        name_amount_answers(&answers_old),
        name_amount_answers(&answers_new)
    );

    // The fresh high-water advanced past the explicit imports.
    new.write(|tx| {
        let next: HolderId = tx.alloc()?;
        assert!(
            next.0 > max_holder,
            "minted {} at or below the imported high water {max_holder}",
            next.0
        );
        Ok(())
    })
    .expect("mint after import");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] // one protocol: the three rejection shapes through one store
fn statement_violations_surface_from_commit_through_the_public_api() {
    let dir = common::TempDir::new("api-violations");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let holder = db
        .write(|tx| {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder { id, name: "alice" })?;
            Ok(id)
        })
        .expect("seed");

    // Functionality violation: two live accounts claiming one fresh id.
    // The error carries the statement id and the offending fact bytes,
    // and the WHOLE transaction aborts (the good insert too).
    let err = db
        .write(|tx| {
            tx.insert(&Account {
                id: AccountId(7),
                holder,
                balance: 1,
            })?;
            tx.insert(&Account {
                id: AccountId(7),
                holder,
                balance: 2,
            })?;
            Ok(())
        })
        .unwrap_err();
    let bumbledb::Error::CommitRejected { ref violations } = err else {
        panic!("expected CommitRejected, got {err}");
    };
    let [
        bumbledb::Violation::Functionality {
            statement, fact, ..
        },
    ] = violations.as_slice()
    else {
        panic!("expected one key citation, got {violations:?}");
    };
    // Materialized order: Holder.id's fresh auto-key, Account.id's
    // fresh auto-key, then the declared containment.
    assert_eq!(*statement, StatementId(1));
    assert!(!fact.is_empty());
    // The rendered diagnostic cites the statement in the algebra.
    let rendered = format!("{}", err.display_with(&ledger_schema()));
    assert!(rendered.contains("Account(id) -> Account"), "{rendered}");
    let count = db
        .read(|snap| Ok(snap.scan_facts::<Account>()?.count()))
        .expect("scan");
    assert_eq!(count, 0, "the aborted transaction left nothing");

    // Containment, source side: an inserted account whose holder does
    // not exist. `Display` through the schema cites the statement
    // rendered back in the algebra, and the judgment direction.
    let err = db
        .write(|tx| {
            tx.insert(&Account {
                id: AccountId(1),
                holder: HolderId(404),
                balance: 5,
            })
        })
        .unwrap_err();
    assert!(matches!(
        &err,
        bumbledb::Error::CommitRejected { violations } if matches!(
            violations.as_slice(),
            [bumbledb::Violation::Containment {
                statement: StatementId(2),
                direction: Direction::SourceUnsatisfied,
                ..
            }]
        )
    ));
    let rendered = format!("{}", err.display_with(&ledger_schema()));
    assert!(
        rendered.contains("Account(holder) <= Holder(id)"),
        "{rendered}"
    );
    assert!(rendered.contains("source"), "{rendered}");

    // Containment, target side: deleting a holder a surviving account
    // still requires — the requiring source is named by its fact.
    db.write(|tx| {
        tx.insert(&Account {
            id: AccountId(1),
            holder,
            balance: 5,
        })
    })
    .expect("reference the holder");
    let err = db
        .write(|tx| {
            tx.delete(&Holder {
                id: holder,
                name: "alice",
            })
        })
        .unwrap_err();
    let bumbledb::Error::CommitRejected { ref violations } = err else {
        panic!("expected CommitRejected, got {err}");
    };
    let [
        bumbledb::Violation::Containment {
            direction, fact, ..
        },
    ] = violations.as_slice()
    else {
        panic!("expected one containment citation, got {violations:?}");
    };
    assert_eq!(*direction, Direction::TargetRequired);
    assert!(
        !fact.is_empty(),
        "the requiring source is named by its fact"
    );
    let rendered = format!("{}", err.display_with(&ledger_schema()));
    assert!(
        rendered.contains("Account(holder) <= Holder(id)"),
        "{rendered}"
    );
    assert!(rendered.contains("target"), "{rendered}");
}

#[test]
fn open_mismatches_and_snapshot_usability() {
    let dir = common::TempDir::new("api-open-mismatch");
    drop(Db::create(dir.path(), Ledger).expect("create"));

    // Db-level mismatch: a different schema refuses to open.
    let other = bumbledb::schema::SchemaDescriptor {
        relations: vec![bumbledb::schema::RelationDescriptor {
            extension: None,
            name: "Other".into(),
            fields: vec![bumbledb::schema::FieldDescriptor {
                name: "x".into(),
                value_type: bumbledb::schema::ValueType::U64,
                generation: bumbledb::schema::Generation::None,
            }],
        }],
        statements: vec![],
    };
    let Err(err) = Db::open(dir.path(), other).map(|_| ()) else {
        panic!("a different schema must refuse to open");
    };
    assert!(matches!(err, bumbledb::Error::SchemaMismatch { .. }));

    // Create-over-existing refuses at the Db level too.
    let Err(err) = Db::create(dir.path(), Ledger).map(|_| ()) else {
        panic!("create over an existing environment must refuse");
    };
    assert!(matches!(err, bumbledb::Error::AlreadyInitialized));

    // A failed execute leaves the snapshot usable, and the caller-buffer
    // path works through the public surface.
    let db = Db::open(dir.path(), Ledger).expect("open");
    db.write(|tx| {
        let id: HolderId = tx.alloc()?;
        tx.insert(&Holder { id, name: "bo" })
    })
    .expect("seed");
    let mut join = db.prepare(&join_query()).expect("prepare");
    db.read(|snap| {
        let mut out = Answers::new();
        // Wrong param count: a typed error...
        let err = snap
            .execute(&mut join, &[BindValue::U64(1)], &mut out)
            .unwrap_err();
        assert!(matches!(err, bumbledb::Error::ParamCountMismatch { .. }));
        // ...and the same snapshot executes fine afterwards.
        snap.execute(&mut join, &[], &mut out)?;
        assert_eq!(out.len(), 0, "no accounts yet");
        Ok(())
    })
    .expect("snapshot stays usable");
}

#[test]
fn pinned_snapshot_reads_its_generation_across_later_commits() {
    let dir = common::TempDir::new("api-pinned");
    let db = Db::create(dir.path(), Ledger).expect("create");
    db.write(|tx| {
        let id: HolderId = tx.alloc()?;
        tx.insert(&Holder { id, name: "first" })
    })
    .expect("seed");

    let mut join = db.prepare(&join_query()).expect("prepare");
    db.read(|snap| {
        let before = snap.scan_facts::<Holder>()?.count();
        assert_eq!(before, 1);
        // Two commits land while this snapshot stays open (LMDB readers
        // never block the writer; MDB_NOTLS reader slots).
        for round in 0..2 {
            db.write(|tx| {
                let id: HolderId = tx.alloc()?;
                tx.insert(&Holder {
                    id,
                    name: &format!("later-{round}"),
                })
            })?;
        }
        // The pinned snapshot still answers at its own generation.
        assert_eq!(snap.scan_facts::<Holder>()?.count(), 1);
        let answers = snap.execute_collect(&mut join, &[])?;
        assert_eq!(answers.len(), 0);
        Ok(())
    })
    .expect("pinned read");

    // A fresh snapshot sees all three.
    let after = db
        .read(|snap| Ok(snap.scan_facts::<Holder>()?.count()))
        .expect("fresh read");
    assert_eq!(after, 3);
}

#[test]
fn bulk_load_equals_sequential_inserts_and_survives_chunks() {
    let dir_bulk = common::TempDir::new("api-bulk-a");
    let dir_seq = common::TempDir::new("api-bulk-b");
    let bulk = Db::create(dir_bulk.path(), Ledger).expect("create");
    let seq = Db::create(dir_seq.path(), Ledger).expect("create");

    // > one chunk of holders (chunk = 4096).
    let n = 4_100u64;
    let facts: Vec<Vec<Value>> = (0..n)
        .map(|i| {
            vec![
                Value::U64(i),
                Value::String(format!("h{}", i % 97).into_bytes().into()),
            ]
        })
        .collect();
    let loaded = bulk
        .bulk_load(Holder::RELATION, facts.clone())
        .expect("bulk load");
    assert_eq!(loaded, n);
    for chunk in facts.chunks(512) {
        seq.write(|tx| {
            for f in chunk {
                tx.insert_dyn(Holder::RELATION, f)?;
            }
            Ok(())
        })
        .expect("sequential insert");
    }

    // Set equality of the full export: an ETL bug is a data-loss bug.
    // (Scan order is row-id order, and row ids depend on chunk boundaries
    // — relations are sets, so the comparison sorts by the fresh id.)
    let by_id = |mut rows: Vec<Vec<Value>>| {
        rows.sort_by_key(|f| match f[0] {
            Value::U64(id) => id,
            _ => unreachable!("id column"),
        });
        rows
    };
    let a = by_id(
        bulk.read(|snap| snap.scan(Holder::RELATION)?.collect::<Result<_, _>>())
            .expect("scan bulk"),
    );
    let b = by_id(
        seq.read(|snap| snap.scan(Holder::RELATION)?.collect::<Result<_, _>>())
            .expect("scan seq"),
    );
    assert_eq!(a, b);
    assert_eq!(a.len(), usize::try_from(n).expect("64-bit"));

    // Mid-stream failure: prior chunks stay committed and the error
    // carries the committed count.
    let dir_fail = common::TempDir::new("api-bulk-fail");
    let fail = Db::create(dir_fail.path(), Ledger).expect("create");
    let mut bad = facts;
    bad[4_099] = vec![Value::U64(0)]; // arity mismatch in the second chunk
    let err = fail.bulk_load(Holder::RELATION, bad).unwrap_err();
    assert_eq!(err.committed, 4_096, "the complete first chunk persisted");
    assert!(matches!(err.error, bumbledb::Error::FactShape(_)));
    let persisted = fail
        .read(|snap| Ok(snap.scan_facts::<Holder>()?.count()))
        .expect("scan");
    assert_eq!(persisted, 4_096);
}

#[test]
fn disk_size_and_generation_report_store_state() {
    let dir = common::TempDir::new("api-disk-size");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let empty = db.disk_size().expect("size");
    assert!(empty > 0, "a fresh environment still has pages");
    assert_eq!(db.generation().expect("gen").value(), 0);

    db.write(|tx| {
        for _ in 0..10_000u64 {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id,
                name: &format!("holder-{}", id.0),
            })?;
        }
        Ok(())
    })
    .expect("bulk write");
    let grown = db.disk_size().expect("size");
    assert!(grown > empty, "10k facts grow the file: {empty} -> {grown}");
    assert_eq!(db.generation().expect("gen").value(), 1);
}

/// The magnitude-first cover choice (docs/architecture/40-execution.md), end to end: the
/// balance shape — a big relation joined to a param-selected small side
/// — must iterate the selected side (7 keys) and probe the big one,
/// never the reverse. Work is pinned by counters, not wall clock.
#[test]
fn cover_choice_iterates_the_selected_side() {
    use bumbledb::ir::{AggOp, Atom, FindTerm, ParamId, Query, Term, VarId};

    let dir = common::TempDir::new("api-cover-choice");
    let db = Db::create(dir.path(), Ledger).expect("create");
    // 500 holders (ids 0..7 share the name "target"), 20 accounts each.
    db.write(|tx| {
        let mut holders = Vec::new();
        for i in 0..500u64 {
            let id: HolderId = tx.alloc()?;
            let name = if i < 7 {
                "target".to_owned()
            } else {
                format!("h{i}")
            };
            tx.insert(&Holder { id, name: &name })?;
            holders.push(id);
        }
        for i in 0..10_000u64 {
            let id: AccountId = tx.alloc()?;
            tx.insert(&Account {
                id,
                holder: holders[usize::try_from(i % 500).expect("small")],
                balance: i64::try_from(i).expect("fits"),
            })?;
        }
        Ok(())
    })
    .expect("populate");

    // Q(h, Sum(balance)) :- Account(holder = h, balance),
    //                       Holder(id = h, name = ?0).
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![
            Atom {
                source: bumbledb::AtomSource::Edb(Account::RELATION),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: bumbledb::AtomSource::Edb(Holder::RELATION),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Param(ParamId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = db.prepare(&query).expect("prepare");
    let params = vec![BindValue::Str("target")];
    let (out, stats) = db
        .read(|snap| snap.profile(&mut prepared, &params))
        .expect("profile");
    assert_eq!(out.len(), 7, "one group per target holder");
    assert_eq!(stats.emits, 140, "20 accounts x 7 holders reach the sink");

    // The join-variable node iterates the 7-key selected side...
    let batch_entries: Vec<u64> = stats.rules[0]
        .nodes
        .iter()
        .map(|n| n.batch_entries)
        .collect();
    assert!(
        batch_entries.contains(&7),
        "the cover is the selected side: {stats:?}"
    );
    // ...and total drawn entries are O(selected), never O(relation).
    let total: u64 = batch_entries.iter().sum();
    assert_eq!(total, 147, "7 holder keys + 140 account entries: {stats:?}");
}

/// Compaction (docs/architecture/50-storage.md): a chunk-churned store copies to a
/// substantially smaller, byte-identical, fully writable sibling — and
/// never clobbers an existing destination.
#[test]
fn compaction_drops_the_freelist_and_preserves_content() {
    use bumbledb::ir::Value;

    let dir = common::TempDir::new("api-compact");
    let source_dir = dir.path().join("source");
    let db = Db::create(&source_dir, Ledger).expect("create");
    // Many small commits grow a real freelist through CoW churn.
    for round in 0..40u64 {
        db.write(|tx| {
            for i in 0..250u64 {
                let id: HolderId = tx.alloc()?;
                tx.insert(&Holder {
                    id,
                    name: &format!("h{round}-{i}"),
                })?;
            }
            Ok(())
        })
        .expect("commit");
    }
    let source_size = db.disk_size().expect("size");
    let generation = db.generation().expect("generation");
    let scan_digest = |db: &Db<Ledger>| -> Vec<Vec<Value>> {
        let mut rows: Vec<Vec<Value>> = db
            .read(|snap| snap.scan(Holder::RELATION)?.collect::<Result<_, _>>())
            .expect("scan");
        rows.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        rows
    };
    let source_rows = scan_digest(&db);

    let compact_dir = dir.path().join("compacted");
    db.compact(&compact_dir).expect("compact");
    // Never clobbers.
    let err = db.compact(&compact_dir).expect_err("must refuse");
    assert!(matches!(err, bumbledb::Error::Io(_)), "{err:?}");
    drop(db);

    let compacted = Db::open(&compact_dir, Ledger).expect("open compacted");
    let compact_size = compacted.disk_size().expect("size");
    assert!(
        compact_size * 10 <= source_size * 8,
        "compaction reclaims the churn: {compact_size} vs {source_size}"
    );
    assert_eq!(compacted.generation().expect("generation"), generation);
    assert_eq!(scan_digest(&compacted), source_rows, "byte-identical facts");

    // A first-class store: writes commit and read back.
    compacted
        .write(|tx| {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id,
                name: "post-compaction",
            })
        })
        .expect("write");
    assert_eq!(
        scan_digest(&compacted).len(),
        source_rows.len() + 1,
        "the compacted store keeps living"
    );
}

/// A prepared query executes only against snapshots of the database that
/// prepared it. Before the environment-instance check, executing A's
/// prepared query against B (same schema, same generation) returned B's
/// data through A's memo keys.
#[test]
fn a_prepared_query_refuses_a_foreign_snapshot() {
    let dir_a = common::TempDir::new("api-foreign-prepared-a");
    let dir_b = common::TempDir::new("api-foreign-prepared-b");
    let db_a = Db::create(dir_a.path(), Ledger).expect("create a");
    let db_b = Db::create(dir_b.path(), Ledger).expect("create b");
    for (db, name, balance) in [(&db_a, "alice", 10), (&db_b, "bob", 20)] {
        db.write(|tx| {
            let holder: HolderId = tx.alloc()?;
            tx.insert(&Holder { id: holder, name })?;
            let id: AccountId = tx.alloc()?;
            tx.insert(&Account {
                id,
                holder,
                balance,
            })
        })
        .expect("seed one distinct fact pair");
    }
    assert_eq!(db_a.generation().expect("gen a").value(), 1);
    assert_eq!(
        db_b.generation().expect("gen b").value(),
        1,
        "both clocks read 1"
    );

    let mut prepared = db_a.prepare(&join_query()).expect("prepare on A");
    db_a.read(|snap| {
        let out = snap.execute_collect(&mut prepared, &[])?;
        assert_eq!(name_amount_answers(&out), vec![("alice".to_owned(), 10)]);
        Ok(())
    })
    .expect("execute on the preparing db");

    // Step 4 of the audit repro: execute against B. Every execution entry
    // refuses — never B-as-A's-data.
    db_b.read(|snap| {
        let err = snap.execute_collect(&mut prepared, &[]).unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::ForeignPreparedQuery),
            "{err:?}"
        );
        let mut out = Answers::new();
        let err = snap.execute(&mut prepared, &[], &mut out).unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::ForeignPreparedQuery),
            "{err:?}"
        );
        let err = snap.introspect(&mut prepared, &[]).unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::ForeignPreparedQuery),
            "{err:?}"
        );
        let err = snap.profile(&mut prepared, &[]).unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::ForeignPreparedQuery),
            "{err:?}"
        );
        // The staleness signal checks its entry identically: pinned
        // statistics belong to the preparing environment.
        let err = prepared.staleness(snap).unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::ForeignPreparedQuery),
            "{err:?}"
        );
        Ok(())
    })
    .expect("read on b");

    // The preparing db still executes fine afterward.
    db_a.read(|snap| {
        let out = snap.execute_collect(&mut prepared, &[])?;
        assert_eq!(name_amount_answers(&out), vec![("alice".to_owned(), 10)]);
        Ok(())
    })
    .expect("A unaffected");
}

/// The advisory lock — a second live handle on the same path is
/// a loud open-time error; dropping the first releases it.
#[test]
fn a_second_handle_on_a_live_path_is_locked_out() {
    let dir = common::TempDir::new("api-env-lock");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let err = Db::open(dir.path(), Ledger).map(|_| ()).unwrap_err();
    assert!(matches!(err, bumbledb::Error::EnvironmentLocked), "{err:?}");
    let err = Db::create(dir.path(), Ledger).map(|_| ()).unwrap_err();
    assert!(
        matches!(err, bumbledb::Error::EnvironmentLocked),
        "create is locked out before it can even refuse: {err:?}"
    );
    drop(db);
    let reopened = Db::open(dir.path(), Ledger).expect("the lock died with the handle");
    drop(reopened);
}

/// `create` refuses a directory holding someone else's LMDB
/// environment (named databases, no `_meta`), while the half-created
/// bumbledb recovery case — an empty root — still proceeds.
#[test]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
fn create_refuses_a_foreign_lmdb_environment() {
    let dir = common::TempDir::new("api-env-foreign-lmdb");
    {
        // SAFETY: this test environment is opened once, in this scope.
        let env = unsafe {
            heed::EnvOpenOptions::new()
                .max_dbs(2)
                .open(dir.path())
                .expect("raw lmdb env")
        };
        let mut wtxn = env.write_txn().expect("txn");
        let db: heed::Database<heed::types::Bytes, heed::types::Bytes> = env
            .create_database(&mut wtxn, Some("someone_elses_table"))
            .expect("foreign named db");
        db.put(&mut wtxn, b"k", b"v").expect("put");
        wtxn.commit().expect("commit");
    }
    let err = Db::create(dir.path(), Ledger).map(|_| ()).unwrap_err();
    assert!(
        matches!(err, bumbledb::Error::AlreadyInitialized),
        "{err:?}"
    );

    // The recovery case: an LMDB file with an empty root (exactly what a
    // crash between directory creation and the meta commit leaves).
    let dir = common::TempDir::new("api-env-half-created");
    {
        // SAFETY: as above.
        let env = unsafe {
            heed::EnvOpenOptions::new()
                .max_dbs(2)
                .open(dir.path())
                .expect("raw lmdb env")
        };
        let wtxn = env.write_txn().expect("txn");
        wtxn.commit().expect("commit nothing");
    }
    drop(Db::create(dir.path(), Ledger).expect("an empty root is recoverable"));
}

/// `Db::write` is non-reentrant — a nested call on the same
/// thread panics with the named message instead of deadlocking forever,
/// and the write lock clears for the next (sequential) write.
#[test]
fn nested_write_panics_instead_of_deadlocking() {
    let dir = common::TempDir::new("api-nested-write");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = db.write(|_| db.write(|_| Ok(())));
    }));
    let payload = result.expect_err("must panic");
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .expect("string panic payload");
    assert!(message.contains("nested Db::write"), "{message}");

    // Sequential writes on the same thread still work: the write lock cleared.
    db.write(|tx| {
        let id: HolderId = tx.alloc()?;
        tx.insert(&Holder {
            id,
            name: "after the panic",
        })
    })
    .expect("the writer survives");
}

/// The concurrency family: prepared queries on
/// reader threads race a writer that moves two facts together every
/// commit. Every execution must observe both answers at one generation —
/// equal balances, always — never a torn mix of two generations.
#[test]
fn prepared_executions_observe_exactly_one_generation() {
    let dir = common::TempDir::new("api-gen-atomic");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let (hx, hy, ax, ay) = db
        .write(|tx| {
            let hx: HolderId = tx.alloc()?;
            tx.insert(&Holder { id: hx, name: "x" })?;
            let hy: HolderId = tx.alloc()?;
            tx.insert(&Holder { id: hy, name: "y" })?;
            let ax: AccountId = tx.alloc()?;
            tx.insert(&Account {
                id: ax,
                holder: hx,
                balance: 0,
            })?;
            let ay: AccountId = tx.alloc()?;
            tx.insert(&Account {
                id: ay,
                holder: hy,
                balance: 0,
            })?;
            Ok((hx, hy, ax, ay))
        })
        .expect("seed");

    let db = &db;
    std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            for round in 1..=40i64 {
                db.write(|tx| {
                    tx.delete(&Account {
                        id: ax,
                        holder: hx,
                        balance: round - 1,
                    })?;
                    tx.insert(&Account {
                        id: ax,
                        holder: hx,
                        balance: round,
                    })?;
                    tx.delete(&Account {
                        id: ay,
                        holder: hy,
                        balance: round - 1,
                    })?;
                    tx.insert(&Account {
                        id: ay,
                        holder: hy,
                        balance: round,
                    })
                })
                .expect("paired rewrite");
            }
        });
        for _ in 0..3 {
            scope.spawn(|| {
                let mut prepared = db.prepare(&join_query()).expect("prepare");
                let mut out = Answers::new();
                for _ in 0..80 {
                    db.read(|snap| {
                        snap.execute(&mut prepared, &[], &mut out)?;
                        let answers = name_amount_answers(&out);
                        assert_eq!(answers.len(), 2, "both facts, always: {answers:?}");
                        assert_eq!(
                            answers[0].1, answers[1].1,
                            "a torn read mixed two generations: {answers:?}"
                        );
                        Ok(())
                    })
                    .expect("consistent execution");
                }
            });
        }
        writer.join().expect("writer thread");
    });
}

/// A *successful* commit persists every fresh
/// value it issued, even when no facts changed — an id the closure
/// returned to the host is never re-issued. Both no-op shapes: the
/// empty delta (alloc, nothing else) and the nets-to-nothing delta
/// (insert then delete of the same absent fact). The generation must
/// not move for either — `Q` marks are not query-visible state.
#[test]
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "the method path does not satisfy the higher-ranked bound"
)] // HRTB: the method path does not unify
fn escaped_fresh_ids_survive_noop_commits() {
    let dir = common::TempDir::new("api-fresh-escape");
    let db = Db::create(dir.path(), Ledger).expect("create");

    // The empty-delta path.
    let a: HolderId = db.write(|tx| tx.alloc()).expect("bare alloc");
    let generation_after_a = db.generation().expect("generation");
    let b: HolderId = db
        .write(|tx| {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id,
                name: "first real holder",
            })?;
            Ok(id)
        })
        .expect("real write");
    assert!(b.0 > a.0, "escaped id {a:?} re-issued as {b:?}");

    // The nets-to-nothing path (`changed: false`, non-empty delta).
    let c: HolderId = db
        .write(|tx| {
            let id: HolderId = tx.alloc()?;
            let ghost = Holder { id, name: "ghost" };
            tx.insert(&ghost)?;
            tx.delete(&ghost)?;
            Ok(id)
        })
        .expect("nets to nothing");
    let generation_after_c = db.generation().expect("generation");
    let d: HolderId = db
        .write(|tx| {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id,
                name: "second real holder",
            })?;
            Ok(id)
        })
        .expect("real write");
    assert!(d.0 > c.0, "escaped id {c:?} re-issued as {d:?}");

    // Neither no-op moved the generation: Q marks are write-path
    // bookkeeping, not query-visible state.
    assert_eq!(
        generation_after_a.value(),
        0,
        "a bare alloc is not a state change"
    );
    assert_eq!(
        generation_after_c.value(),
        1,
        "a nets-to-nothing write is not a state change"
    );
}

/// Deleting a fact whose string was never interned is a proven
/// no-op — the fact's bytes would embed an id that was never minted —
/// and the dictionary does not grow. A later insert of that value must
/// still treat it as novel (both engine-visible effects of not minting).
#[test]
fn deleting_a_never_interned_string_is_a_mint_free_noop() {
    let dir = common::TempDir::new("api-mint-free-delete");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let holder = db
        .write(|tx| {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder { id, name: "real" })?;
            Ok(id)
        })
        .expect("seed");

    // Typed delete of a never-interned name: changed = false, and the
    // whole write is a no-op commit (generation unmoved).
    let generation = db.generation().expect("generation");
    db.write(|tx| {
        let changed = tx.delete(&Holder {
            id: holder,
            name: "never interned",
        })?;
        assert!(!changed, "a never-interned value matches no fact");
        Ok(())
    })
    .expect("typed delete");
    // Dynamic delete, same contract.
    db.write(|tx| {
        let changed = tx.delete_dyn(
            Holder::RELATION,
            &[
                Value::U64(holder.0),
                Value::String("also never interned".as_bytes().into()),
            ],
        )?;
        assert!(!changed);
        Ok(())
    })
    .expect("dynamic delete");
    assert_eq!(db.generation().expect("generation"), generation);

    // The real fact is untouched, and insert-then-delete in one
    // transaction still cancels exactly (the pending map serves the
    // delete path).
    db.write(|tx| {
        let id: HolderId = tx.alloc()?;
        let transient = Holder {
            id,
            name: "transient",
        };
        assert!(tx.insert(&transient)?);
        assert!(tx.delete(&transient)?);
        Ok(())
    })
    .expect("cancel");
    let names: Vec<String> = db
        .read(|snap| {
            snap.scan_facts::<Holder>()?
                .map(|h| h.map(|h| h.name.to_owned()))
                .collect::<bumbledb::Result<Vec<_>>>()
        })
        .expect("scan");
    assert_eq!(names, vec!["real".to_owned()]);
}

/// An out-of-range relation id at the dynamic
/// (ETL) surface is a typed `FactShape` error at every public boundary —
/// `insert_dyn`, `delete_dyn`, `bulk_load`, and `scan` — never a panic.
#[test]
fn out_of_range_relation_ids_are_typed_errors() {
    let dir = common::TempDir::new("api-unknown-relation");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let bogus = bumbledb::RelationId(999);
    let is_unknown = |err: &bumbledb::Error| {
        matches!(
            err,
            bumbledb::Error::FactShape(bumbledb::error::FactShapeError::UnknownRelation {
                relation
            }) if relation.0 == 999
        )
    };

    db.write(|tx| {
        let err = tx.insert_dyn(bogus, &[Value::U64(1)]).unwrap_err();
        assert!(is_unknown(&err), "{err:?}");
        let err = tx.delete_dyn(bogus, &[Value::U64(1)]).unwrap_err();
        assert!(is_unknown(&err), "{err:?}");
        Ok(())
    })
    .expect("write closes cleanly");

    let err = db
        .bulk_load(bogus, vec![vec![Value::U64(1)]])
        .map(|_| ())
        .unwrap_err();
    assert!(is_unknown(&err.error), "{:?}", err.error);
    assert_eq!(err.committed, 0);

    db.read(|snap| {
        let err = snap.scan(bogus).map(|_| ()).unwrap_err();
        assert!(is_unknown(&err), "{err:?}");
        Ok(())
    })
    .expect("read closes cleanly");
}

/// The plan-staleness signal (`docs/architecture/70-api.md`): prepare
/// pins per-occurrence row counts; `staleness` compares them against a
/// snapshot's live counters. Growth to ~4x reads as ratio 4 on the grown
/// occurrence (and as the max); re-preparing resets the pin; a shrunk
/// relation also reads as drift > 1 — the ratio is symmetric.
#[test]
fn staleness_reports_drift_and_reprepare_resets_it() {
    let dir = common::TempDir::new("api-staleness");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let holder = db
        .write(|tx| {
            let holder: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id: holder,
                name: "alice",
            })?;
            for balance in 0..8 {
                let id: AccountId = tx.alloc()?;
                tx.insert(&Account {
                    id,
                    holder,
                    balance,
                })?;
            }
            Ok(holder)
        })
        .expect("seed 1 holder + 8 accounts");
    let prepared = db.prepare(&join_query()).expect("prepare at N");

    // Fresh plan: both occurrences pinned, nothing drifted.
    db.read(|snap| {
        let staleness = prepared.staleness(snap)?;
        assert_eq!(staleness.per_occurrence.len(), 2);
        assert!(
            (staleness.max_ratio - 1.0).abs() < f64::EPSILON,
            "{staleness:?}"
        );
        Ok(())
    })
    .expect("fresh read");

    // Grow Account 8 → 32 (~4x); Holder stays put.
    db.write(|tx| {
        for balance in 8..32 {
            let id: AccountId = tx.alloc()?;
            tx.insert(&Account {
                id,
                holder,
                balance,
            })?;
        }
        Ok(())
    })
    .expect("grow accounts to 4N");

    db.read(|snap| {
        let staleness = prepared.staleness(snap)?;
        let account = staleness
            .per_occurrence
            .iter()
            .find(|d| d.relation == Account::RELATION)
            .expect("the Account occurrence is pinned");
        assert_eq!(account.pinned, 8);
        assert_eq!(account.live, 32);
        assert!((account.ratio - 4.0).abs() < f64::EPSILON, "{account:?}");
        let holder = staleness
            .per_occurrence
            .iter()
            .find(|d| d.relation == Holder::RELATION)
            .expect("the Holder occurrence is pinned");
        assert!((holder.ratio - 1.0).abs() < f64::EPSILON, "{holder:?}");
        assert!(
            (staleness.max_ratio - 4.0).abs() < f64::EPSILON,
            "the max is the worst occurrence: {staleness:?}"
        );
        Ok(())
    })
    .expect("drifted read");

    // Re-prepare: the pin resets to the live counts.
    let reprepared = db.prepare(&join_query()).expect("re-prepare at 4N");
    db.read(|snap| {
        let staleness = reprepared.staleness(snap)?;
        assert!(
            (staleness.max_ratio - 1.0).abs() < f64::EPSILON,
            "{staleness:?}"
        );
        Ok(())
    })
    .expect("reset read");

    // Shrink Account 32 → 8: drift reads > 1 in this direction too.
    let accounts: Vec<Account> = db
        .read(|snap| snap.scan_facts::<Account>()?.collect())
        .expect("collect accounts");
    db.write(|tx| {
        for account in accounts.iter().take(24) {
            tx.delete(account)?;
        }
        Ok(())
    })
    .expect("shrink accounts");
    db.read(|snap| {
        let staleness = reprepared.staleness(snap)?;
        let account = staleness
            .per_occurrence
            .iter()
            .find(|d| d.relation == Account::RELATION)
            .expect("the Account occurrence is pinned");
        assert_eq!(account.pinned, 32);
        assert_eq!(account.live, 8);
        assert!(account.ratio > 1.0, "shrink reads as drift: {account:?}");
        assert!((staleness.max_ratio - 4.0).abs() < f64::EPSILON);
        Ok(())
    })
    .expect("shrunk read");
}

/// The degenerate-equivalence contract (20-query-ir.md § engine recursion; the
/// theorem is `lean/Bumbledb/Exec/Fixpoint.lean: degenerate_embedding`):
/// a one-predicate, no-`Idb` `Program` prepares through the program
/// boundary (`Db::prepare_program` — the whole program roster, then the
/// output predicate's ordinary pipeline) and executes identically to
/// its `Query` form, end to end.
#[test]
fn a_degenerate_program_executes_as_its_query() {
    let dir = common::TempDir::new("api-degenerate-program");
    let db = Db::create(dir.path(), Ledger).expect("create");
    db.write(|tx| {
        for (name, balances) in [("alice", vec![100, -25]), ("bob", vec![40])] {
            let holder: HolderId = tx.alloc()?;
            tx.insert(&Holder { id: holder, name })?;
            for balance in balances {
                let id: AccountId = tx.alloc()?;
                tx.insert(&Account {
                    id,
                    holder,
                    balance,
                })?;
            }
        }
        Ok(())
    })
    .expect("write");

    let query = join_query();
    let mut as_query = db.prepare(&query).expect("prepare query");
    let mut as_program = db
        .prepare_program(&bumbledb::Program::from(query))
        .expect("prepare degenerate program");
    db.read(|snap| {
        let query_answers = snap.execute_collect(&mut as_query, &[])?;
        let program_answers = snap.execute_collect(&mut as_program, &[])?;
        assert_eq!(
            name_amount_answers(&query_answers),
            name_amount_answers(&program_answers),
            "the degenerate program IS its query"
        );
        assert_eq!(query_answers.len(), 3);
        // The byte-identity check: a no-`Idb` program takes ZERO new
        // code paths — `prepare_program` routes the degenerate form
        // through `prepare` verbatim, so the introspection reports (the
        // rendered query, the plans, the counted stats) are the same
        // bytes.
        let (_, query_report) = snap.introspect(&mut as_query, &[])?;
        let (_, program_report) = snap.introspect(&mut as_program, &[])?;
        assert_eq!(
            query_report, program_report,
            "the degenerate program's artifact is byte-identical to its query's"
        );
        Ok(())
    })
    .expect("read");
}

/// Recursion at the public surface (the deleted R1 fence's ground): a
/// roster-clean recursive program prepares and executes under the
/// fixpoint driver (`api/prepared/fixpoint.rs`), and the self-loop
/// closure `p0(x) | Account(id: x); p0(x) | p0(x)` denotes exactly the
/// base rule's set — the recursive rule re-derives, the seen-set
/// absorbs, the fixpoint closes in one growing round
/// (`lean/Bumbledb/Exec/Fixpoint.lean: program_eval_sound`).
#[test]
fn prepare_program_executes_recursion_under_the_driver() {
    use bumbledb::ir::{AtomSource, HeadTerm};
    let dir = common::TempDir::new("api-program-driver");
    let db = Db::create(dir.path(), Ledger).expect("create");
    db.write(|tx| {
        let holder: HolderId = tx.alloc()?;
        tx.insert(&Holder {
            id: holder,
            name: "alice",
        })?;
        for balance in [100, -25, 40] {
            let id: AccountId = tx.alloc()?;
            tx.insert(&Account {
                id,
                holder,
                balance,
            })?;
        }
        Ok(())
    })
    .expect("write");

    let base = Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(Account::RELATION),
            bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let recursive = Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Idb(bumbledb::PredId(0)),
            bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let program = bumbledb::Program {
        predicates: vec![bumbledb::PredicateDef {
            head: vec![HeadTerm::Var],
            rules: vec![base.clone(), recursive],
        }],
        output: bumbledb::PredId(0),
    };
    let mut recursive_prepared = db.prepare_program(&program).expect("recursion executes");
    let mut base_prepared = db.prepare(&Query::single(base)).expect("prepare base");
    db.read(|snap| {
        let closure = snap.execute_collect(&mut recursive_prepared, &[])?;
        let base_only = snap.execute_collect(&mut base_prepared, &[])?;
        let ids = |answers: &bumbledb::Answers| -> std::collections::BTreeSet<u64> {
            answers
                .answers()
                .map(|answer| match answer.get(0) {
                    bumbledb::AnswerValue::U64(id) => id,
                    other => panic!("account ids are u64, got {other:?}"),
                })
                .collect()
        };
        assert_eq!(
            ids(&closure),
            ids(&base_only),
            "the self-loop fixpoint is the base set"
        );
        assert!(!base_only.is_empty(), "the populated store has accounts");
        Ok(())
    })
    .expect("read");
}

bumbledb::schema! {
    pub Graph;

    relation GraphEdge {
        src: u64,
        dst: u64,
    }
}

/// The transitive-closure program over [`Graph`]:
/// `p0(x, z) | GraphEdge(x, z); p0(x, z) | GraphEdge(x, y), p0(y, z)`.
fn closure_program() -> bumbledb::Program {
    use bumbledb::ir::{AtomSource, HeadTerm};
    let edge = |a: u16, b: u16| Atom {
        source: AtomSource::Edb(GraphEdge::RELATION),
        bindings: vec![
            (FieldId(0), Term::Var(VarId(a))),
            (FieldId(1), Term::Var(VarId(b))),
        ],
    };
    bumbledb::Program {
        predicates: vec![bumbledb::PredicateDef {
            head: vec![HeadTerm::Var, HeadTerm::Var],
            rules: vec![
                Rule {
                    finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
                    atoms: vec![edge(0, 1)],
                    negated: vec![],
                    conditions: vec![],
                },
                Rule {
                    finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
                    atoms: vec![
                        edge(0, 1),
                        Atom {
                            source: AtomSource::Idb(bumbledb::PredId(0)),
                            bindings: vec![
                                (FieldId(0), Term::Var(VarId(1))),
                                (FieldId(1), Term::Var(VarId(2))),
                            ],
                        },
                    ],
                    negated: vec![],
                    conditions: vec![],
                },
            ],
        }],
        output: bumbledb::PredId(0),
    }
}

/// Scalar/vectorized equality on a recursive fixture: the closure over
/// a chain-with-branches graph answers identically at batch size 1 (the
/// scalar regime) and the default vectorized batch — the driver rides
/// the ordinary executor, so the batch-size affordance covers the delta
/// variants exactly as it covers plain rules.
#[test]
fn recursive_answers_agree_scalar_and_vectorized() {
    let dir = common::TempDir::new("api-recursive-batch");
    let db = Db::create(dir.path(), Graph).expect("create");
    db.write(|tx| {
        for (src, dst) in [(0, 1), (1, 2), (2, 3), (3, 4), (1, 5), (5, 6), (2, 6)] {
            tx.insert(&GraphEdge { src, dst })?;
        }
        Ok(())
    })
    .expect("write");

    let pairs = |answers: &bumbledb::Answers| -> std::collections::BTreeSet<(u64, u64)> {
        answers
            .answers()
            .map(|answer| {
                let (AnswerValue::U64(x), AnswerValue::U64(z)) = (answer.get(0), answer.get(1))
                else {
                    panic!("closure columns are u64")
                };
                (x, z)
            })
            .collect()
    };
    let mut vectorized = db.prepare_program(&closure_program()).expect("prepare");
    let mut scalar = db.prepare_program(&closure_program()).expect("prepare");
    scalar.set_batch_size(1);
    db.read(|snap| {
        let vectorized = pairs(&snap.execute_collect(&mut vectorized, &[])?);
        let scalar = pairs(&snap.execute_collect(&mut scalar, &[])?);
        assert_eq!(scalar, vectorized, "one denotation, two batch regimes");
        // The hand answer: reachability over the fixed graph.
        let expected: std::collections::BTreeSet<(u64, u64)> = [
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
            (0, 5),
            (0, 6),
            (1, 2),
            (1, 3),
            (1, 4),
            (1, 5),
            (1, 6),
            (2, 3),
            (2, 4),
            (2, 6),
            (3, 4),
            (5, 6),
        ]
        .into_iter()
        .collect();
        assert_eq!(vectorized, expected, "the closure matches the hand answer");
        Ok(())
    })
    .expect("read");
}

/// The fixpoint observability surface (docs/architecture/40-execution.md
/// § observability): `profile` on a recursive program reports the
/// driver's per-stratum, per-round delta sizes and union accounting
/// through the `Counters` seam's fixpoint hooks, and `introspect`
/// renders labeled plan units plus the stratum section — counted paths
/// only; the release execute path monomorphizes `NoopCounters` and
/// reports nothing.
#[test]
fn fixpoint_profile_reports_strata_rounds_and_deltas() {
    let dir = common::TempDir::new("api-fixpoint-profile");
    let db = Db::create(dir.path(), Graph).expect("create");
    db.write(|tx| {
        for (src, dst) in [(0, 1), (1, 2), (2, 3), (3, 4), (1, 5), (5, 6), (2, 6)] {
            tx.insert(&GraphEdge { src, dst })?;
        }
        Ok(())
    })
    .expect("write");
    let mut prepared = db.prepare_program(&closure_program()).expect("prepare");
    db.read(|snap| {
        let (answers, stats) = snap.profile(&mut prepared, &[])?;
        assert_eq!(answers.len(), 16, "the closure's hand answer");
        assert!(
            stats.rules.is_empty(),
            "per-unit node stats do not exist under the driver"
        );
        // One recursive stratum, its rounds in order: round 0 is the
        // base rule (no delta images), each later round's delta is the
        // previous round's newly seen rows, and the converging round
        // derives nothing new.
        assert_eq!(stats.strata.len(), 1, "one predicate, one stratum");
        let stratum = &stats.strata[0];
        assert_eq!(stratum.stratum, 0);
        assert!(stratum.rounds[0].deltas.is_empty(), "round 0 has no delta");
        assert_eq!(
            stratum.rounds[0].emitted, 7,
            "the base rule emits each edge"
        );
        assert_eq!(stratum.rounds[0].absorbed, 0);
        let mut new_rows = Vec::new();
        for (idx, round) in stratum.rounds.iter().enumerate() {
            if idx > 0 {
                assert_eq!(round.deltas.len(), 1, "one predicate carries a delta");
                assert_eq!(round.deltas[0].predicate, 0);
                let prev = &stratum.rounds[idx - 1];
                assert_eq!(
                    round.deltas[0].rows,
                    prev.emitted - prev.absorbed,
                    "a round's delta is the previous round's newly seen rows"
                );
            }
            new_rows.push(round.emitted - round.absorbed);
        }
        assert_eq!(*new_rows.last().expect("rounds ran"), 0, "convergence");
        assert_eq!(
            new_rows.iter().sum::<u64>(),
            16,
            "newly seen across rounds is exactly the closure"
        );
        assert_eq!(
            stats.emits,
            stratum.rounds.iter().map(|r| r.emitted).sum::<u64>(),
            "whole-program emits are the per-round sum"
        );
        // The rendered report tells the same story: the version marker,
        // labeled plan units, and the stratum section.
        let (_, report) = snap.introspect(&mut prepared, &[])?;
        assert!(report.starts_with("introspection v3\n"), "{report}");
        assert!(report.contains("predicate p0 rule 0:"), "{report}");
        assert!(
            report.contains("predicate p0 rule 1 delta variant 0"),
            "{report}"
        );
        assert!(report.contains("stratum 0:"), "{report}");
        assert!(report.contains("round 1: delta p0=7; emitted "), "{report}");
        Ok(())
    })
    .expect("read");
}

/// The fixpoint budget at the public surface: a one-round budget trips
/// on a two-round closure with the typed execution error — constructed,
/// not hoped for (`Error::FixpointBudgetExceeded`, `MeasureOfRay`'s
/// error model: ids and counts, the snapshot stays usable).
#[test]
fn a_tight_fixpoint_budget_trips_with_the_typed_error() {
    use bumbledb::ir::{AtomSource, HeadTerm};
    let dir = common::TempDir::new("api-program-budget");
    let db = Db::create(dir.path(), Ledger).expect("create");
    db.write(|tx| {
        let holder: HolderId = tx.alloc()?;
        tx.insert(&Holder {
            id: holder,
            name: "alice",
        })?;
        for balance in [100, -25, 40] {
            let id: AccountId = tx.alloc()?;
            tx.insert(&Account {
                id,
                holder,
                balance,
            })?;
        }
        Ok(())
    })
    .expect("write");

    let base = Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(Account::RELATION),
            bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let recursive = Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Idb(bumbledb::PredId(0)),
            bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let program = bumbledb::Program {
        predicates: vec![bumbledb::PredicateDef {
            head: vec![HeadTerm::Var],
            rules: vec![base, recursive],
        }],
        output: bumbledb::PredId(0),
    };
    let mut prepared = db.prepare_program(&program).expect("recursion executes");
    prepared.set_fixpoint_budget(0, u64::MAX);
    let error = db
        .read(|snap| snap.execute_collect(&mut prepared, &[]).map(|_| ()))
        .expect_err("a zero-round budget cannot close a nonempty fixpoint");
    assert!(
        matches!(
            error,
            bumbledb::Error::FixpointBudgetExceeded {
                stratum: 0,
                rounds: 0,
                ..
            }
        ),
        "expected the typed budget error, got: {error}"
    );
    // The snapshot stays usable — MeasureOfRay's model: the same
    // prepared query executes clean once the budget admits the rounds.
    prepared.set_fixpoint_budget(16, u64::MAX);
    db.read(|snap| snap.execute_collect(&mut prepared, &[]).map(|_| ()))
        .expect("the widened budget closes the fixpoint");
}
