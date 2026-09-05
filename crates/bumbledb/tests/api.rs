//! end to end through the public surface — create → write{insert} →
//! read{point lookup, join, aggregate} → mutate via delete+insert → read
//! the export → collection-insert ETL round trip on both lanes (`insert`
use bumbledb::ir::{
    Atom, AtomSource, FindTerm, FoldOp, HeadTerm, InteriorId, NonEmpty, ParamId, Query, Rec,
    RecRule, RecStep, Rule, Term, Value, VarId,
};
use bumbledb::schema::FieldId;
use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{
    AnswerValue, Answers, BindValue, Db, Direction, Fact, ParamArg, StatementId, Theory,
};

mod common;

fn ledger_schema() -> bumbledb::Schema {
    Ledger
        .descriptor()
        .validate()
        .expect("the test schema is valid")
}

bumbledb::schema! {
    pub Ledger;

    relation Holder {
        id: u64 as HolderId,
        name: str,
    }
    relation Account {
        id: u64 as AccountId,
        holder: u64 as HolderId,
        balance: i64,
    }

    // Containment targets must be declared keys of the target relation
    // (chapter 10: "the target key/selection requirements remain checked
    // schema premises"); the statement order below is load-bearing —
    // violation tests assert StatementId(1) is the Account key and
    // StatementId(2) the containment.
    Holder(id) -> Holder;
    Account(id) -> Account;
    Account(holder) <= Holder(id);
}

/// The database issues no identity: tests mint application-owned ids from
/// one process-wide counter (unique, increasing — the old fresh shape).
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn mint() -> u64 {
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

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

fn aggregate_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
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
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");

    let accounts = db
        .write(common::work(), |tx| {
            let alice = HolderId(mint());
            tx.insert([&Holder {
                id: alice,
                name: "alice",
            }])?;
            let bob = HolderId(mint());
            tx.insert([&Holder {
                id: bob,
                name: "bob",
            }])?;
            let mut accounts = Vec::new();
            for (holder, balance) in [(alice, 100), (alice, -25), (bob, 40)] {
                let id = AccountId(mint());
                tx.insert([&Account {
                    id,
                    holder,
                    balance,
                }])?;
                accounts.push(Account {
                    id,
                    holder,
                    balance,
                });
            }
            Ok(accounts)
        })
        .expect("write")
        .unwrap()
        .value;

    let mut point = db.prepare(&point_query()).expect("prepare point");
    let mut join = db.prepare(&join_query()).expect("prepare join");
    let mut aggregate = db.prepare(&aggregate_query()).expect("prepare agg");
    db.read(common::work(), |snap| {
        let answers = snap.execute_collect(&mut point, &[BindValue::U64(accounts[2].id.0)])?;
        assert_eq!(answers.len(), 1);
        assert_eq!(answers.get(0, 0), AnswerValue::I64(40));

        let answers = snap.execute_collect(&mut join, &[] as &[bumbledb::BindValue])?;
        assert_eq!(
            name_amount_answers(&answers),
            vec![
                ("alice".to_owned(), -25),
                ("alice".to_owned(), 100),
                ("bob".to_owned(), 40),
            ]
        );

        let answers = snap.execute_collect(&mut aggregate, &[] as &[bumbledb::BindValue])?;
        assert_eq!(
            name_amount_answers(&answers),
            vec![("alice".to_owned(), 75), ("bob".to_owned(), 40)]
        );
        Ok(())
    })
    .expect("read");

    let old = accounts[0];
    db.write(common::work(), |tx| {
        tx.insert([&Account { balance: 90, ..old }])?;
        tx.delete([&old])?;
        Ok(())
    })
    .expect("mutate")
    .unwrap();

    db.read(common::work(), |snap| {
        let answers = snap.execute_collect(&mut join, &[] as &[bumbledb::BindValue])?;
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
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    db.write(common::work(), |tx| {
        let id = HolderId(mint());
        tx.insert([&Holder { id, name: "keep" }])
    })
    .expect("seed")
    .unwrap();

    // A panicking closure: the delta dies in the unwind, LMDB untouched.
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = db.write(common::work(), |tx| -> bumbledb::Result<()> {
            let id = HolderId(mint());
            tx.insert([&Holder {
                id,
                name: "doomed-by-panic",
            }])?;
            panic!("boom");
        });
    }));
    assert!(panicked.is_err());

    let failed = db.write(common::work(), |tx| -> bumbledb::Result<()> {
        let id = HolderId(mint());
        tx.insert([&Holder {
            id,
            name: "doomed-by-error",
        }])?;
        Err(bumbledb::Error::Overflow(
            bumbledb::OverflowKind::Aggregate {
                find: bumbledb::FindIndex(0),
            },
        ))
    });
    assert!(failed.is_err());

    db.write(common::work(), |tx| {
        let id = HolderId(mint());
        tx.insert([&Holder { id, name: "after" }])
    })
    .expect("mutex usable after a panic")
    .unwrap();

    let names = db
        .read(common::work(), |snap| {
            let mut names = Vec::new();
            for fact in snap.scan(Holder::RELATION)? {
                let fact = fact?;
                let Value::String(raw) = &fact[1] else {
                    panic!("field 1 is the name");
                };
                names.push(raw.to_string());
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
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");

    db.write(common::work(), |tx| {
        let holder = HolderId(mint());
        tx.insert([&Holder {
            id: holder,
            name: "seed",
        }])?;
        let id = AccountId(mint());
        tx.insert([&Account {
            id,
            holder,
            balance: 1,
        }])
    })
    .expect("seed")
    .unwrap();

    std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            for round in 0..20 {
                db.write(common::work(), |tx| {
                    let holder = HolderId(mint());
                    tx.insert([&Holder {
                        id: holder,
                        name: &format!("holder-{round}"),
                    }])?;
                    let id = AccountId(mint());
                    tx.insert([&Account {
                        id,
                        holder,
                        balance: round,
                    }])
                })
                .expect("paired write")
                .unwrap();
            }
        });
        for _ in 0..2 {
            scope.spawn(|| {
                for _ in 0..50 {
                    db.read(common::work(), |snap| {
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
fn export_scan_inserts_into_a_fresh_database() {
    let dir_old = common::TempDir::new("api-etl-old");
    let dir_new = common::TempDir::new("api-etl-new");
    let old = Db::create(dir_old.path(), Ledger, common::work())
        .expect("create old")
        .expect("accepted");

    let max_holder = old
        .write(common::work(), |tx| {
            let mut max = 0;
            for (name, balance) in [("alice", 100i64), ("bob", -7), ("carol", 40)] {
                let holder = HolderId(mint());
                tx.insert([&Holder { id: holder, name }])?;
                let id = AccountId(mint());
                tx.insert([&Account {
                    id,
                    holder,
                    balance,
                }])?;
                max = max.max(holder.0);
            }
            Ok(max)
        })
        .expect("seed")
        .unwrap()
        .value;

    let (holders, accounts) = old
        .read(common::work(), |snap| {
            let holders: Vec<Vec<Value>> =
                snap.scan(Holder::RELATION)?.collect::<Result<_, _>>()?;
            let accounts: Vec<Vec<Value>> =
                snap.scan(Account::RELATION)?.collect::<Result<_, _>>()?;
            Ok((holders, accounts))
        })
        .expect("export");

    let new = Db::create(dir_new.path(), Ledger, common::work())
        .expect("create new")
        .expect("accepted");
    let loaded = new
        .write(common::work(), |tx| {
            tx.insert_dyn(Holder::RELATION, holders)
                .map(bumbledb::MutationReport::changed)
        })
        .expect("load holders")
        .unwrap()
        .value;
    assert_eq!(loaded, 3);
    let loaded = new
        .write(common::work(), |tx| {
            tx.insert_dyn(Account::RELATION, accounts)
                .map(bumbledb::MutationReport::changed)
        })
        .expect("load accounts")
        .unwrap()
        .value;
    assert_eq!(loaded, 3);

    let mut join_old = old.prepare(&join_query()).expect("prepare");
    let answers_old = old
        .read(common::work(), |snap| snap.execute_collect(&mut join_old, &[] as &[bumbledb::BindValue]))
        .expect("query old");
    let mut join_new = new.prepare(&join_query()).expect("prepare");
    let answers_new = new
        .read(common::work(), |snap| snap.execute_collect(&mut join_new, &[] as &[bumbledb::BindValue]))
        .expect("query new");
    assert_eq!(
        name_amount_answers(&answers_old),
        name_amount_answers(&answers_new)
    );

    new.write(common::work(), |_tx| {
        let next = HolderId(mint());
        assert!(
            next.0 > max_holder,
            "minted {} at or below the imported high water {max_holder}",
            next.0
        );
        Ok(())
    })
    .expect("mint after import")
    .unwrap();
}

#[test]
#[expect(clippy::too_many_lines, reason = "one public-API citation walk")]
fn statement_violations_surface_from_commit_through_the_public_api() {
    let dir = common::TempDir::new("api-violations");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let holder = db
        .write(common::work(), |tx| {
            let id = HolderId(mint());
            tx.insert([&Holder { id, name: "alice" }])?;
            Ok(id)
        })
        .expect("seed")
        .unwrap()
        .value;

    let violations = common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Account {
            id: AccountId(7),
            holder,
            balance: 1,
        }])?;
        tx.insert([&Account {
            id: AccountId(7),
            holder,
            balance: 2,
        }])?;
        Ok(())
    }));
    let [
        (
            bumbledb::Violation::Functionality {
                statement, fact, ..
            },
            _,
        ),
    ] = violations.as_slice()
    else {
        panic!("expected one key citation, got {violations:?}");
    };

    assert_eq!(ledger_schema().id_of(*statement), StatementId(1));
    assert!(!fact.is_empty());

    let rendered = format!("{}", violations.display_with(&ledger_schema()));
    assert!(rendered.contains("Account(id) -> Account"), "{rendered}");
    let count = db
        .read(common::work(), |snap| Ok(snap.scan_facts::<Account>()?.count()))
        .expect("scan");
    assert_eq!(count, 0, "the aborted transaction left nothing");

    let violations = common::expect_rejected(db.write(common::work(), |tx| {
        tx.insert([&Account {
            id: AccountId(1),
            holder: HolderId(404),
            balance: 5,
        }])
    }));
    let [
        (
            bumbledb::Violation::Containment {
                direction: Direction::SourceUnsatisfied,
                ..
            },
            _,
        ),
    ] = violations.as_slice()
    else {
        panic!("expected one source-unsatisfied citation, got {violations:?}");
    };
    assert_eq!(
        violations.get(0).unwrap().statement_id(&ledger_schema()),
        StatementId(2)
    );
    let rendered = format!("{}", violations.display_with(&ledger_schema()));
    assert!(
        rendered.contains("Account(holder) <= Holder(id)"),
        "{rendered}"
    );
    assert!(rendered.contains("source"), "{rendered}");

    db.write(common::work(), |tx| {
        tx.insert([&Account {
            id: AccountId(1),
            holder,
            balance: 5,
        }])
    })
    .expect("reference the holder")
    .unwrap();
    let violations = common::expect_rejected(db.write(common::work(), |tx| {
        tx.delete([&Holder {
            id: holder,
            name: "alice",
        }])
    }));
    let [
        (
            bumbledb::Violation::Containment {
                direction, fact, ..
            },
            _,
        ),
    ] = violations.as_slice()
    else {
        panic!("expected one containment citation, got {violations:?}");
    };
    // Final-state semantics: deleting the referenced holder leaves the
    // account SOURCE-unsatisfied — the one direction the judge speaks
    // (which command moved is not part of the verdict).
    assert_eq!(*direction, Direction::SourceUnsatisfied);
    assert!(
        !fact.is_empty(),
        "the requiring source is named by its fact"
    );
    let rendered = format!("{}", violations.display_with(&ledger_schema()));
    assert!(
        rendered.contains("Account(holder) <= Holder(id)"),
        "{rendered}"
    );
    assert!(rendered.contains("source"), "{rendered}");
}

#[test]
fn open_mismatches_and_snapshot_usability() {
    let dir = common::TempDir::new("api-open-mismatch");
    drop(
        Db::create(dir.path(), Ledger, common::work())
            .expect("create")
            .expect("accepted"),
    );

    let other = bumbledb::schema::SchemaDescriptor {
        relations: vec![bumbledb::schema::RelationDescriptor {
            extension: None,
            name: "Other".into(),
            fields: vec![bumbledb::schema::FieldDescriptor {
                name: "x".into(),
                value_type: bumbledb::schema::ValueType::U64,
            }],
        }],
        statements: vec![],
    };
    let Err(err) = Db::open(dir.path(), other, common::work()).map(|_| ()) else {
        panic!("a different schema must refuse to open");
    };
    assert!(
        matches!(&err, bumbledb::Error::Store(e)
            if matches!(**e, bumbledb::store::StoreError::SchemaMismatch)),
        "{err:?}"
    );

    let Err(err) = Db::create(dir.path(), Ledger, common::work()).map(|_| ()) else {
        panic!("create over an existing environment must refuse");
    };
    assert!(
        matches!(&err, bumbledb::Error::Store(e)
            if matches!(**e, bumbledb::store::StoreError::DestinationExists { .. })),
        "{err:?}"
    );

    let db = Db::open(dir.path(), Ledger, common::work()).expect("open");
    db.write(common::work(), |tx| {
        let id = HolderId(mint());
        tx.insert([&Holder { id, name: "bo" }])
    })
    .expect("seed")
    .unwrap();
    let mut join = db.prepare(&join_query()).expect("prepare");
    db.read(common::work(), |snap| {
        let mut out = Answers::new();

        let err = snap
            .execute(&mut join, &[BindValue::U64(1)], &mut out)
            .unwrap_err();
        assert!(matches!(err, bumbledb::Error::ParamCountMismatch { .. }));

        snap.execute(&mut join, &[] as &[bumbledb::BindValue], &mut out)?;
        assert_eq!(out.len(), 0, "no accounts yet");
        Ok(())
    })
    .expect("snapshot stays usable");
}

#[test]
fn pinned_snapshot_reads_its_generation_across_later_commits() {
    let dir = common::TempDir::new("api-pinned");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    db.write(common::work(), |tx| {
        let id = HolderId(mint());
        tx.insert([&Holder { id, name: "first" }])
    })
    .expect("seed")
    .unwrap();

    let mut join = db.prepare(&join_query()).expect("prepare");
    db.read(common::work(), |snap| {
        let before = snap.scan_facts::<Holder>()?.count();
        assert_eq!(before, 1);
        // Two commits land while this snapshot stays open (LMDB readers

        for round in 0..2 {
            db.write(common::work(), |tx| {
                let id = HolderId(mint());
                tx.insert([&Holder {
                    id,
                    name: &format!("later-{round}"),
                }])
            })?
            .unwrap();
        }

        assert_eq!(snap.scan_facts::<Holder>()?.count(), 1);
        let answers = snap.execute_collect(&mut join, &[] as &[bumbledb::BindValue])?;
        assert_eq!(answers.len(), 0);
        Ok(())
    })
    .expect("pinned read");

    let after = db
        .read(common::work(), |snap| Ok(snap.scan_facts::<Holder>()?.count()))
        .expect("fresh read");
    assert_eq!(after, 3);
}

#[test]
fn collection_insert_equals_sequential_inserts() {
    let dir_all = common::TempDir::new("api-insert-all");
    let dir_seq = common::TempDir::new("api-insert-seq");
    let all = Db::create(dir_all.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let seq = Db::create(dir_seq.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");

    let n = 4_100u64;
    let facts: Vec<Vec<Value>> = (0..n)
        .map(|i| vec![Value::U64(i), Value::String(format!("h{}", i % 97).into())])
        .collect();
    let loaded = all
        .write(common::work(), |tx| {
            tx.insert_dyn(Holder::RELATION, facts.clone())
                .map(bumbledb::MutationReport::changed)
        })
        .expect("collection insert")
        .unwrap()
        .value;
    assert_eq!(loaded, n);
    for chunk in facts.chunks(512) {
        seq.write(common::work(), |tx| tx.insert_dyn(Holder::RELATION, chunk).map(|_| ()))
            .expect("sequential insert")
            .unwrap();
    }

    let by_id = |mut rows: Vec<Vec<Value>>| {
        rows.sort_by_key(|f| match f[0] {
            Value::U64(id) => id,
            _ => unreachable!("id column"),
        });
        rows
    };
    let a = by_id(
        all.read(common::work(), |snap| snap.scan(Holder::RELATION)?.collect::<Result<_, _>>())
            .expect("scan all"),
    );
    let b = by_id(
        seq.read(common::work(), |snap| snap.scan(Holder::RELATION)?.collect::<Result<_, _>>())
            .expect("scan seq"),
    );
    assert_eq!(a, b);
    assert_eq!(a.len(), usize::try_from(n).expect("64-bit"));

    let dir_fail = common::TempDir::new("api-insert-fail");
    let fail = Db::create(dir_fail.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let mut bad = facts;
    bad[4_099] = vec![Value::U64(0)];
    let err = fail
        .write(common::work(), |tx| {
            tx.insert_dyn(Holder::RELATION, bad)
                .map(bumbledb::MutationReport::changed)
        })
        .unwrap_err();
    assert!(matches!(err, bumbledb::Error::FactShape(_)), "{err:?}");
    let persisted = fail
        .read(common::work(), |snap| Ok(snap.scan_facts::<Holder>()?.count()))
        .expect("scan");
    assert_eq!(persisted, 0);
}

#[test]
fn typed_collection_insert_is_idempotent_and_judgment_rejects_the_write() {
    let dir = common::TempDir::new("api-insert-typed");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");

    let n = 4_100u64;
    let names = ["ada", "bob", "eve"];
    let holder = |i: u64| Holder {
        id: HolderId(i),
        name: names[usize::try_from(i % 3).expect("small")],
    };
    let holders: Vec<_> = (0..n).map(holder).collect();
    let loaded = db
        .write(common::work(), |tx| Ok(tx.insert(&holders)?.changed()))
        .expect("typed insert")
        .unwrap()
        .value;
    assert_eq!(loaded, n);
    let again = db
        .write(common::work(), |tx| Ok(tx.insert(&holders)?.changed()))
        .expect("typed re-import")
        .unwrap()
        .value;
    assert_eq!(again, 0);
    let persisted = db
        .read(common::work(), |snap| Ok(snap.scan_facts::<Holder>()?.count()))
        .expect("scan");
    assert_eq!(persisted, usize::try_from(n).expect("64-bit"));

    let account = |i: u64| Account {
        id: AccountId(i),
        holder: HolderId(if i == 4_099 { n + 7 } else { i % 3 }),
        balance: 1,
    };
    let accounts: Vec<_> = (0..n).map(account).collect();
    let _ = common::expect_rejected(db.write(common::work(), |tx| Ok(tx.insert(&accounts)?.changed())));
    let account_count = db
        .read(common::work(), |snap| Ok(snap.scan_facts::<Account>()?.count()))
        .expect("scan accounts");
    assert_eq!(account_count, 0);
}

#[test]
fn disk_size_and_generation_report_store_state() {
    let dir = common::TempDir::new("api-disk-size");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let empty = db.disk_size().expect("size");
    assert!(empty > 0, "a fresh environment still has pages");
    assert_eq!(db.generation(common::work()).expect("gen").value(), 0);

    db.write(common::work(), |tx| {
        for _ in 0..10_000u64 {
            let id = HolderId(mint());
            tx.insert([&Holder {
                id,
                name: &format!("holder-{}", id.0),
            }])?;
        }
        Ok(())
    })
    .expect("collection write")
    .unwrap();
    let grown = db.disk_size().expect("size");
    assert!(grown > empty, "10k facts grow the file: {empty} -> {grown}");
    assert_eq!(db.generation(common::work()).expect("gen").value(), 1);
}

#[test]
fn cover_choice_iterates_the_selected_side() {
    use bumbledb::ir::{Atom, FindTerm, FoldOp, ParamId, Query, Term, VarId};

    let dir = common::TempDir::new("api-cover-choice");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");

    db.write(common::work(), |tx| {
        let mut holders = Vec::new();
        for i in 0..500u64 {
            let id = HolderId(mint());
            let name = if i < 7 {
                "target".to_owned()
            } else {
                format!("h{i}")
            };
            tx.insert([&Holder { id, name: &name }])?;
            holders.push(id);
        }
        for i in 0..10_000u64 {
            let id = AccountId(mint());
            tx.insert([&Account {
                id,
                holder: holders[usize::try_from(i % 500).expect("small")],
                balance: i64::try_from(i).expect("fits"),
            }])?;
        }
        Ok(())
    })
    .expect("populate")
    .unwrap();

    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
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
    let params = vec![ParamArg::Scalar(BindValue::Str("target"))];
    let out = db
        .read(common::work(), |snap| snap.execute_collect(&mut prepared, &params))
        .expect("execute");
    assert_eq!(out.len(), 7, "one group per target holder");
}

#[test]
fn compaction_drops_the_freelist_and_preserves_content() {
    use bumbledb::ir::Value;

    let dir = common::TempDir::new("api-compact");
    let source_dir = dir.path().join("source");
    let db = Db::create(&source_dir, Ledger, common::work())
        .expect("create")
        .expect("accepted");

    for round in 0..40u64 {
        db.write(common::work(), |tx| {
            for i in 0..250u64 {
                let id = HolderId(mint());
                tx.insert([&Holder {
                    id,
                    name: &format!("h{round}-{i}"),
                }])?;
            }
            Ok(())
        })
        .expect("commit")
        .unwrap();
    }
    let source_size = db.disk_size().expect("size");
    let generation = db.generation(common::work()).expect("generation");
    let scan_digest = |db: &Db<Ledger>| -> Vec<Vec<Value>> {
        let mut rows: Vec<Vec<Value>> = db
            .read(common::work(), |snap| snap.scan(Holder::RELATION)?.collect::<Result<_, _>>())
            .expect("scan");
        rows.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        rows
    };
    let source_rows = scan_digest(&db);

    let compact_dir = dir.path().join("compacted");
    db.compact(&compact_dir).expect("compact");

    let err = db.compact(&compact_dir).expect_err("must refuse");
    assert!(
        matches!(&err, bumbledb::Error::Store(e)
            if matches!(**e, bumbledb::store::StoreError::DestinationExists { .. })),
        "{err:?}"
    );
    drop(db);

    let compacted = Db::open(&compact_dir, Ledger, common::work()).expect("open compacted");
    let compact_size = compacted.disk_size().expect("size");
    assert!(
        compact_size * 10 <= source_size * 8,
        "compaction reclaims the churn: {compact_size} vs {source_size}"
    );
    assert_eq!(compacted.generation(common::work()).expect("generation"), generation);
    assert_eq!(scan_digest(&compacted), source_rows, "byte-identical facts");

    compacted
        .write(common::work(), |tx| {
            let id = HolderId(mint());
            tx.insert([&Holder {
                id,
                name: "post-compaction",
            }])
        })
        .expect("write")
        .unwrap();
    assert_eq!(
        scan_digest(&compacted).len(),
        source_rows.len() + 1,
        "the compacted store keeps living"
    );
}

/// Before the environment-instance check, executing A's prepared query against
/// B (same schema, same generation) returned B's data through A's memo keys.
#[test]
fn a_prepared_query_refuses_a_foreign_snapshot() {
    let dir_a = common::TempDir::new("api-foreign-prepared-a");
    let dir_b = common::TempDir::new("api-foreign-prepared-b");
    let db_a = Db::create(dir_a.path(), Ledger, common::work())
        .expect("create a")
        .expect("accepted");
    let db_b = Db::create(dir_b.path(), Ledger, common::work())
        .expect("create b")
        .expect("accepted");
    for (db, name, balance) in [(&db_a, "alice", 10), (&db_b, "bob", 20)] {
        db.write(common::work(), |tx| {
            let holder = HolderId(mint());
            tx.insert([&Holder { id: holder, name }])?;
            let id = AccountId(mint());
            tx.insert([&Account {
                id,
                holder,
                balance,
            }])
        })
        .expect("seed one distinct fact pair")
        .unwrap();
    }
    assert_eq!(db_a.generation(common::work()).expect("gen a").value(), 1);
    assert_eq!(
        db_b.generation(common::work()).expect("gen b").value(),
        1,
        "both clocks read 1"
    );

    let mut prepared = db_a.prepare(&join_query()).expect("prepare on A");
    db_a.read(common::work(), |snap| {
        let out = snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?;
        assert_eq!(name_amount_answers(&out), vec![("alice".to_owned(), 10)]);
        Ok(())
    })
    .expect("execute on the preparing db");

    db_b.read(common::work(), |snap| {
        let err = snap
            .execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])
            .unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::ForeignPreparedQuery),
            "{err:?}"
        );
        let mut out = Answers::new();
        let err = snap
            .execute(&mut prepared, &[] as &[bumbledb::BindValue], &mut out)
            .unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::ForeignPreparedQuery),
            "{err:?}"
        );
        let err = snap.introspect(&mut prepared, &[]).unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::ForeignPreparedQuery),
            "{err:?}"
        );
        Ok(())
    })
    .expect("read on b");

    db_a.read(common::work(), |snap| {
        let out = snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?;
        assert_eq!(name_amount_answers(&out), vec![("alice".to_owned(), 10)]);
        Ok(())
    })
    .expect("A unaffected");
}

#[test]
fn a_second_handle_on_a_live_path_is_locked_out() {
    let dir = common::TempDir::new("api-env-lock");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let err = Db::open(dir.path(), Ledger, common::work()).map(|_| ()).unwrap_err();
    assert!(
        matches!(&err, bumbledb::Error::Store(e)
            if matches!(**e, bumbledb::store::StoreError::StoreLocked { .. })),
        "{err:?}"
    );
    let err = Db::create(dir.path(), Ledger, common::work()).map(|_| ()).unwrap_err();
    assert!(
        matches!(&err, bumbledb::Error::Store(e)
            if matches!(**e, bumbledb::store::StoreError::DestinationExists { .. })),
        "create refuses an existing destination before the lock: {err:?}"
    );
    drop(db);
    let reopened = Db::open(dir.path(), Ledger, common::work()).expect("the lock died with the handle");
    drop(reopened);
}

#[test]
#[expect(
    unsafe_code,
    reason = "the localized unsafe operation has a documented safety invariant"
)]
fn create_refuses_a_foreign_lmdb_environment() {
    let dir = common::TempDir::new("api-env-foreign-lmdb");
    std::fs::create_dir_all(dir.path()).expect("mkdir");
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
    let err = Db::create(dir.path(), Ledger, common::work()).map(|_| ()).unwrap_err();
    assert!(
        matches!(&err, bumbledb::Error::Store(e)
            if matches!(**e, bumbledb::store::StoreError::DestinationExists { .. })),
        "{err:?}"
    );

    let dir = common::TempDir::new("api-env-half-created");
    std::fs::create_dir_all(dir.path()).expect("mkdir");
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
    // A half-created foreign environment is not a recognizable store: the
    // successor refuses the open before adopting anything.
    let err = Db::open(dir.path(), Ledger, common::work()).map(|_| ()).unwrap_err();
    assert!(
        matches!(&err, bumbledb::Error::Store(e)
            if matches!(**e, bumbledb::store::StoreError::UnrecognizedStore { .. })),
        "{err:?}"
    );
    let err = Db::create(dir.path(), Ledger, common::work()).map(|_| ()).unwrap_err();
    assert!(
        matches!(&err, bumbledb::Error::Store(e)
            if matches!(**e, bumbledb::store::StoreError::DestinationExists { .. })),
        "{err:?}"
    );
}

#[test]
fn nested_write_is_a_typed_refusal_instead_of_deadlocking() {
    let dir = common::TempDir::new("api-nested-write");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    // The successor refuses reentrancy with the typed
    // `StoreError::ReentrantWriter` — never a deadlock, never a panic.
    let err = db
        .write(common::work(), |_| db.write(common::work(), |_| Ok(())).map(|_| ()))
        .expect_err("the nested write refuses");
    assert!(
        matches!(&err, bumbledb::Error::Store(e)
            if matches!(**e, bumbledb::store::StoreError::ReentrantWriter)),
        "{err:?}"
    );

    db.write(common::work(), |tx| {
        let id = HolderId(mint());
        tx.insert([&Holder {
            id,
            name: "after the refusal",
        }])
    })
    .expect("the writer survives")
    .unwrap();
}

#[test]
fn prepared_executions_observe_exactly_one_generation() {
    let dir = common::TempDir::new("api-gen-atomic");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let (hx, hy, ax, ay) = db
        .write(common::work(), |tx| {
            let hx = HolderId(mint());
            tx.insert([&Holder { id: hx, name: "x" }])?;
            let hy = HolderId(mint());
            tx.insert([&Holder { id: hy, name: "y" }])?;
            let ax = AccountId(mint());
            tx.insert([&Account {
                id: ax,
                holder: hx,
                balance: 0,
            }])?;
            let ay = AccountId(mint());
            tx.insert([&Account {
                id: ay,
                holder: hy,
                balance: 0,
            }])?;
            Ok((hx, hy, ax, ay))
        })
        .expect("seed")
        .unwrap()
        .value;

    let db = &db;
    std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            for round in 1..=40i64 {
                db.write(common::work(), |tx| {
                    tx.delete([&Account {
                        id: ax,
                        holder: hx,
                        balance: round - 1,
                    }])?;
                    tx.insert([&Account {
                        id: ax,
                        holder: hx,
                        balance: round,
                    }])?;
                    tx.delete([&Account {
                        id: ay,
                        holder: hy,
                        balance: round - 1,
                    }])?;
                    tx.insert([&Account {
                        id: ay,
                        holder: hy,
                        balance: round,
                    }])
                })
                .expect("paired rewrite")
                .unwrap();
            }
        });
        for _ in 0..3 {
            scope.spawn(|| {
                let mut prepared = db.prepare(&join_query()).expect("prepare");
                let mut out = Answers::new();
                for _ in 0..80 {
                    db.read(common::work(), |snap| {
                        snap.execute(&mut prepared, &[] as &[bumbledb::BindValue], &mut out)?;
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

/// The old escaped-fresh-id halves retired with the fresh machinery
/// (E-NO-RESERVE): there is no bare reserve and no `Q` mark. What
/// survives is the state-change law itself — a write that nets to
/// nothing must not move the generation.
#[test]
fn a_nets_to_nothing_write_is_not_a_state_change() {
    let dir = common::TempDir::new("api-fresh-escape");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");

    db.write(common::work(), |tx| {
        tx.insert([&Holder {
            id: HolderId(mint()),
            name: "first real holder",
        }])
        .map(|_| ())
    })
    .expect("real write")
    .unwrap();
    let generation_after_seed = db.generation(common::work()).expect("generation");
    assert_eq!(generation_after_seed.value(), 1);

    db.write(common::work(), |tx| {
        let ghost = Holder {
            id: HolderId(mint()),
            name: "ghost",
        };
        tx.insert([&ghost])?;
        tx.delete([&ghost])?;
        Ok(())
    })
    .expect("nets to nothing")
    .unwrap();
    assert_eq!(
        db.generation(common::work()).expect("generation"),
        generation_after_seed,
        "a nets-to-nothing write is not a state change"
    );
}

#[test]
fn deleting_a_never_interned_string_is_a_mint_free_noop() {
    let dir = common::TempDir::new("api-mint-free-delete");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let holder = db
        .write(common::work(), |tx| {
            let id = HolderId(mint());
            tx.insert([&Holder { id, name: "real" }])?;
            Ok(id)
        })
        .expect("seed")
        .unwrap()
        .value;

    let generation = db.generation(common::work()).expect("generation");
    db.write(common::work(), |tx| {
        let changed = tx.delete([&Holder {
            id: holder,
            name: "never interned",
        }])?;
        assert_eq!(
            changed.changed(),
            0,
            "a never-interned value matches no fact"
        );
        Ok(())
    })
    .expect("typed delete")
    .unwrap();

    db.write(common::work(), |tx| {
        let changed = tx.delete_dyn(
            Holder::RELATION,
            [&[
                Value::U64(holder.0),
                Value::String("also never interned".into()),
            ]],
        )?;
        assert_eq!(changed.changed(), 0);
        Ok(())
    })
    .expect("dynamic delete")
    .unwrap();
    assert_eq!(db.generation(common::work()).expect("generation"), generation);

    db.write(common::work(), |tx| {
        let id = HolderId(mint());
        let transient = Holder {
            id,
            name: "transient",
        };
        assert_eq!(tx.insert([&transient])?.changed(), 1);
        assert_eq!(tx.delete([&transient])?.changed(), 1);
        Ok(())
    })
    .expect("cancel")
    .unwrap();
    let names: Vec<String> = db
        .read(common::work(), |snap| {
            snap.scan_facts::<Holder>()?
                .map(|h| h.map(|h| h.name.to_owned()))
                .collect::<bumbledb::Result<Vec<_>>>()
        })
        .expect("scan");
    assert_eq!(names, vec!["real".to_owned()]);
}

#[test]
fn out_of_range_relation_ids_are_typed_errors() {
    let dir = common::TempDir::new("api-unknown-relation");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let bogus = bumbledb::RelationId(999);
    let is_unknown = |err: &bumbledb::Error| {
        matches!(
            err,
            bumbledb::Error::FactShape(bumbledb::error::FactShapeError::Id(
                bumbledb::DynIdError::UnknownRelation { relation }
            )) if relation.0 == 999
        )
    };

    db.write(common::work(), |tx| {
        let err = tx.insert_dyn(bogus, [&[Value::U64(1)]]).unwrap_err();
        assert!(is_unknown(&err), "{err:?}");
        let err = tx.delete_dyn(bogus, [&[Value::U64(1)]]).unwrap_err();
        assert!(is_unknown(&err), "{err:?}");
        Ok(())
    })
    .expect("write closes cleanly")
    .unwrap();

    let err = db
        .write(common::work(), |tx| {
            tx.insert_dyn(bogus, vec![vec![Value::U64(1)]])
                .map(bumbledb::MutationReport::changed)
        })
        .map(|_| ())
        .unwrap_err();
    assert!(is_unknown(&err), "{err:?}");

    db.read(common::work(), |snap| {
        let err = snap.scan(bogus).map(|_| ()).unwrap_err();
        assert!(is_unknown(&err), "{err:?}");
        Ok(())
    })
    .expect("read closes cleanly");
}

#[test]
fn a_plain_query_executes_as_today() {
    let dir = common::TempDir::new("api-degenerate-query");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    db.write(common::work(), |tx| {
        for (name, balances) in [("alice", vec![100, -25]), ("bob", vec![40])] {
            let holder = HolderId(mint());
            tx.insert([&Holder { id: holder, name }])?;
            for balance in balances {
                let id = AccountId(mint());
                tx.insert([&Account {
                    id,
                    holder,
                    balance,
                }])?;
            }
        }
        Ok(())
    })
    .expect("write")
    .unwrap();

    let query = join_query();
    let mut prepared = db.prepare(&query).expect("prepare query");
    db.read(common::work(), |snap| {
        let answers = snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?;
        assert_eq!(answers.len(), 3);
        let again = snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?;
        assert_eq!(
            name_amount_answers(&answers),
            name_amount_answers(&again),
            "a plain query is stable across executions"
        );
        Ok(())
    })
    .expect("read");
}

fn identity_main(arity: u16) -> Rule {
    Rule {
        finds: (0..arity).map(|i| FindTerm::Var(VarId(i))).collect(),
        atoms: vec![Atom {
            source: AtomSource::Interior(InteriorId(0)),
            bindings: (0..arity)
                .map(|i| (FieldId(i), Term::Var(VarId(i))))
                .collect(),
        }],
        negated: vec![],
        conditions: vec![],
    }
}

/// Recursion at the public surface: a roster-clean linear rec prepares and
/// executes under the reach driver, and the self-loop `interior 0(x) |
/// Account(id: x); interior 0(x) | interior 0(x)` denotes exactly the base
/// rule's set — the rec arm re-derives, the seen-set absorbs, the fixpoint
/// closes in one growing round (`lean/Bumbledb/Exec/Reach.lean:
/// evalLinearReach_eq_lfp`).
#[test]
fn prepare_executes_recursion_under_the_driver() {
    let dir = common::TempDir::new("api-reach-driver");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    db.write(common::work(), |tx| {
        let holder = HolderId(mint());
        tx.insert([&Holder {
            id: holder,
            name: "alice",
        }])?;
        for balance in [100, -25, 40] {
            let id = AccountId(mint());
            tx.insert([&Account {
                id,
                holder,
                balance,
            }])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();

    let base = Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(Account::RELATION),
            bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![],
    };
    let query = Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0)],
                atoms: base.atoms.clone(),
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0)],
                self_bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                atoms: vec![],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var],
        rules: vec![identity_main(1)],
    };
    let mut recursive_prepared = db.prepare(&query).expect("recursion executes");
    let mut base_prepared = db.prepare(&Query::single(base)).expect("prepare base");
    db.read(common::work(), |snap| {
        let closure =
            snap.execute_collect(&mut recursive_prepared, &[] as &[bumbledb::BindValue])?;
        let base_only = snap.execute_collect(&mut base_prepared, &[] as &[bumbledb::BindValue])?;
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

fn closure_query() -> Query {
    let edge = |a: u16, b: u16| Atom {
        source: AtomSource::Edb(GraphEdge::RELATION),
        bindings: vec![
            (FieldId(0), Term::Var(VarId(a))),
            (FieldId(1), Term::Var(VarId(b))),
        ],
    };
    Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: vec![edge(0, 1)],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0), VarId(2)],
                self_bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
                atoms: vec![edge(0, 1)],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![identity_main(2)],
    }
}

fn primer_reach_xx() -> Query {
    let edge = |a: u16, b: u16| Atom {
        source: AtomSource::Edb(GraphEdge::RELATION),
        bindings: vec![
            (FieldId(0), Term::Var(VarId(a))),
            (FieldId(1), Term::Var(VarId(b))),
        ],
    };
    Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0), VarId(1)],
                atoms: vec![edge(0, 1)],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(0), VarId(2)],
                self_bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
                atoms: vec![edge(0, 1)],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        }],
    }
}

#[test]
fn recursive_answers_agree_scalar_and_vectorized() {
    let dir = common::TempDir::new("api-recursive-batch");
    let db = Db::create(dir.path(), Graph, common::work())
        .expect("create")
        .expect("accepted");
    db.write(common::work(), |tx| {
        for (src, dst) in [(0, 1), (1, 2), (2, 3), (3, 4), (1, 5), (5, 6), (2, 6)] {
            tx.insert([&GraphEdge { src, dst }])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();

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
    let mut vectorized = db.prepare(&closure_query()).expect("prepare");
    let mut scalar = db.prepare(&closure_query()).expect("prepare");
    scalar.set_batch_size(1);
    db.read(common::work(), |snap| {
        let vectorized =
            pairs(&snap.execute_collect(&mut vectorized, &[] as &[bumbledb::BindValue])?);
        let scalar = pairs(&snap.execute_collect(&mut scalar, &[] as &[bumbledb::BindValue])?);
        assert_eq!(scalar, vectorized, "one denotation, two batch regimes");

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

#[test]
fn primer_shaped_reach_xx_is_empty_on_a_dag() {
    let dir = common::TempDir::new("api-primer-reach-xx");
    let db = Db::create(dir.path(), Graph, common::work())
        .expect("create")
        .expect("accepted");
    db.write(common::work(), |tx| {
        for (src, dst) in [(0, 1), (1, 2), (2, 3)] {
            tx.insert([&GraphEdge { src, dst }])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();
    let mut prepared = db.prepare(&primer_reach_xx()).expect("prepare");
    db.read(common::work(), |snap| {
        let answers = snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?;
        assert!(answers.is_empty(), "a DAG has no reach(x, x)");
        Ok(())
    })
    .expect("read");

    db.write(common::work(), |tx| {
        tx.insert([&GraphEdge { src: 3, dst: 0 }])?;
        Ok(())
    })
    .expect("close the cycle")
    .unwrap();
    db.read(common::work(), |snap| {
        let answers = snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?;
        assert!(!answers.is_empty(), "a cycle produces reach(x, x)");
        Ok(())
    })
    .expect("read after cycle");
}

#[test]
fn reach_execute_answers_the_closure() {
    let dir = common::TempDir::new("api-reach-execute");
    let db = Db::create(dir.path(), Graph, common::work())
        .expect("create")
        .expect("accepted");
    db.write(common::work(), |tx| {
        for (src, dst) in [(0, 1), (1, 2), (2, 3), (3, 4), (1, 5), (5, 6), (2, 6)] {
            tx.insert([&GraphEdge { src, dst }])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();
    let mut prepared = db.prepare(&closure_query()).expect("prepare");
    db.read(common::work(), |snap| {
        let answers = snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])?;
        assert_eq!(answers.len(), 16, "the closure's hand answer");
        let (_, report) = snap.introspect(&mut prepared, &[])?;
        assert!(report.contains("query:"), "{report}");
        Ok(())
    })
    .expect("read");
}

#[test]
fn a_tight_derived_budget_trips_under_reach() {
    const CHAIN: u64 = 66_000;
    let dir = common::TempDir::new("api-reach-budget");
    let db = Db::create(dir.path(), Graph, common::work())
        .expect("create")
        .expect("accepted");
    db.write(common::work(), |tx| {
        for n in 0..CHAIN {
            tx.insert([&GraphEdge { src: n, dst: n + 1 }])?;
        }
        Ok(())
    })
    .expect("write")
    .unwrap();
    let mut prepared = db
        .prepare(&single_source_chain_query())
        .expect("recursion executes");
    let error = db
        .read(common::work(), |snap| {
            snap.execute_collect(&mut prepared, &[] as &[bumbledb::BindValue])
                .map(|_| ())
        })
        .expect_err("66k hops exceed the default 2^16-round budget");
    assert!(
        matches!(
            error,
            bumbledb::Error::DerivedBudgetExceeded { rounds, .. } if rounds > 0
        ),
        "expected DerivedBudgetExceeded with rounds > 0, got: {error}"
    );
}

fn single_source_chain_query() -> Query {
    Query {
        interiors: vec![],
        rec: Some(Rec {
            base: NonEmpty::one(RecRule {
                finds: vec![VarId(0)],
                atoms: vec![Atom {
                    source: AtomSource::Edb(GraphEdge::RELATION),
                    bindings: vec![
                        (FieldId(0), Term::Literal(Value::U64(0))),
                        (FieldId(1), Term::Var(VarId(0))),
                    ],
                }],
                conditions: vec![],
            }),
            rec: NonEmpty::one(RecStep {
                finds: vec![VarId(1)],
                self_bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
                atoms: vec![Atom {
                    source: AtomSource::Edb(GraphEdge::RELATION),
                    bindings: vec![
                        (FieldId(0), Term::Var(VarId(0))),
                        (FieldId(1), Term::Var(VarId(1))),
                    ],
                }],
                conditions: vec![],
            }),
        }),
        head: vec![HeadTerm::Var],
        rules: vec![identity_main(1)],
    }
}
