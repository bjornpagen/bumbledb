//! PRD 28 integration tests: the `60-api.md` usage shapes end to end
//! through the public surface — create → write{alloc+insert} → read{point
//! lookup, join, aggregate} → mutate via delete+insert → read again; the
//! write-closure abort contracts; the threading contract; and the export →
//! `bulk_load` ETL round trip.

use std::path::PathBuf;

use bumbledb::ir::{AggOp, Atom, FindTerm, ParamId, Query, Term, Value, VarId};
use bumbledb::schema::FieldId;
use bumbledb::{Db, Fact, ResultBuffer, ResultValue};

bumbledb::schema! {
    relation Holder {
        id: u64 as HolderId, serial,
        name: str,
    }
    relation Account {
        id: u64 as AccountId, serial,
        holder: u64 as HolderId, fk(Holder.id),
        balance: i64,
    }
}

fn test_dir(tag: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("bumbledb-api-{tag}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("test dir");
    path
}

/// Q(name, balance) :- Account(holder = h, balance), Holder(id = h, name).
fn join_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: Account::RELATION,
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: Holder::RELATION,
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        predicates: vec![],
    }
}

/// Q(name, Sum(balance)) :- Account(holder = h, balance), Holder(id = h,
/// name).
fn aggregate_query() -> Query {
    Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![
            Atom {
                relation: Account::RELATION,
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: Holder::RELATION,
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        predicates: vec![],
    }
}

/// Q(balance) :- Account(id = ?0, balance) — the point-lookup (guard) shape.
fn point_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: Account::RELATION,
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        predicates: vec![],
    }
}

/// Collects a two-column (String, I64) result buffer into a sorted vec —
/// results are sets; the host sorts.
fn name_amount_rows(out: &ResultBuffer) -> Vec<(String, i64)> {
    let mut rows: Vec<(String, i64)> = (0..out.len())
        .map(|row| {
            let ResultValue::String(name) = out.get(row, 0) else {
                panic!("column 0 is a string");
            };
            let ResultValue::I64(amount) = out.get(row, 1) else {
                panic!("column 1 is an i64");
            };
            (name.to_owned(), amount)
        })
        .collect();
    rows.sort();
    rows
}

#[test]
fn usage_shapes_end_to_end() {
    let dir = test_dir("usage");
    let db = Db::create(&dir, schema()).expect("create");

    // Write: serial minting + typed inserts.
    let accounts = db
        .write(|tx| {
            let alice: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id: alice,
                name: "alice".to_owned(),
            })?;
            let bob: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id: bob,
                name: "bob".to_owned(),
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

    // Read: point lookup (guard probe), join, aggregate.
    let mut point = db.prepare(&point_query()).expect("prepare point");
    let mut join = db.prepare(&join_query()).expect("prepare join");
    let mut aggregate = db.prepare(&aggregate_query()).expect("prepare agg");
    db.read(|snap| {
        let rows = snap.execute_collect(&mut point, &[Value::U64(accounts[2].id.0)])?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows.get(0, 0), ResultValue::I64(40));

        let rows = snap.execute_collect(&mut join, &[])?;
        assert_eq!(
            name_amount_rows(&rows),
            vec![
                ("alice".to_owned(), -25),
                ("alice".to_owned(), 100),
                ("bob".to_owned(), 40),
            ]
        );

        let rows = snap.execute_collect(&mut aggregate, &[])?;
        assert_eq!(
            name_amount_rows(&rows),
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
        let rows = snap.execute_collect(&mut join, &[])?;
        assert_eq!(
            name_amount_rows(&rows),
            vec![
                ("alice".to_owned(), -25),
                ("alice".to_owned(), 90),
                ("bob".to_owned(), 40),
            ]
        );
        let (rows, report) = snap.explain(&mut join, &[])?;
        assert_eq!(rows.len(), 3);
        assert!(!report.is_empty(), "explain renders a report");
        Ok(())
    })
    .expect("read after mutate");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn aborted_writes_leave_prior_state_intact() {
    let dir = test_dir("abort");
    let db = Db::create(&dir, schema()).expect("create");
    db.write(|tx| {
        let id: HolderId = tx.alloc()?;
        tx.insert(&Holder {
            id,
            name: "keep".to_owned(),
        })
    })
    .expect("seed");

    // A panicking closure: the delta dies in the unwind, LMDB untouched.
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: bumbledb::Result<()> = db.write(|tx| {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id,
                name: "doomed-by-panic".to_owned(),
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
            name: "doomed-by-error".to_owned(),
        })?;
        Err(bumbledb::Error::Overflow { find: 0 })
    });
    assert!(failed.is_err());

    // The writer mutex is released and prior state intact.
    db.write(|tx| {
        let id: HolderId = tx.alloc()?;
        tx.insert(&Holder {
            id,
            name: "after".to_owned(),
        })
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

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn concurrent_readers_while_writing() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Db<'static>>();

    let dir = test_dir("threads");
    let db = Db::create(&dir, schema()).expect("create");
    // Seed one pair so readers always see data.
    db.write(|tx| {
        let holder: HolderId = tx.alloc()?;
        tx.insert(&Holder {
            id: holder,
            name: "seed".to_owned(),
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
                        name: format!("holder-{round}"),
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

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn export_scan_bulk_loads_into_a_fresh_database() {
    let dir_old = test_dir("etl-old");
    let dir_new = test_dir("etl-new");
    let old = Db::create(&dir_old, schema()).expect("create old");

    let max_holder = old
        .write(|tx| {
            let mut max = 0;
            for (name, balance) in [("alice", 100i64), ("bob", -7), ("carol", 40)] {
                let holder: HolderId = tx.alloc()?;
                tx.insert(&Holder {
                    id: holder,
                    name: name.to_owned(),
                })?;
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

    // Import: FK targets first; explicit serial values preserve identity.
    let new = Db::create(&dir_new, schema()).expect("create new");
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
    let rows_old = old
        .read(|snap| snap.execute_collect(&mut join_old, &[]))
        .expect("query old");
    let mut join_new = new.prepare(&join_query()).expect("prepare");
    let rows_new = new
        .read(|snap| snap.execute_collect(&mut join_new, &[]))
        .expect("query new");
    assert_eq!(name_amount_rows(&rows_old), name_amount_rows(&rows_new));

    // The serial high-water advanced past the explicit imports.
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

    drop(old);
    drop(new);
    let _ = std::fs::remove_dir_all(&dir_old);
    let _ = std::fs::remove_dir_all(&dir_new);
}
