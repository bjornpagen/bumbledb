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

/// The magnitude-first cover choice (docs/perf/06), end to end: the
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

/// Compaction (docs/perf/09): a chunk-churned store copies to a
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
