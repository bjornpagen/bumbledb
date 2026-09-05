use bumbledb::Db;

mod common;

bumbledb::schema! {
    pub Ledger;

    relation Account {
        id: u64 as AccountId,
        holder: str,
        balance: i64,
    }

    Account(id) -> Account;
}

/// The read-your-writes matrix: insert → found; delete → gone; delete +
/// reinsert(modified) → the modified fact — all before commit, and all equal to
/// the post-commit answer (asserted through a fresh transaction *and* a
/// read-transaction scan).
#[test]
fn point_reads_observe_the_final_state_before_commit() {
    let dir = common::TempDir::new("points-read-your-writes");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");

    let id = db
        .write(common::work(), |tx| {
            let id = AccountId(1);
            let acct = Account {
                id,
                holder: "ada",
                balance: 10,
            };

            assert_eq!(tx.insert([&acct])?.changed(), 1);
            assert!(tx.contains(&acct)?);
            assert_eq!(tx.get(AccountById { id })?, Some(acct));

            assert_eq!(tx.delete([&acct])?.changed(), 1);
            assert!(!tx.contains(&acct)?);
            assert_eq!(tx.get(AccountById { id })?, None);

            let modified = Account {
                balance: 42,
                ..acct
            };
            assert_eq!(tx.insert([&modified])?.changed(), 1);
            assert!(tx.contains(&modified)?);
            assert!(!tx.contains(&acct)?);
            assert_eq!(tx.get(AccountById { id })?, Some(modified));
            Ok(id)
        })
        .expect("write")
        .unwrap()
        .value;

    let survivor = Account {
        id,
        holder: "ada",
        balance: 42,
    };
    db.write(common::work(), |tx| {
        assert!(tx.contains(&survivor)?);
        assert!(!tx.contains(&Account {
            balance: 10,
            ..survivor
        })?);
        assert_eq!(tx.get(AccountById { id })?, Some(survivor));
        Ok(())
    })
    .expect("post-commit point reads")
    .unwrap();

    db.read(common::work(), |snap| {
        let facts: Vec<Account> = snap.scan_facts()?.collect::<bumbledb::Result<_>>()?;
        assert_eq!(facts, vec![survivor]);
        Ok(())
    })
    .expect("read");
}

#[test]
fn point_reads_fall_through_to_committed_state() {
    let dir = common::TempDir::new("points-committed-fallthrough");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let id = db
        .write(common::work(), |tx| {
            let id = AccountId(1);
            tx.insert([&Account {
                id,
                holder: "seed",
                balance: 7,
            }])?;
            Ok(id)
        })
        .expect("seed")
        .unwrap()
        .value;

    db.write(common::work(), |tx| {
        let other = AccountId(2);
        tx.insert([&Account {
            id: other,
            holder: "other",
            balance: 1,
        }])?;
        let seeded = Account {
            id,
            holder: "seed",
            balance: 7,
        };
        assert!(tx.contains(&seeded)?);
        assert_eq!(tx.get(AccountById { id })?, Some(seeded));

        assert!(!tx.contains(&Account {
            id: AccountId(999),
            holder: "ghost",
            balance: 0,
        })?);

        assert_eq!(tx.get(AccountById { id: AccountId(999) })?, None);
        Ok(())
    })
    .expect("fallthrough reads")
    .unwrap();
}

/// Regression: a compensating delete that *cancels* a pending insert nets to
/// nothing — the shared key tuple must keep answering with its committed owner,
/// typed and dynamic alike, and the blessed upsert idiom composed after the
/// cancelled pair takes the seen arm and commits cleanly (the poisoned overlay
/// used to deny the committed row and steer the idiom into a spurious
/// `Admission::Rejected`).
#[test]
fn a_cancelled_insert_never_shadows_the_committed_row() {
    let dir = common::TempDir::new("points-cancelled-insert");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let id = db
        .write(common::work(), |tx| {
            let id = AccountId(1);
            tx.insert([&Account {
                id,
                holder: "ada",
                balance: 10,
            }])?;
            Ok(id)
        })
        .expect("seed")
        .unwrap()
        .value;

    db.write(common::work(), |tx| {
        assert_eq!(
            tx.insert([&Account {
                id,
                holder: "ada",
                balance: 20,
            }])?
            .changed(),
            1
        );
        assert_eq!(
            tx.delete([&Account {
                id,
                holder: "ada",
                balance: 20,
            }])?
            .changed(),
            1
        );

        let committed = Account {
            id,
            holder: "ada",
            balance: 10,
        };
        assert!(tx.contains(&committed)?);
        assert_eq!(tx.get(AccountById { id })?, Some(committed));
        let row = tx.get_dyn(
            bumbledb::schema::RelationId(0),
            bumbledb::schema::StatementId(0),
            &[bumbledb::Value::U64(id.0)],
        )?;
        assert!(
            row.is_some(),
            "the dynamic point read sees the committed row"
        );

        tx.delete([&Account {
            id,
            holder: "ada",
            balance: 10,
        }])?;
        tx.insert([&Account {
            id,
            holder: "ada",
            balance: 11,
        }])?;
        Ok(())
    })
    .expect("the composed upsert commits cleanly")
    .unwrap();

    db.read(common::work(), |snap| {
        let facts: Vec<Account> = snap.scan_facts()?.collect::<bumbledb::Result<_>>()?;
        assert_eq!(
            facts,
            vec![Account {
                id,
                holder: "ada",
                balance: 11,
            }]
        );
        Ok(())
    })
    .expect("read");
}

bumbledb::schema! {
    pub Registry;

    relation Pair {
        left: u64 as LeftId,
        right: u64 as RightId,
    }
    relation Tag {
        id: u64 as TagId,
        label: str,
    }

    Pair(left) -> Pair;
    Pair(right) -> Pair;
    Tag(id) -> Tag;
}

#[test]
fn every_declared_key_is_its_own_typed_key() {
    use bumbledb::Key;
    assert_eq!(
        <PairByLeft as Key>::STATEMENT,
        bumbledb::schema::StatementId(0)
    );
    assert_eq!(
        <PairByRight as Key>::STATEMENT,
        bumbledb::schema::StatementId(1)
    );
    assert_eq!(
        <TagById as Key>::STATEMENT,
        bumbledb::schema::StatementId(2)
    );

    let dir = common::TempDir::new("points-multi-fresh-keys");
    let db = Db::create(dir.path(), Registry, common::work())
        .expect("create")
        .expect("accepted");
    let (left, right) = db
        .write(common::work(), |tx| {
            let left = LeftId(1);
            let right = RightId(1);
            tx.insert([&Pair { left, right }])?;
            assert_eq!(tx.get(PairByLeft { left })?, Some(Pair { left, right }));
            assert_eq!(tx.get(PairByRight { right })?, Some(Pair { left, right }));
            Ok((left, right))
        })
        .expect("seed")
        .unwrap()
        .value;
    db.read(common::work(), |snap| {
        assert_eq!(snap.get(PairByLeft { left })?, Some(Pair { left, right }));
        assert_eq!(snap.get(PairByRight { right })?, Some(Pair { left, right }));
        assert_eq!(
            snap.get(PairByRight {
                right: RightId(999)
            })?,
            None
        );
        Ok(())
    })
    .expect("read");
}

#[test]
fn snapshot_get_reads_committed_state_through_the_declared_key() {
    let dir = common::TempDir::new("points-snapshot-get");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let id = db
        .write(common::work(), |tx| {
            let id = AccountId(1);
            tx.insert([&Account {
                id,
                holder: "ada",
                balance: 7,
            }])?;
            Ok(id)
        })
        .expect("seed")
        .unwrap()
        .value;

    db.read(common::work(), |snap| {
        assert_eq!(
            snap.get(AccountById { id })?,
            Some(Account {
                id,
                holder: "ada",
                balance: 7,
            })
        );
        assert_eq!(snap.get(AccountById { id: AccountId(999) })?, None);
        Ok(())
    })
    .expect("read");
}

/// The holder string comes back as a borrowed view of the transaction, so
/// ownership is an explicit host act — copy the fields out before mutating the
/// transaction again.
fn add(db: &Db<Ledger>, id: AccountId, x: i64) -> bumbledb::Result<()> {
    db.write(common::work(), |tx| {
        let old = tx
            .get(AccountById { id })?
            .map(|old| (old.holder.to_owned(), old.balance));
        match old {
            Some((holder, balance)) => {
                tx.delete([&Account {
                    id,
                    holder: &holder,
                    balance,
                }])?;
                tx.insert([&Account {
                    id,
                    holder: &holder,
                    balance: balance + x,
                }])?;
            }
            None => {
                tx.insert([&Account {
                    id,
                    holder: "counter",
                    balance: x,
                }])?;
            }
        }
        Ok(())
    })?
    .unwrap();
    Ok(())
}

#[test]
fn the_upsert_idiom_round_trips_a_counter_across_three_transactions() {
    let dir = common::TempDir::new("points-upsert-counter");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");

    let id = AccountId(7);
    add(&db, id, 1).expect("first upsert inserts");
    add(&db, id, 10).expect("second upsert increments");
    add(&db, id, 100).expect("third upsert increments");

    db.read(common::work(), |snap| {
        let facts: Vec<Account> = snap.scan_facts()?.collect::<bumbledb::Result<_>>()?;
        assert_eq!(
            facts,
            vec![Account {
                id,
                holder: "counter",
                balance: 111,
            }]
        );
        Ok(())
    })
    .expect("read");
}

#[test]
fn snapshot_contains_answers_typed_membership_against_committed_state() {
    let dir = common::TempDir::new("points-snap-contains");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let id = db
        .write(common::work(), |tx| {
            let id = AccountId(1);
            tx.insert([&Account {
                id,
                holder: "ada",
                balance: 10,
            }])?;
            Ok(id)
        })
        .expect("write")
        .unwrap()
        .value;
    db.read(common::work(), |snap| {
        let committed = Account {
            id,
            holder: "ada",
            balance: 10,
        };
        assert!(snap.contains(&committed)?);
        assert!(!snap.contains(&Account {
            balance: 11,
            ..committed
        })?);
        assert!(!snap.contains(&Account {
            holder: "ghost",
            ..committed
        })?);
        Ok(())
    })
    .expect("read");
}

#[test]
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "the method-path form is not general enough over the snapshot lifetime (HRTB)"
)]
fn snapshot_generation_is_the_tx_id_witnessed_inside_the_snapshot() {
    let dir = common::TempDir::new("points-snap-generation");
    let db = Db::create(dir.path(), Ledger, common::work())
        .expect("create")
        .expect("accepted");
    let before = db.read(common::work(), |snap| snap.generation()).expect("read");
    let committed = db
        .write(common::work(), |tx| {
            let id = AccountId(1);
            tx.insert([&Account {
                id,
                holder: "ada",
                balance: 1,
            }])?;
            Ok(())
        })
        .expect("write")
        .unwrap();
    let after = db.read(common::work(), |snap| snap.generation()).expect("read");
    assert_eq!(after, committed.generation);
    assert_ne!(before, after);
}
