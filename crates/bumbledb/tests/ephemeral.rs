//! The ephemeral store kind (`docs/architecture/70-api.md` § environment
//! lifecycle, `50-storage.md` § the ephemeral store kind): the cross-open
//! matrix — every constructor × store-kind cell is a typed outcome, never
//! a silent durability change — and the durable/ephemeral differential
//! oracle: one deterministic ops sequence replayed against both kinds,
//! asserting identical commit verdicts, identical COMPLETE violation
//! sets, identical `WriteTx` point reads, and identical full relation
//! contents. The flag an ephemeral store carries (`NOSYNC`)
//! changes durability mechanism only; every semantic is shared.

use bumbledb::{Db, Error, Fact, Fresh, StoreKind, Value};

mod common;

bumbledb::schema! {
    pub Staging;

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

// ---------------------------------------------------------------------
// The cross-open matrix: {Db::open, Db::ephemeral} × {durable, ephemeral}
// ---------------------------------------------------------------------

/// Matrix cell 1: `Db::open` on a durable store — the ordinary reopen.
#[test]
fn open_on_a_durable_store_succeeds() {
    let dir = common::TempDir::new("ephemeral-open-durable");
    drop(Db::create(dir.path(), Staging).expect("create durable"));
    drop(Db::open(dir.path(), Staging).expect("open durable"));
}

/// Matrix cell 2: `Db::open` on an ephemeral store is the typed
/// refusal — the durable surface never quietly holds a store that
/// skipped its fsyncs.
#[test]
fn open_on_an_ephemeral_store_is_a_typed_store_kind_mismatch() {
    let dir = common::TempDir::new("ephemeral-open-ephemeral");
    drop(Db::ephemeral(dir.path(), Staging).expect("create ephemeral"));
    let err = Db::open(dir.path(), Staging)
        .err()
        .expect("open must refuse the kind");
    assert!(
        matches!(
            err,
            Error::StoreKindMismatch {
                found: StoreKind::Ephemeral,
                expected: StoreKind::Durable,
            }
        ),
        "{err:?}"
    );
}

/// Matrix cell 3: `Db::ephemeral` on a durable store is the typed
/// refusal — the ephemeral surface never quietly strips a durable
/// store's guarantee.
#[test]
fn ephemeral_on_a_durable_store_is_a_typed_store_kind_mismatch() {
    let dir = common::TempDir::new("ephemeral-ephemeral-durable");
    drop(Db::create(dir.path(), Staging).expect("create durable"));
    let err = Db::ephemeral(dir.path(), Staging)
        .err()
        .expect("ephemeral must refuse the kind");
    assert!(
        matches!(
            err,
            Error::StoreKindMismatch {
                found: StoreKind::Durable,
                expected: StoreKind::Ephemeral,
            }
        ),
        "{err:?}"
    );
}

/// Matrix cell 4: `Db::ephemeral` on an ephemeral store reopens it —
/// contents survive a clean process handoff (only machine-crash
/// durability is renounced), and the fingerprint check still guards.
#[test]
fn ephemeral_on_an_ephemeral_store_reopens_with_contents() {
    let dir = common::TempDir::new("ephemeral-reopen");
    let db = Db::ephemeral(dir.path(), Staging).expect("create ephemeral");
    let holder = db
        .write(|tx| {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder { id, name: "ada" })?;
            Ok(id)
        })
        .expect("commit");
    drop(db);
    let db = Db::ephemeral(dir.path(), Staging).expect("reopen ephemeral");
    db.write(|tx| {
        assert!(
            tx.contains(&Holder {
                id: holder,
                name: "ada",
            })?,
            "the ephemeral store's contents survive a clean reopen"
        );
        Ok(())
    })
    .expect("read back");
}

// GRAVESTONE — `ephemeral_open_allocates_the_full_map_eagerly` (the
// capacity contract's pin: `blocks() * 512 >= 4 << 30` at open and
// again at reopen — the eager-allocation mechanism assertion). DELETED,
// not retargeted, by cleanup-0.5.0 ruling 1 (the ephemeral-lazy
// unification, the retired U1 ephemeral-lazy packet, git history): the
// eager capacity contract it pinned is retired — one 32 GiB `MAP_SIZE`
// for both kinds, no `WRITE_MAP`, no open-time preallocation — so the
// pinned behavior no longer exists to assert. A 32 GiB eager assertion
// cannot run on a ~14 GB CI runner, and retargeting the 4 GiB bound
// downward is the forbidden weakening — DO NOT retarget this test.
// Laziness is now the contract (capacity refusal is the filesystem's
// own, at write time), and the refusal byte-identity tests below are
// the surviving pins of the open path.

/// Matrix cell 3's non-mutation lock (`docs/architecture/70-api.md`:
/// `Db::ephemeral` never destroys data — and never mutates on refusal):
/// the refusal on a durable store leaves `data.mdb` byte-identical.
/// A constructor that opened with the ephemeral flags BEFORE verifying
/// the kind would hold a store it must refuse (the fixit record: under
/// the retired `MDB_WRITEMAP` flag set that reopen permanently resized
/// the durable store); the probe-first open must leave the file's
/// length and bytes untouched, and the store must still open durable
/// with its contents intact.
#[test]
fn ephemeral_refusal_on_a_durable_store_leaves_the_data_file_byte_identical() {
    let dir = common::TempDir::new("ephemeral-refusal-no-mutation");
    let db = Db::create(dir.path(), Staging).expect("create durable");
    let holder = db
        .write(|tx| {
            let id: HolderId = tx.alloc()?;
            tx.insert(&Holder { id, name: "ada" })?;
            Ok(id)
        })
        .expect("commit one row");
    drop(db);

    let data = dir.path().join("data.mdb");
    let before = std::fs::read(&data).expect("read data.mdb before the refusal");
    // A real store's data file is orders of magnitude below the map —
    // the assertion below would be vacuous otherwise (and this bound is
    // itself the pin that no open path truncates a durable store's
    // data file to the full map).
    assert!(
        before.len() < 1 << 30,
        "fixture data file unexpectedly large: {} bytes",
        before.len()
    );

    let err = Db::ephemeral(dir.path(), Staging)
        .err()
        .expect("ephemeral must refuse the kind");
    assert!(
        matches!(
            err,
            Error::StoreKindMismatch {
                found: StoreKind::Durable,
                expected: StoreKind::Ephemeral,
            }
        ),
        "{err:?}"
    );

    let after = std::fs::read(&data).expect("read data.mdb after the refusal");
    assert_eq!(
        before.len(),
        after.len(),
        "the refusal changed data.mdb's length"
    );
    assert_eq!(before, after, "the refusal changed data.mdb's bytes");

    // The durable store is untouched in behavior too: open green, row intact.
    let db = Db::open(dir.path(), Staging).expect("the durable store still opens");
    db.write(|tx| {
        assert!(
            tx.contains(&Holder {
                id: holder,
                name: "ada",
            })?,
            "the durable store's contents survive the refused probe"
        );
        Ok(())
    })
    .expect("read back");
}

/// The timed-lane lock (`docs/architecture/60-validation.md` device
/// honesty; `70-api.md` § environment lifecycle): every timed bench lane
/// opens its target through `Db::open`
/// (`crates/bumbledb-bench/src/driver/bench.rs`), and `Db::open` on an
/// ephemeral store is the typed refusal above — so a timed lane
/// structurally CANNOT time an ephemeral store, and no lane carries a
/// kind check of its own. This test is that argument's pin.
#[test]
fn timed_lanes_structurally_cannot_time_an_ephemeral_store() {
    let dir = common::TempDir::new("ephemeral-timed-lane-lock");
    drop(Db::ephemeral(dir.path(), Staging).expect("create ephemeral"));
    assert!(matches!(
        Db::open(dir.path(), Staging),
        Err(Error::StoreKindMismatch { .. })
    ));
}

/// The matrix's create edge: `Db::create` on an ephemeral store refuses
/// as it refuses every initialized directory — re-initializing `_meta`
/// over live data is the corruption `AlreadyInitialized` exists for; the
/// kind never gets a say because create never reads a store at all.
#[test]
fn create_on_an_ephemeral_store_is_already_initialized() {
    let dir = common::TempDir::new("ephemeral-create-ephemeral");
    drop(Db::ephemeral(dir.path(), Staging).expect("create ephemeral"));
    let err = Db::create(dir.path(), Staging)
        .err()
        .expect("create must refuse");
    assert!(matches!(err, Error::AlreadyInitialized), "{err:?}");
}

// ---------------------------------------------------------------------
// The durable/ephemeral differential oracle
// ---------------------------------------------------------------------

/// One deterministic ops step's observable outcome on one store.
#[derive(Debug, PartialEq)]
enum StepOutcome {
    Committed {
        /// The `WriteTx::get` point read taken inside the NEXT write
        /// transaction's view (balance by account id) — the
        /// final-state-view semantics under comparison.
        point_read: Option<i64>,
    },
    Rejected {
        /// The COMPLETE violation set, rendered — `Violations` is the
        /// sealed payload on both stores and compares whole (order
        /// included) through its `PartialEq`.
        violations: bumbledb::Violations,
    },
}

/// Replays the fixed ops sequence against one store, recording each
/// step's outcome. The sequence exercises accept, key rejection
/// (functionality), containment rejection, and delete+insert mutation.
fn replay(db: &Db<Staging>) -> Vec<StepOutcome> {
    let mut outcomes = Vec::new();
    let mut step = |result: Result<(), Error>, probe: u64| {
        let outcome = match result {
            Ok(()) => StepOutcome::Committed {
                point_read: db
                    .write(|tx| {
                        Ok(tx
                            .get(AccountId::from_fresh(probe))?
                            .map(|account| account.balance))
                    })
                    .expect("the point-read probe transaction commits"),
            },
            Err(Error::CommitRejected { violations }) => StepOutcome::Rejected { violations },
            Err(other) => panic!("only the judgment may reject the sequence: {other:?}"),
        };
        outcomes.push(outcome);
    };

    // Step 1 (accepted): two holders, two accounts.
    step(
        db.write(|tx| {
            for (holder, name, account, balance) in [(1, "ada", 10, 100), (2, "bob", 20, 250)] {
                tx.insert(&Holder {
                    id: HolderId::from_fresh(holder),
                    name,
                })?;
                tx.insert(&Account {
                    id: AccountId::from_fresh(account),
                    holder: HolderId::from_fresh(holder),
                    balance,
                })?;
            }
            Ok(())
        }),
        10,
    );
    // Step 2 (key violation): two facts, one account id.
    step(
        db.write(|tx| {
            tx.insert(&Account {
                id: AccountId::from_fresh(30),
                holder: HolderId::from_fresh(1),
                balance: 1,
            })?;
            tx.insert(&Account {
                id: AccountId::from_fresh(30),
                holder: HolderId::from_fresh(2),
                balance: 2,
            })?;
            Ok(())
        }),
        30,
    );
    // Step 3 (containment violation): an account of a holder that
    // does not exist.
    step(
        db.write(|tx| {
            tx.insert(&Account {
                id: AccountId::from_fresh(40),
                holder: HolderId::from_fresh(9),
                balance: 9,
            })?;
            Ok(())
        }),
        40,
    );
    // Step 4 (accepted mutation): the blessed delete+insert idiom.
    step(
        db.write(|tx| {
            tx.delete(&Account {
                id: AccountId::from_fresh(10),
                holder: HolderId::from_fresh(1),
                balance: 100,
            })?;
            tx.insert(&Account {
                id: AccountId::from_fresh(10),
                holder: HolderId::from_fresh(1),
                balance: 175,
            })?;
            Ok(())
        }),
        10,
    );
    outcomes
}

/// Full contents of both ordinary relations, sorted for comparison.
fn contents(db: &Db<Staging>) -> Vec<Vec<Value>> {
    let mut rows: Vec<Vec<Value>> = db
        .read(|snap| {
            let mut rows: Vec<Vec<Value>> =
                snap.scan(Holder::RELATION)?.collect::<Result<_, _>>()?;
            rows.extend(
                snap.scan(Account::RELATION)?
                    .collect::<Result<Vec<_>, _>>()?,
            );
            Ok(rows)
        })
        .expect("scan");
    rows.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    rows
}

/// The cheap oracle (`docs/architecture/60-validation.md` § the ramdisk
/// sanction and the ephemeral oracle): the same deterministic ops
/// sequence against a durable and an ephemeral store yields identical
/// verdicts, identical complete violation sets, identical point reads,
/// and identical full relation contents — the flags change the
/// durability mechanism, never a semantic.
#[test]
fn the_same_ops_sequence_judges_identically_on_durable_and_ephemeral_stores() {
    let durable_dir = common::TempDir::new("ephemeral-differential-durable");
    let ephemeral_dir = common::TempDir::new("ephemeral-differential-ephemeral");
    let durable = Db::create(durable_dir.path(), Staging).expect("create durable");
    let ephemeral = Db::ephemeral(ephemeral_dir.path(), Staging).expect("create ephemeral");

    let durable_outcomes = replay(&durable);
    let ephemeral_outcomes = replay(&ephemeral);
    assert_eq!(
        durable_outcomes, ephemeral_outcomes,
        "verdicts, violation sets, and point reads diverge across store kinds"
    );
    // The sequence exercised both verdict polarities.
    assert!(
        durable_outcomes
            .iter()
            .any(|o| matches!(o, StepOutcome::Rejected { .. }))
            && durable_outcomes
                .iter()
                .any(|o| matches!(o, StepOutcome::Committed { .. })),
        "the oracle's sequence is degenerate: {durable_outcomes:?}"
    );

    assert_eq!(
        contents(&durable),
        contents(&ephemeral),
        "full relation contents diverge across store kinds"
    );
}
