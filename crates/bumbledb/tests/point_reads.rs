//! `WriteTx` point reads through the public surface
//! (`docs/architecture/70-api.md` § `WriteTx` point reads): `contains`/`get`
//! observe committed state overlaid with the pending delta — the
//! final-state view the judgment phase judges — so every pre-commit answer
//! equals the post-commit one, and the blessed upsert idiom is sound
//! within a single write transaction.

use bumbledb::Db;

mod common;

bumbledb::schema! {
    pub Ledger;

    relation Account {
        id: u64 as AccountId, fresh,
        holder: str,
        balance: i64,
    }
}

/// The read-your-writes matrix: insert → found; delete → gone; delete +
/// reinsert(modified) → the modified fact — all before commit, and all
/// equal to the post-commit answer (asserted through a fresh transaction
/// *and* a read-transaction scan).
#[test]
fn point_reads_observe_the_final_state_before_commit() {
    let dir = common::TempDir::new("points-read-your-writes");
    let db = Db::create(dir.path(), Ledger).expect("create");

    let id = db
        .write(|tx| {
            let id = tx.alloc::<AccountId>()?;
            let acct = Account {
                id,
                holder: "ada",
                balance: 10,
            };
            // Insert, then read back through the pending delta — the
            // holder string exists only as a provisional intern id here.
            assert!(tx.insert(&acct)?);
            assert!(tx.contains(&acct)?);
            assert_eq!(tx.get(id)?, Some(acct.clone()));
            // Delete: the final state no longer holds the fact.
            assert!(tx.delete(&acct)?);
            assert!(!tx.contains(&acct)?);
            assert_eq!(tx.get(id)?, None);
            // Delete + reinsert(modified): the key re-establishes with
            // the modified fact.
            let modified = Account {
                balance: 42,
                ..acct.clone()
            };
            assert!(tx.insert(&modified)?);
            assert!(tx.contains(&modified)?);
            assert!(!tx.contains(&acct)?);
            assert_eq!(tx.get(id)?, Some(modified));
            Ok(id)
        })
        .expect("write");

    // The post-commit point reads answer identically.
    let survivor = Account {
        id,
        holder: "ada",
        balance: 42,
    };
    db.write(|tx| {
        assert!(tx.contains(&survivor)?);
        assert!(!tx.contains(&Account {
            balance: 10,
            ..survivor.clone()
        })?);
        assert_eq!(tx.get(id)?, Some(survivor.clone()));
        Ok(())
    })
    .expect("post-commit point reads");

    // And the read-transaction view agrees fact-for-fact.
    db.read(|snap| {
        let facts: Vec<Account> = snap.scan_facts()?.collect::<bumbledb::Result<_>>()?;
        assert_eq!(facts, vec![survivor.clone()]);
        Ok(())
    })
    .expect("read");
}

/// Committed-state fallthrough: a fact committed in a prior transaction
/// and untouched in this delta is found through the committed view; a
/// never-interned string proves absence without touching the dictionary.
#[test]
fn point_reads_fall_through_to_committed_state() {
    let dir = common::TempDir::new("points-committed-fallthrough");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let id = db
        .write(|tx| {
            let id = tx.alloc::<AccountId>()?;
            tx.insert(&Account {
                id,
                holder: "seed",
                balance: 7,
            })?;
            Ok(id)
        })
        .expect("seed");

    db.write(|tx| {
        // Touch an unrelated fact so the delta is nonempty but the probed
        // key has no overlay.
        let other = tx.alloc::<AccountId>()?;
        tx.insert(&Account {
            id: other,
            holder: "other",
            balance: 1,
        })?;
        let seeded = Account {
            id,
            holder: "seed",
            balance: 7,
        };
        assert!(tx.contains(&seeded)?);
        assert_eq!(tx.get(id)?, Some(seeded));
        // A never-interned holder short-circuits to false — the fact
        // provably exists nowhere.
        assert!(!tx.contains(&Account {
            id: AccountId(999),
            holder: "ghost",
            balance: 0,
        })?);
        // An unallocated key misses cleanly.
        assert_eq!(tx.get(AccountId(999))?, None);
        Ok(())
    })
    .expect("fallthrough reads");
}

/// Regression: a compensating delete that *cancels* a pending insert nets
/// to nothing — the shared key tuple must keep answering with its
/// committed owner, typed and dynamic alike, and the blessed upsert idiom
/// composed after the cancelled pair takes the seen arm and commits
/// cleanly (the poisoned overlay used to deny the committed row and
/// steer the idiom into a spurious `CommitRejected`).
#[test]
fn a_cancelled_insert_never_shadows_the_committed_row() {
    let dir = common::TempDir::new("points-cancelled-insert");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let id = db
        .write(|tx| {
            let id = tx.alloc::<AccountId>()?;
            tx.insert(&Account {
                id,
                holder: "ada",
                balance: 10,
            })?;
            Ok(id)
        })
        .expect("seed");

    db.write(|tx| {
        // A pending insert on the committed key, then its compensating
        // delete: the pair cancels, net delta nothing.
        assert!(tx.insert(&Account {
            id,
            holder: "ada",
            balance: 20,
        })?);
        assert!(tx.delete(&Account {
            id,
            holder: "ada",
            balance: 20,
        })?);

        // Every point read answers exactly what a post-commit read would.
        let committed = Account {
            id,
            holder: "ada",
            balance: 10,
        };
        assert!(tx.contains(&committed)?);
        assert_eq!(tx.get(id)?, Some(committed));
        let row = tx.get_dyn(
            bumbledb::schema::RelationId(0),
            bumbledb::schema::StatementId(0),
            &[bumbledb::Value::U64(id.0)],
        )?;
        assert!(
            row.is_some(),
            "the dynamic point read sees the committed row"
        );

        // The upsert idiom takes the seen arm: delete + insert bumped.
        tx.delete(&Account {
            id,
            holder: "ada",
            balance: 10,
        })?;
        tx.insert(&Account {
            id,
            holder: "ada",
            balance: 11,
        })?;
        Ok(())
    })
    .expect("the composed upsert commits cleanly");

    db.read(|snap| {
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
        left: u64 as LeftId, fresh,
        right: u64 as RightId, fresh,
    }
    relation Tag {
        id: u64 as TagId, fresh,
        label: str,
    }
}

/// The single-fresh restriction is dead: a relation with SEVERAL fresh
/// fields reads through each key as its own Rust type — `LeftId` and
/// `RightId` carry distinct `Key::STATEMENT`s (the materialized order's
/// first block: relation declaration order, then field order), so which
/// FD a point read goes through is the key value's type, never a
/// runtime question. Both transaction kinds answer through both keys,
/// and a later relation's fresh key ordinal follows on.
#[test]
fn every_fresh_field_is_its_own_typed_key() {
    use bumbledb::Key;
    assert_eq!(<LeftId as Key>::STATEMENT, bumbledb::schema::StatementId(0));
    assert_eq!(
        <RightId as Key>::STATEMENT,
        bumbledb::schema::StatementId(1)
    );
    assert_eq!(<TagId as Key>::STATEMENT, bumbledb::schema::StatementId(2));

    let dir = common::TempDir::new("points-multi-fresh-keys");
    let db = Db::create(dir.path(), Registry).expect("create");
    let (left, right) = db
        .write(|tx| {
            let left = tx.alloc::<LeftId>()?;
            let right = tx.alloc::<RightId>()?;
            tx.insert(&Pair { left, right })?;
            assert_eq!(tx.get(left)?, Some(Pair { left, right }));
            assert_eq!(tx.get(right)?, Some(Pair { left, right }));
            Ok((left, right))
        })
        .expect("seed");
    db.read(|snap| {
        assert_eq!(snap.get(left)?, Some(Pair { left, right }));
        assert_eq!(snap.get(right)?, Some(Pair { left, right }));
        assert_eq!(snap.get(RightId(999))?, None);
        Ok(())
    })
    .expect("read");
}

/// `Snapshot::get` — the committed-state sibling of `WriteTx::get`,
/// through the same typed key value: a committed fact comes back from
/// the read scope (`db.read(|snap| snap.get(id))`), and an unallocated
/// id misses cleanly.
#[test]
fn snapshot_get_reads_committed_state_through_the_fresh_key() {
    let dir = common::TempDir::new("points-snapshot-get");
    let db = Db::create(dir.path(), Ledger).expect("create");
    let id = db
        .write(|tx| {
            let id = tx.alloc::<AccountId>()?;
            tx.insert(&Account {
                id,
                holder: "ada",
                balance: 7,
            })?;
            Ok(id)
        })
        .expect("seed");

    db.read(|snap| {
        assert_eq!(
            snap.get(id)?,
            Some(Account {
                id,
                holder: "ada",
                balance: 7,
            })
        );
        assert_eq!(snap.get(AccountId(999))?, None);
        Ok(())
    })
    .expect("read");
}

/// The blessed upsert idiom, as written in `70-api.md`: get → delete +
/// insert, or insert. The holder string comes back as a borrowed view of
/// the transaction, so ownership is an explicit host act — copy the
/// fields out before mutating the transaction again.
fn add(db: &Db<Ledger>, id: AccountId, x: i64) -> bumbledb::Result<()> {
    db.write(|tx| {
        let old = tx.get(id)?.map(|old| (old.holder.to_owned(), old.balance));
        match old {
            Some((holder, balance)) => {
                tx.delete(&Account {
                    id,
                    holder: &holder,
                    balance,
                })?;
                tx.insert(&Account {
                    id,
                    holder: &holder,
                    balance: balance + x,
                })?;
            }
            None => {
                tx.insert(&Account {
                    id,
                    holder: "counter",
                    balance: x,
                })?;
            }
        }
        Ok(())
    })
}

/// A counter increment round-trips across three write transactions: the
/// first inserts, the next two read-modify-write — exactly one fact
/// survives, carrying the sum.
#[test]
fn the_upsert_idiom_round_trips_a_counter_across_three_transactions() {
    let dir = common::TempDir::new("points-upsert-counter");
    let db = Db::create(dir.path(), Ledger).expect("create");
    // An explicit fresh value is legal on the write path; the high-water
    // mark advances past it.
    let id = AccountId(7);
    add(&db, id, 1).expect("first upsert inserts");
    add(&db, id, 10).expect("second upsert increments");
    add(&db, id, 100).expect("third upsert increments");

    db.read(|snap| {
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
