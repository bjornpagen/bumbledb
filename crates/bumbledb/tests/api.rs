//! The 60-api doc integration tests: the `60-api.md` usage shapes end to end
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

#[test]
fn constraint_violations_surface_from_commit_through_the_public_api() {
    let dir = test_dir("violations");
    let db = Db::create(&dir, schema()).expect("create");
    let holder = db
        .write(|tx| {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id,
                name: "alice".to_owned(),
            })?;
            Ok(id)
        })
        .expect("seed");

    // Unique violation: two live accounts claiming one serial id. The
    // error carries relation + constraint ids and the offending fact
    // bytes, and the WHOLE transaction aborts (the good insert too).
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
    let bumbledb::Error::UniqueViolation {
        relation,
        fact_bytes,
        ..
    } = err
    else {
        panic!("expected UniqueViolation, got {err}");
    };
    assert_eq!(relation, Account::RELATION);
    assert!(!fact_bytes.is_empty());
    let count = db
        .read(|snap| Ok(snap.scan_facts::<Account>()?.count()))
        .expect("scan");
    assert_eq!(count, 0, "the aborted transaction left nothing");

    // Forward FK violation: the fact bytes name the offender.
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
        err,
        bumbledb::Error::ForeignKeyViolation {
            violation: bumbledb::error::FkViolation::MissingTarget { .. },
            ..
        }
    ));

    // Restrict: deleting a referenced holder names the referrer by fact.
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
                name: "alice".to_owned(),
            })
        })
        .unwrap_err();
    let bumbledb::Error::ForeignKeyViolation {
        violation: bumbledb::error::FkViolation::RemainingReference { fact_bytes, .. },
        ..
    } = err
    else {
        panic!("expected RemainingReference, got {err}");
    };
    assert!(!fact_bytes.is_empty(), "the referrer is named by its fact");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn open_mismatches_and_snapshot_usability() {
    let dir = test_dir("open-mismatch");
    drop(Db::create(&dir, schema()).expect("create"));

    // Db-level mismatch: a different schema refuses to open.
    let other = bumbledb::schema::SchemaDescriptor {
        relations: vec![bumbledb::schema::RelationDescriptor {
            name: "Other".into(),
            fields: vec![bumbledb::schema::FieldDescriptor {
                name: "x".into(),
                value_type: bumbledb::schema::ValueType::U64,
                generation: bumbledb::schema::Generation::None,
            }],
            constraints: vec![],
        }],
    }
    .validate()
    .expect("valid");
    let Err(err) = Db::open(&dir, &other).map(|_| ()) else {
        panic!("a different schema must refuse to open");
    };
    assert!(matches!(err, bumbledb::Error::SchemaMismatch { .. }));

    // Create-over-existing refuses at the Db level too.
    let Err(err) = Db::create(&dir, schema()).map(|_| ()) else {
        panic!("create over an existing environment must refuse");
    };
    assert!(matches!(err, bumbledb::Error::AlreadyInitialized));

    // A failed execute leaves the snapshot usable, and the caller-buffer
    // path works through the public surface.
    let db = Db::open(&dir, schema()).expect("open");
    db.write(|tx| {
        let id: HolderId = tx.alloc()?;
        tx.insert(&Holder {
            id,
            name: "bo".to_owned(),
        })
    })
    .expect("seed");
    let mut join = db.prepare(&join_query()).expect("prepare");
    db.read(|snap| {
        let mut out = ResultBuffer::new();
        // Wrong param count: a typed error...
        let err = snap
            .execute(&mut join, &[Value::U64(1)], &mut out)
            .unwrap_err();
        assert!(matches!(err, bumbledb::Error::ParamCountMismatch { .. }));
        // ...and the same snapshot executes fine afterwards.
        snap.execute(&mut join, &[], &mut out)?;
        assert_eq!(out.len(), 0, "no accounts yet");
        Ok(())
    })
    .expect("snapshot stays usable");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pinned_snapshot_reads_its_generation_across_later_commits() {
    let dir = test_dir("pinned");
    let db = Db::create(&dir, schema()).expect("create");
    db.write(|tx| {
        let id: HolderId = tx.alloc()?;
        tx.insert(&Holder {
            id,
            name: "first".to_owned(),
        })
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
                    name: format!("later-{round}"),
                })
            })?;
        }
        // The pinned snapshot still answers at its own generation.
        assert_eq!(snap.scan_facts::<Holder>()?.count(), 1);
        let rows = snap.execute_collect(&mut join, &[])?;
        assert_eq!(rows.len(), 0);
        Ok(())
    })
    .expect("pinned read");

    // A fresh snapshot sees all three.
    let after = db
        .read(|snap| Ok(snap.scan_facts::<Holder>()?.count()))
        .expect("fresh read");
    assert_eq!(after, 3);

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bulk_load_equals_sequential_inserts_and_survives_chunks() {
    let dir_bulk = test_dir("bulk-a");
    let dir_seq = test_dir("bulk-b");
    let bulk = Db::create(&dir_bulk, schema()).expect("create");
    let seq = Db::create(&dir_seq, schema()).expect("create");

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
    // — relations are sets, so the comparison sorts by the serial id.)
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
    let dir_fail = test_dir("bulk-fail");
    let fail = Db::create(&dir_fail, schema()).expect("create");
    let mut bad = facts;
    bad[4_099] = vec![Value::U64(0)]; // arity mismatch in the second chunk
    let err = fail.bulk_load(Holder::RELATION, bad).unwrap_err();
    assert_eq!(err.committed, 4_096, "the complete first chunk persisted");
    assert!(matches!(err.error, bumbledb::Error::FactShape(_)));
    let persisted = fail
        .read(|snap| Ok(snap.scan_facts::<Holder>()?.count()))
        .expect("scan");
    assert_eq!(persisted, 4_096);

    drop((bulk, seq, fail));
    for d in [dir_bulk, dir_seq, dir_fail] {
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[test]
fn disk_size_and_generation_report_store_state() {
    let dir = test_dir("disk-size");
    let db = Db::create(&dir, schema()).expect("create");
    let empty = db.disk_size().expect("size");
    assert!(empty > 0, "a fresh environment still has pages");
    assert_eq!(db.generation().expect("gen"), 0);

    db.write(|tx| {
        for _ in 0..10_000u64 {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id,
                name: format!("holder-{}", id.0),
            })?;
        }
        Ok(())
    })
    .expect("bulk write");
    let grown = db.disk_size().expect("size");
    assert!(grown > empty, "10k facts grow the file: {empty} -> {grown}");
    assert_eq!(db.generation().expect("gen"), 1);

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The magnitude-first cover choice (docs/architecture/30-execution.md), end to end: the
/// balance shape — a big relation joined to a param-selected small side
/// — must iterate the selected side (7 keys) and probe the big one,
/// never the reverse. Work is pinned by counters, not wall clock.
#[test]
fn cover_choice_iterates_the_selected_side() {
    use bumbledb::ir::{AggOp, Atom, FindTerm, ParamId, Query, Term, Value, VarId};

    let dir = test_dir("cover-choice");
    let db = Db::create(&dir, schema()).expect("create");
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
            tx.insert(&Holder { id, name })?;
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
    let query = Query {
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
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: Holder::RELATION,
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Param(ParamId(0))),
                ],
            },
        ],
        predicates: vec![],
    };
    let mut prepared = db.prepare(&query).expect("prepare");
    let params = vec![Value::String(Box::from(&b"target"[..]))];
    let (out, stats) = db
        .read(|snap| snap.profile(&mut prepared, &params))
        .expect("profile");
    assert_eq!(out.len(), 7, "one group per target holder");
    assert_eq!(stats.emits, 140, "20 accounts x 7 holders reach the sink");

    // The join-variable node iterates the 7-key selected side...
    let batch_entries: Vec<u64> = stats.nodes.iter().map(|n| n.batch_entries).collect();
    assert!(
        batch_entries.contains(&7),
        "the cover is the selected side: {stats:?}"
    );
    // ...and total drawn entries are O(selected), never O(relation).
    let total: u64 = batch_entries.iter().sum();
    assert_eq!(total, 147, "7 holder keys + 140 account entries: {stats:?}");
}

/// Compaction (docs/architecture/40-storage.md): a chunk-churned store copies to a
/// substantially smaller, byte-identical, fully writable sibling — and
/// never clobbers an existing destination.
#[test]
fn compaction_drops_the_freelist_and_preserves_content() {
    use bumbledb::ir::Value;

    let dir = test_dir("compact");
    let source_dir = dir.join("source");
    let db = Db::create(&source_dir, schema()).expect("create");
    // Many small commits grow a real freelist through CoW churn.
    for round in 0..40u64 {
        db.write(|tx| {
            for i in 0..250u64 {
                let id: HolderId = tx.alloc()?;
                tx.insert(&Holder {
                    id,
                    name: format!("h{round}-{i}"),
                })?;
            }
            Ok(())
        })
        .expect("commit");
    }
    let source_size = db.disk_size().expect("size");
    let generation = db.generation().expect("generation");
    let scan_digest = |db: &Db<'_>| -> Vec<Vec<Value>> {
        let mut rows: Vec<Vec<Value>> = db
            .read(|snap| snap.scan(Holder::RELATION)?.collect::<Result<_, _>>())
            .expect("scan");
        rows.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        rows
    };
    let source_rows = scan_digest(&db);

    let compact_dir = dir.join("compacted");
    db.compact(&compact_dir).expect("compact");
    // Never clobbers.
    let err = db.compact(&compact_dir).expect_err("must refuse");
    assert!(matches!(err, bumbledb::Error::Io(_)), "{err:?}");
    drop(db);

    let compacted = Db::open(&compact_dir, schema()).expect("open compacted");
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
                name: "post-compaction".to_owned(),
            })
        })
        .expect("write");
    assert_eq!(
        scan_digest(&compacted).len(),
        source_rows.len() + 1,
        "the compacted store keeps living"
    );
}

/// PRD 00 (docs/hardening): the audit's CRITICAL repro, verbatim — a
/// prepared query executes only against snapshots of the database that
/// prepared it. Before the environment-instance check, executing A's
/// prepared query against B (same schema, same generation) returned B's
/// data through A's memo keys.
#[test]
fn a_prepared_query_refuses_a_foreign_snapshot() {
    let dir_a = test_dir("foreign-prepared-a");
    let dir_b = test_dir("foreign-prepared-b");
    let db_a = Db::create(&dir_a, schema()).expect("create a");
    let db_b = Db::create(&dir_b, schema()).expect("create b");
    for (db, name, balance) in [(&db_a, "alice", 10), (&db_b, "bob", 20)] {
        db.write(|tx| {
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
            })
        })
        .expect("seed one distinct fact pair");
    }
    assert_eq!(db_a.generation().expect("gen a"), 1);
    assert_eq!(db_b.generation().expect("gen b"), 1, "both clocks read 1");

    let mut prepared = db_a.prepare(&join_query()).expect("prepare on A");
    db_a.read(|snap| {
        let out = snap.execute_collect(&mut prepared, &[])?;
        assert_eq!(name_amount_rows(&out), vec![("alice".to_owned(), 10)]);
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
        let mut out = ResultBuffer::new();
        let err = snap.execute(&mut prepared, &[], &mut out).unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::ForeignPreparedQuery),
            "{err:?}"
        );
        let err = snap.explain(&mut prepared, &[]).unwrap_err();
        assert!(
            matches!(err, bumbledb::Error::ForeignPreparedQuery),
            "{err:?}"
        );
        let err = snap.profile(&mut prepared, &[]).unwrap_err();
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
        assert_eq!(name_amount_rows(&out), vec![("alice".to_owned(), 10)]);
        Ok(())
    })
    .expect("A unaffected");

    drop(db_a);
    drop(db_b);
    let _ = std::fs::remove_dir_all(&dir_a);
    let _ = std::fs::remove_dir_all(&dir_b);
}

/// PRD 00: the wipe-and-recreate variant — same path, new environment,
/// new identity. The old prepared query is foreign to the recreated
/// store even though every byte of the path matches.
#[test]
fn a_recreated_store_is_foreign_to_old_prepared_queries() {
    let dir = test_dir("foreign-recreate");
    let db = Db::create(&dir, schema()).expect("create");
    db.write(|tx| {
        let holder: HolderId = tx.alloc()?;
        tx.insert(&Holder {
            id: holder,
            name: "original".to_owned(),
        })?;
        let id: AccountId = tx.alloc()?;
        tx.insert(&Account {
            id,
            holder,
            balance: 1,
        })
    })
    .expect("seed");
    let mut prepared = db.prepare(&join_query()).expect("prepare");
    drop(db);

    std::fs::remove_dir_all(&dir).expect("wipe");
    let recreated = Db::create(&dir, schema()).expect("recreate at the same path");
    recreated
        .read(|snap| {
            let err = snap.execute_collect(&mut prepared, &[]).unwrap_err();
            assert!(
                matches!(err, bumbledb::Error::ForeignPreparedQuery),
                "{err:?}"
            );
            Ok(())
        })
        .expect("read");

    drop(recreated);
    let _ = std::fs::remove_dir_all(&dir);
}

/// PRD 00: the advisory lock — a second live handle on the same path is
/// a loud open-time error; dropping the first releases it.
#[test]
fn a_second_handle_on_a_live_path_is_locked_out() {
    let dir = test_dir("env-lock");
    let db = Db::create(&dir, schema()).expect("create");
    let err = Db::open(&dir, schema()).map(|_| ()).unwrap_err();
    assert!(matches!(err, bumbledb::Error::EnvironmentLocked), "{err:?}");
    let err = Db::create(&dir, schema()).map(|_| ()).unwrap_err();
    assert!(
        matches!(err, bumbledb::Error::EnvironmentLocked),
        "create is locked out before it can even refuse: {err:?}"
    );
    drop(db);
    let reopened = Db::open(&dir, schema()).expect("the lock died with the handle");
    drop(reopened);
    let _ = std::fs::remove_dir_all(&dir);
}

/// PRD 00: `create` refuses a directory holding someone else's LMDB
/// environment (named databases, no `_meta`), while the half-created
/// bumbledb recovery case — an empty root — still proceeds.
#[test]
#[allow(unsafe_code)]
fn create_refuses_a_foreign_lmdb_environment() {
    let dir = test_dir("env-foreign-lmdb");
    {
        // SAFETY: this test environment is opened once, in this scope.
        let env = unsafe {
            heed::EnvOpenOptions::new()
                .max_dbs(2)
                .open(&dir)
                .expect("raw lmdb env")
        };
        let mut wtxn = env.write_txn().expect("txn");
        let db: heed::Database<heed::types::Bytes, heed::types::Bytes> = env
            .create_database(&mut wtxn, Some("someone_elses_table"))
            .expect("foreign named db");
        db.put(&mut wtxn, b"k", b"v").expect("put");
        wtxn.commit().expect("commit");
    }
    let err = Db::create(&dir, schema()).map(|_| ()).unwrap_err();
    assert!(
        matches!(err, bumbledb::Error::AlreadyInitialized),
        "{err:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);

    // The recovery case: an LMDB file with an empty root (exactly what a
    // crash between directory creation and the meta commit leaves).
    let dir = test_dir("env-half-created");
    {
        // SAFETY: as above.
        let env = unsafe {
            heed::EnvOpenOptions::new()
                .max_dbs(2)
                .open(&dir)
                .expect("raw lmdb env")
        };
        let wtxn = env.write_txn().expect("txn");
        wtxn.commit().expect("commit nothing");
    }
    let db = Db::create(&dir, schema()).expect("an empty root is recoverable");
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// PRD 00: `Db::write` is non-reentrant — a nested call on the same
/// thread panics with the named message instead of deadlocking forever,
/// and the guard clears for the next (sequential) write.
#[test]
fn nested_write_panics_instead_of_deadlocking() {
    let dir = test_dir("nested-write");
    let db = Db::create(&dir, schema()).expect("create");
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

    // Sequential writes on the same thread still work: the guard cleared.
    db.write(|tx| {
        let id: HolderId = tx.alloc()?;
        tx.insert(&Holder {
            id,
            name: "after the panic".to_owned(),
        })
    })
    .expect("the writer survives");
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// PRD 00 (the audit's requested concurrency family): prepared queries on
/// reader threads race a writer that moves two facts together every
/// commit. Every execution must observe both rows at one generation —
/// equal balances, always — never a torn mix of two generations.
#[test]
fn prepared_executions_observe_exactly_one_generation() {
    let dir = test_dir("gen-atomic");
    let db = Db::create(&dir, schema()).expect("create");
    let (hx, hy, ax, ay) = db
        .write(|tx| {
            let hx: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id: hx,
                name: "x".to_owned(),
            })?;
            let hy: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id: hy,
                name: "y".to_owned(),
            })?;
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
                let mut out = ResultBuffer::new();
                for _ in 0..80 {
                    db.read(|snap| {
                        snap.execute(&mut prepared, &[], &mut out)?;
                        let rows = name_amount_rows(&out);
                        assert_eq!(rows.len(), 2, "both facts, always: {rows:?}");
                        assert_eq!(
                            rows[0].1, rows[1].1,
                            "a torn read mixed two generations: {rows:?}"
                        );
                        Ok(())
                    })
                    .expect("consistent execution");
                }
            });
        }
        writer.join().expect("writer thread");
    });

    let _ = std::fs::remove_dir_all(&dir);
}

/// PRD 01 (docs/hardening): a *successful* commit persists every serial
/// value it issued, even when no facts changed — an id the closure
/// returned to the host is never re-issued. Both no-op shapes: the
/// empty delta (alloc, nothing else) and the nets-to-nothing delta
/// (insert then delete of the same absent fact). The generation must
/// not move for either — `Q` marks are not query-visible state.
#[test]
#[allow(clippy::redundant_closure_for_method_calls)] // HRTB: the method path does not unify
fn escaped_serials_survive_noop_commits() {
    let dir = test_dir("serial-escape");
    let db = Db::create(&dir, schema()).expect("create");

    // The empty-delta path.
    let a: HolderId = db.write(|tx| tx.alloc()).expect("bare alloc");
    let generation_after_a = db.generation().expect("generation");
    let b: HolderId = db
        .write(|tx| {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id,
                name: "first real holder".to_owned(),
            })?;
            Ok(id)
        })
        .expect("real write");
    assert!(b.0 > a.0, "escaped id {a:?} re-issued as {b:?}");

    // The nets-to-nothing path (`changed: false`, non-empty delta).
    let c: HolderId = db
        .write(|tx| {
            let id: HolderId = tx.alloc()?;
            let ghost = Holder {
                id,
                name: "ghost".to_owned(),
            };
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
                name: "second real holder".to_owned(),
            })?;
            Ok(id)
        })
        .expect("real write");
    assert!(d.0 > c.0, "escaped id {c:?} re-issued as {d:?}");

    // Neither no-op moved the generation: Q marks are write-path
    // bookkeeping, not query-visible state.
    assert_eq!(generation_after_a, 0, "a bare alloc is not a state change");
    assert_eq!(
        generation_after_c, 1,
        "a nets-to-nothing write is not a state change"
    );

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// PRD 01: deleting a fact whose string was never interned is a proven
/// no-op — the fact's bytes would embed an id that was never minted —
/// and the dictionary does not grow. A later insert of that value must
/// still treat it as novel (both engine-visible effects of not minting).
#[test]
fn deleting_a_never_interned_string_is_a_mint_free_noop() {
    let dir = test_dir("mint-free-delete");
    let db = Db::create(&dir, schema()).expect("create");
    let holder = db
        .write(|tx| {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder {
                id,
                name: "real".to_owned(),
            })?;
            Ok(id)
        })
        .expect("seed");

    // Typed delete of a never-interned name: changed = false, and the
    // whole write is a no-op commit (generation unmoved).
    let generation = db.generation().expect("generation");
    db.write(|tx| {
        let changed = tx.delete(&Holder {
            id: holder,
            name: "never interned".to_owned(),
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
            name: "transient".to_owned(),
        };
        assert!(tx.insert(&transient)?);
        assert!(tx.delete(&transient)?);
        Ok(())
    })
    .expect("cancel");
    let names: Vec<String> = db
        .read(|snap| {
            snap.scan_facts::<Holder>()?
                .map(|h| h.map(|h| h.name))
                .collect::<bumbledb::Result<Vec<_>>>()
        })
        .expect("scan");
    assert_eq!(names, vec!["real".to_owned()]);

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
