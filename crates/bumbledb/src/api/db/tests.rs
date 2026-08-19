use super::*;
use crate::error::{
    ConditionalWrite, DynIdError, Error, FactShapeError, FindIndex, LmdbFailure, Mismatch,
};
use crate::ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, Value, VarId};
use crate::testutil::TempDir;
use bumbledb_theory::schema::{
    FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, StatementDescriptor,
    StatementId, ValueType,
};

/// Named(name str) — a string-carrying relation for dictionary tests.
fn named_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Named".into(),
            fields: vec![FieldDescriptor {
                name: "name".into(),
                value_type: ValueType::String,
                generation: Generation::None,
            }],
        }],
        statements: vec![],
    }
}

/// The reader cache, semantics pinned:
/// (a) a commit between reads is visible to the next read (the
///     parked snapshot is invalidated by the commit sequence);
/// (b) reads with no intervening commit reuse the parked snapshot
///     (observable: the LMDB generation is identical);
/// (c) an erroring read closure leaves the cache serviceable;
/// (d) 10,000 reads neither grow the reader table nor leak (probed
///     by the reads simply succeeding — LMDB's table is 126 slots
///     by default, so slot leakage fails loudly well before 10k).
#[test]
fn the_reader_cache_is_invisible_except_in_speed() {
    let dir = TempDir::new("db-reader-cache");
    let db = Db::create(dir.path(), named_schema())
        .expect("create")
        .expect("accepted");
    let named = RelationId(0);
    let count_named = |snap: &ReadInstance<'_, SchemaDescriptor>| -> Result<u64> {
        let mut n = 0;
        for row in snap.scan(named)? {
            row?;
            n += 1;
        }
        Ok(n)
    };

    // (a) write-between-reads visibility.
    let before = db.read(|snap| count_named(snap)).expect("read");
    assert_eq!(before, 0);
    db.write(|tx| {
        tx.insert_dyn(named, [&[Value::String("first".into())]])
            .map(|_| ())
    })
    .expect("write")
    .unwrap();
    let after = db.read(|snap| count_named(snap)).expect("read");
    assert_eq!(after, 1, "the commit is visible to the very next read");

    // (b) no intervening commit: the generation is snapshot-identical
    // (the parked reader IS the same snapshot).
    let g1 = db.read(|snap| snap.txn.generation()).expect("read");
    let g2 = db.read(|snap| snap.txn.generation()).expect("read");
    assert_eq!(g1, g2, "parked reuse serves the same snapshot");

    // (c) an erroring closure leaves the cache serviceable.
    let err: Result<()> = db.read(|_| {
        Err(crate::error::Error::Overflow(
            crate::error::OverflowKind::Aggregate { find: FindIndex(7) },
        ))
    });
    assert!(err.is_err());
    let again = db.read(|snap| count_named(snap)).expect("read after error");
    assert_eq!(again, 1);

    // (d) reader-table hygiene under 10k reads interleaved with
    // writes (every write invalidates; every read re-parks).
    for i in 0..100u64 {
        db.write(|tx| {
            tx.insert_dyn(named, [&[Value::String(format!("n{i}").into())]])
                .map(|_| ())
        })
        .expect("write")
        .unwrap();
        for _ in 0..100 {
            db.read(|snap| count_named(snap)).expect("read");
        }
    }
    let total = db.read(|snap| count_named(snap)).expect("read");
    assert_eq!(total, 101);
}

fn dict_entries<S>(db: &Db<S>) -> u64 {
    let rtxn = db.env.read_txn().expect("txn");
    db.env.dict().len(rtxn.raw()).expect("len")
}

/// The delete path never mints — a typo'd
/// delete leaves `_dict` byte-identical, at the storage level.
#[test]
fn a_typo_delete_leaves_the_dictionary_unchanged() {
    let dir = TempDir::new("db-mint-free-dict");
    let db = Db::create(dir.path(), named_schema())
        .expect("create")
        .expect("accepted");
    let named = RelationId(0);
    db.write(|tx| {
        tx.insert_dyn(named, [&[Value::String("real".into())]])
            .map(|_| ())
    })
    .expect("seed")
    .unwrap();
    let entries = dict_entries(&db);
    assert_eq!(entries, 2, "one value: forward + reverse entries");

    db.write(|tx| {
        let changed = tx.delete_dyn(named, [&[Value::String("ghost".into())]])?;
        assert_eq!(
            changed.changed(),
            0,
            "a never-interned value matches no fact"
        );
        Ok(())
    })
    .expect("typo delete")
    .unwrap();
    assert_eq!(dict_entries(&db), entries, "the dictionary grew on a miss");

    // Deleting the real fact still works — the committed-dict arm.
    db.write(|tx| {
        let changed = tx.delete_dyn(named, [&[Value::String("real".into())]])?;
        assert_eq!(changed.changed(), 1);
        Ok(())
    })
    .expect("real delete")
    .unwrap();
}

/// Entry(name str, amount i64) with `Entry(name) -> Entry` — a
/// string-keyed relation for the dynamic point reads. The declared key is
/// the schema's only statement: StatementId(0).
fn entry_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Entry".into(),
            fields: vec![
                FieldDescriptor {
                    name: "name".into(),
                    value_type: ValueType::String,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "amount".into(),
                    value_type: ValueType::I64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }],
    }
}

const ENTRY: RelationId = RelationId(0);
const ENTRY_KEY: StatementId = StatementId(0);

fn entry(name: &str, amount: i64) -> Vec<Value> {
    vec![Value::String(name.into()), Value::I64(amount)]
}

/// The dynamic read-your-writes matrix: every pre-commit `get_dyn` answer
/// equals the post-commit one (the final-state view the judgment phase
/// judges — `70-api.md`), including for a fact whose string key was
/// interned in this very transaction.
#[test]
fn get_dyn_reads_its_own_writes_exactly_as_a_later_transaction_does() {
    let dir = TempDir::new("db-get-dyn-ryw");
    let db = Db::create(dir.path(), entry_schema())
        .expect("create")
        .expect("accepted");

    db.write(|tx| {
        // Insert, then read back through the pending delta (the key
        // string exists only as a provisional intern id here).
        assert_eq!(tx.insert_dyn(ENTRY, [&entry("a", 1)])?.changed(), 1);
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".into())])?,
            Some(entry("a", 1))
        );
        // Delete: the determinant map records absence.
        assert_eq!(tx.delete_dyn(ENTRY, [&entry("a", 1)])?.changed(), 1);
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".into())])?,
            None
        );
        // Delete + reinsert(modified): the key tuple re-establishes with
        // the new fact.
        assert_eq!(tx.insert_dyn(ENTRY, [&entry("a", 2)])?.changed(), 1);
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".into())])?,
            Some(entry("a", 2))
        );
        Ok(())
    })
    .expect("write")
    .unwrap();

    // The post-commit answer is byte-identical.
    db.write(|tx| {
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".into())])?,
            Some(entry("a", 2))
        );
        Ok(())
    })
    .expect("read back")
    .unwrap();
}

/// Committed-state fallthrough: a fact committed in a prior transaction
/// and untouched in this delta is found through the `U` → `F` path.
#[test]
fn get_dyn_falls_through_to_committed_state() {
    let dir = TempDir::new("db-get-dyn-committed");
    let db = Db::create(dir.path(), entry_schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| tx.insert_dyn(ENTRY, [&entry("seed", 42)]).map(|_| ()))
        .expect("seed")
        .unwrap();

    db.write(|tx| {
        // Touch a *different* tuple so the delta is nonempty but the
        // probed key has no overlay.
        tx.insert_dyn(ENTRY, [&entry("other", 1)])?;
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("seed".into())])?,
            Some(entry("seed", 42))
        );
        Ok(())
    })
    .expect("read")
    .unwrap();
}

/// A never-interned string key value proves no fact carries it: `Ok(None)`
/// and the dictionary next-id is untouched (the delete-path mint-free
/// contract, extended to point reads).
#[test]
fn get_dyn_with_a_never_interned_key_answers_none_without_minting() {
    let dir = TempDir::new("db-get-dyn-mint-free");
    let db = Db::create(dir.path(), entry_schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| tx.insert_dyn(ENTRY, [&entry("real", 1)]).map(|_| ()))
        .expect("seed")
        .unwrap();
    let entries = dict_entries(&db);

    db.write(|tx| {
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("ghost".into())])?,
            None
        );
        assert_eq!(
            tx.delta.dict_next(),
            None,
            "the point read minted a provisional id"
        );
        Ok(())
    })
    .expect("probe")
    .unwrap();
    assert_eq!(dict_entries(&db), entries, "the dictionary grew on a miss");
}

/// The dynamic surface is data: a wrong statement id, arity, or value
/// type is a typed `FactShape` error, never a panic.
#[test]
fn get_dyn_rejects_mis_shaped_requests_with_typed_errors() {
    let dir = TempDir::new("db-get-dyn-shape");
    let db = Db::create(dir.path(), entry_schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        // Out-of-range statement id.
        let err = tx
            .get_dyn(ENTRY, StatementId(7), &[Value::String("x".into())])
            .unwrap_err();
        assert!(
            matches!(
                err,
                Error::FactShape(FactShapeError::Id(DynIdError::NotAKeyStatement {
                    relation: ENTRY,
                    statement: StatementId(7),
                }))
            ),
            "{err:?}"
        );
        // Key arity mismatch.
        let err = tx.get_dyn(ENTRY, ENTRY_KEY, &entry("x", 1)).unwrap_err();
        assert!(
            matches!(
                err,
                Error::FactShape(FactShapeError::ArityMismatch {
                    relation: ENTRY,
                    mismatch: Mismatch {
                        witnessed: 2,
                        required: 1,
                    },
                })
            ),
            "{err:?}"
        );
        // Key value type mismatch.
        let err = tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::U64(3)]).unwrap_err();
        assert!(
            matches!(
                err,
                Error::FactShape(FactShapeError::TypeMismatch {
                    relation: ENTRY,
                    field: FieldId(0),
                })
            ),
            "{err:?}"
        );
        Ok(())
    })
    .expect("probe")
    .unwrap();
}

/// S(id fresh, v) — the fresh-minting relation for witness tests.
fn fresh_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "S".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Fresh,
                },
                FieldDescriptor {
                    name: "v".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
}

/// What the `schema!` macro would generate for `id: u64 as SId, fresh` —
/// the typed mint path's proof-carrying newtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SId(u64);

impl Fresh for SId {
    type Schema = SchemaDescriptor;
    const RELATION: RelationId = RelationId(0);
    const FIELD: FieldId = FieldId(0);
    fn from_fresh(raw: u64) -> Self {
        Self(raw)
    }
    fn fresh(self) -> u64 {
        self.0
    }
}

/// The resolver is the checking boundary of the untyped mint path: ids
/// and generation are data, so every mis-aimed resolution is a typed
/// `FactShape` error, never a panic.
#[test]
fn fresh_field_rejects_non_witnesses_with_typed_errors() {
    let dir = TempDir::new("db-fresh-field-resolver");
    let db = Db::create(dir.path(), fresh_schema())
        .expect("create")
        .expect("accepted");
    assert_eq!(
        db.fresh_field(RelationId(0), FieldId(1)).unwrap_err(),
        FactShapeError::Id(DynIdError::NotAFreshField {
            relation: RelationId(0),
            field: FieldId(1),
        })
    );
    assert_eq!(
        db.fresh_field(RelationId(9), FieldId(0)).unwrap_err(),
        FactShapeError::Id(DynIdError::UnknownRelation {
            relation: RelationId(9),
        })
    );
    assert_eq!(
        db.fresh_field(RelationId(0), FieldId(9)).unwrap_err(),
        FactShapeError::Id(DynIdError::UnknownField {
            relation: RelationId(0),
            field: FieldId(9),
        })
    );
}

/// Resolve once, mint per row: one witness mints across many `reserve_at`
/// calls, interleaves with the typed path in one sequence, and the
/// sequence persists across transactions.
#[test]
fn a_witness_mints_the_same_sequence_as_the_typed_path() {
    let dir = TempDir::new("db-alloc-witness");
    let db = Db::create(dir.path(), fresh_schema())
        .expect("create")
        .expect("accepted");
    let id_field = db
        .fresh_field(RelationId(0), FieldId(0))
        .expect("fresh field");
    db.write(|tx| {
        assert_eq!(tx.reserve_at(id_field, 1)?.start().expect("nonempty"), 0);
        assert_eq!(
            tx.reserve::<SId>(1)?.start().expect("nonempty"),
            SId(1),
            "one sequence, two surfaces"
        );
        assert_eq!(tx.reserve_at(id_field, 1)?.start().expect("nonempty"), 2);
        assert_eq!(tx.reserve_at(id_field, 1)?.start().expect("nonempty"), 3);
        assert_eq!(tx.reserve::<SId>(1)?.start().expect("nonempty"), SId(4));
        Ok(())
    })
    .expect("mint")
    .unwrap();
    // A committed sequence never re-issues: the witness continues where
    // the first transaction stopped.
    db.write(|tx| {
        assert_eq!(tx.reserve_at(id_field, 1)?.start().expect("nonempty"), 5);
        Ok(())
    })
    .expect("mint again")
    .unwrap();
}

/// The panic gap, closed (`EscapedIdBurn`, the drop guard in
/// `db/write.rs`): the never-reissue law binds EVERY termination of a
/// write — a PANICKING closure included. `reserve` hands the id to the
/// host before the commit's fate is known
/// (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`), so the
/// unwound transaction persists no data but burns its escaped mint —
/// the api-level mirror of the storage-level
/// `fresh_ids_reserved_in_a_rejected_txn_are_burned`.
#[test]
fn a_panicking_write_burns_its_escaped_fresh_ids() {
    let dir = TempDir::new("db-panic-fresh-burn");
    let db = Db::create(dir.path(), fresh_schema())
        .expect("create")
        .expect("accepted");
    let id_field = db
        .fresh_field(RelationId(0), FieldId(0))
        .expect("fresh field");
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = db.write(|tx| -> Result<()> {
            assert_eq!(
                tx.reserve_at(id_field, 1)?.start().expect("nonempty"),
                0,
                "the id escapes to the host"
            );
            panic!("the host's own bug, mid-closure");
        });
    }));
    assert!(unwound.is_err(), "the closure's panic propagates");
    // The unwound transaction persisted no data — the clock never moved...
    assert_eq!(db.generation().expect("generation").value(), 0);
    // ...but the id it handed out is gone forever: the next write mints
    // PAST the burned id.
    db.write(|tx| {
        assert_eq!(
            tx.reserve_at(id_field, 1)?.start().expect("nonempty"),
            1,
            "0 was issued and never re-issues, the panic notwithstanding"
        );
        Ok(())
    })
    .expect("the writer surface survives the panic")
    .unwrap();
}

#[test]
fn a_failed_escaped_flush_on_closure_err_is_visible_and_retried() {
    let dir = TempDir::new("db-flush-fail-closure");
    let db = Db::create(dir.path(), fresh_schema())
        .expect("create")
        .expect("accepted");
    let id_field = db
        .fresh_field(RelationId(0), FieldId(0))
        .expect("fresh field");
    db.env().fail_next_fresh_flushes(1);
    let err = db
        .write(|tx| -> Result<()> {
            assert_eq!(tx.reserve_at(id_field, 1)?.start().expect("nonempty"), 0);
            Err(Error::ForeignWitness)
        })
        .unwrap_err();
    assert!(
        matches!(err, Error::Lmdb(LmdbFailure::Mdb(heed::MdbError::MapFull))),
        "flush failure is the visible error, got {err:?}"
    );
    db.write(|tx| {
        assert_eq!(
            tx.reserve_at(id_field, 1)?.start().expect("nonempty"),
            1,
            "the parked burn retried; 0 was never reissued"
        );
        Ok(())
    })
    .expect("retry at write begin succeeds")
    .unwrap();
}

#[test]
fn a_still_failing_q_burn_poisons_the_next_write_begin() {
    let dir = TempDir::new("db-flush-poison");
    let db = Db::create(dir.path(), fresh_schema())
        .expect("create")
        .expect("accepted");
    let id_field = db
        .fresh_field(RelationId(0), FieldId(0))
        .expect("fresh field");
    db.env().fail_next_fresh_flushes(2);
    let _ = db
        .write(|tx| -> Result<()> {
            assert_eq!(tx.reserve_at(id_field, 1)?.start().expect("nonempty"), 0);
            Err(Error::ForeignWitness)
        })
        .unwrap_err();
    let ran = std::sync::atomic::AtomicBool::new(false);
    let err = db
        .write(|tx| {
            ran.store(true, std::sync::atomic::Ordering::SeqCst);
            let _ = tx.reserve_at(id_field, 1)?.start().expect("nonempty");
            Ok(())
        })
        .unwrap_err();
    assert!(
        matches!(err, Error::Lmdb(LmdbFailure::Mdb(heed::MdbError::MapFull))),
        "{err:?}"
    );
    assert!(
        !ran.load(std::sync::atomic::Ordering::SeqCst),
        "the write closure must not run while Q is not durable"
    );
}

#[test]
fn a_panicking_write_with_a_failed_flush_still_never_reissues() {
    let dir = TempDir::new("db-panic-flush-fail");
    let db = Db::create(dir.path(), fresh_schema())
        .expect("create")
        .expect("accepted");
    let id_field = db
        .fresh_field(RelationId(0), FieldId(0))
        .expect("fresh field");
    db.env().fail_next_fresh_flushes(1);
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = db.write(|tx| -> Result<()> {
            assert_eq!(tx.reserve_at(id_field, 1)?.start().expect("nonempty"), 0);
            panic!("host bug after reserve");
        });
    }));
    assert!(unwound.is_err());
    db.write(|tx| {
        assert_eq!(
            tx.reserve_at(id_field, 1)?.start().expect("nonempty"),
            1,
            "Drop discarded the flush error but kept the in-process floor"
        );
        Ok(())
    })
    .expect("write begin retried the parked burn")
    .unwrap();
}

/// The drop-order lock window: `Db`'s fields drop in declaration
/// order, and a parked reader's transaction owns its own env clone —
/// if the `Environment` (and with it the advisory lock) dropped before
/// `read_cache`, another handle could acquire the lock while heed
/// still holds the path open, and its `Db::open` would surface heed's
/// `EnvAlreadyOpened` as an untyped `Lmdb` error — breaking a retry
/// loop keyed on the typed `EnvironmentLocked`. The opener thread
/// hammers the window while the owner drops; every non-lock error is
/// the regression.
#[test]
fn dropping_the_handle_never_leaks_an_env_already_opened_window() {
    let dir = TempDir::new("db-drop-order");
    drop(
        Db::create(dir.path(), named_schema())
            .expect("create")
            .expect("accepted"),
    );
    // 1,000 rounds reproduced the pre-fix window well within the first
    // hundred on the M2 Max; the budget keeps the race real without
    // dominating the suite.
    for _ in 0..1000 {
        let db = Db::open(dir.path(), named_schema()).expect("open owner");
        db.read(|_| Ok(())).expect("park a reader");
        let path = dir.path().to_path_buf();
        let hot = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let hot_flag = std::sync::Arc::clone(&hot);
        let opener = std::thread::spawn(move || -> Result<()> {
            loop {
                match Db::open(&path, named_schema()) {
                    Ok(reopened) => {
                        drop(reopened);
                        return Ok(());
                    }
                    Err(Error::EnvironmentLocked) => {
                        hot_flag.store(true, std::sync::atomic::Ordering::Release);
                    }
                    Err(other) => return Err(other),
                }
            }
        });
        // The opener is provably in its retry loop before the drop
        // opens the window.
        while !hot.load(std::sync::atomic::Ordering::Acquire) {
            std::hint::spin_loop();
        }
        drop(db);
        opener
            .join()
            .expect("opener thread")
            .expect("the retry loop must see EnvironmentLocked or success, never a raw Lmdb error");
    }
}

/// The cross-schema witness law (RULED 2026-07-15, reversing "the
/// witness carries the proof"): [`crate::schema::FreshField`] carries a
/// BINDING — its resolving handle's schema typestate — so a witness of
/// one `schema!` schema on another's transaction is a compile error
/// (`tests/schema-compile-fail/foreign_fresh_witness.rs`, the other half
/// of this lock). Here at the dyn boundary the binding proves nothing:
/// every `Db<SchemaDescriptor>` shares one typestate, so a witness
/// resolved by database A reaches database B well-typed — and the
/// mint's per-transaction sequence init re-checks the generation and
/// refuses typed, never a panic, never a silent mint (the pre-ruling
/// hole: debug asserted, release minted 0,1,2… from a Q key of a field
/// NOT fresh in the store's schema, breaking `Generation::Fresh`'s
/// never-reissue guarantee). The refusal aborts the transaction whole:
/// no Q entry, no state change.
#[test]
fn a_foreign_witness_is_refused_typed_not_minted() {
    let foreign_dir = TempDir::new("db-foreign-witness-resolver");
    let foreign = Db::create(foreign_dir.path(), fresh_schema())
        .expect("create foreign")
        .expect("accepted");
    let witness = foreign
        .fresh_field(RelationId(0), FieldId(0))
        .expect("fresh in ITS OWN schema");
    let dir = TempDir::new("db-foreign-witness");
    // A different schema at this store: field 0 of relation 0 is a
    // plain String column, not fresh.
    let db = Db::create(dir.path(), named_schema())
        .expect("create")
        .expect("accepted");
    let outcome = db.write(|tx| tx.reserve_at(witness, 1).map(|_| ()));
    match outcome.unwrap_err() {
        Error::FactShape(FactShapeError::Id(DynIdError::NotAFreshField { relation, field })) => {
            assert_eq!(relation, RelationId(0));
            assert_eq!(field, FieldId(0));
        }
        other => panic!("a foreign witness must refuse typed, not mint: {other:?}"),
    }
    // The aborted transaction persisted nothing — the store's clock
    // never moved.
    assert_eq!(db.generation().expect("generation").value(), 0);
}

/// Shape failure of a collection is parse-then-apply: no row enters the
/// delta, so the transaction is not poisoned and a later insert in the
/// same write still commits.
#[test]
fn a_shape_failure_does_not_poison_the_write() {
    let dir = TempDir::new("db-shape-no-poison");
    let db = Db::create(dir.path(), named_schema())
        .expect("create")
        .expect("accepted");
    let named = RelationId(0);

    db.write(|tx| {
        let err = tx
            .insert_dyn(named, [Vec::<Value>::new()])
            .expect_err("empty row is the wrong arity");
        assert!(matches!(err, Error::FactShape(_)), "{err:?}");
        assert_eq!(
            tx.insert_dyn(named, [&[Value::String("ok".into())]])?
                .changed(),
            1
        );
        Ok(())
    })
    .expect("the shape miss did not poison")
    .unwrap();
    let n = db.read(|snap| Ok(snap.scan(named)?.count())).expect("scan");
    assert_eq!(n, 1);
}

#[test]
fn empty_fresh_range_has_no_minted_id() {
    let dir = TempDir::new("db-empty-fresh");
    let db = Db::create(dir.path(), fresh_schema())
        .expect("create")
        .expect("accepted");
    let field = db
        .fresh_field(RelationId(0), FieldId(0))
        .expect("fresh field");
    db.write(|tx| {
        let range = tx.reserve::<SId>(0)?;
        assert!(range.is_empty());
        assert!(range.start().is_none());
        assert!(range.get(0).is_none());
        assert!(range.iter().next().is_none());
        let raw = tx.reserve_at(field, 0)?;
        assert!(raw.start().is_none());
        Ok(())
    })
    .expect("empty reserve")
    .unwrap();
}

#[test]
fn a_noop_insert_does_not_mark_applied_so_shape_fail_stays_clean() {
    let dir = TempDir::new("db-noop-not-applied");
    let db = Db::create(dir.path(), named_schema())
        .expect("create")
        .expect("accepted");
    let named = RelationId(0);
    let row = [Value::String("keep".into())];
    db.write(|tx| tx.insert_dyn(named, [&row]).map(|_| ()))
        .expect("seed")
        .unwrap();
    db.write(|tx| {
        assert_eq!(tx.insert_dyn(named, [&row])?.changed(), 0, "redundant");
        let err = tx
            .insert_dyn(named, [Vec::<Value>::new()])
            .expect_err("shape");
        assert!(matches!(err, Error::FactShape(_)), "{err:?}");
        assert_eq!(
            tx.insert_dyn(named, [&[Value::String("next".into())]])?
                .changed(),
            1
        );
        Ok(())
    })
    .expect("shape fail after no-op did not poison")
    .unwrap();
}

#[test]
fn poison_preserves_the_original_error_and_empty_insert_is_no_engine_request() {
    let dir = TempDir::new("db-poison-kind");
    let db = Db::create(dir.path(), fresh_schema())
        .expect("create")
        .expect("accepted");
    let rel = RelationId(0);
    let outcome = db.write(|tx| {
        tx.insert_dyn(rel, [&[Value::U64(u64::MAX), Value::U64(0)]])?;
        let first = tx.reserve::<SId>(1).expect_err("exhausted");
        assert!(
            matches!(first, Error::FreshExhausted { .. }),
            "first apply failure is the original: {first:?}"
        );
        assert_eq!(
            tx.insert_dyn(rel, Vec::<Vec<Value>>::new())
                .expect("empty is no engine request")
                .submitted(),
            0
        );
        Ok(())
    });
    match outcome {
        Err(Error::TransactionPoisoned { source }) => {
            assert!(matches!(source.as_ref(), Error::FreshExhausted { .. }));
        }
        other => panic!("Db::write aborts on Ok after poison: {other:?}"),
    }
}

/// Currency { `minor_units`: u64 } = { Usd(2), Eur(2) }: the closed fixture
/// for the write-refusal tests (hand-built — the macro grammar for closed
/// relations is the emission PRD's).
fn closed_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: Some(Box::new([
                bumbledb_theory::schema::Row {
                    handle: "Usd".into(),
                    values: Box::new([Value::U64(2)]),
                },
                bumbledb_theory::schema::Row {
                    handle: "Eur".into(),
                    values: Box::new([Value::U64(2)]),
                },
            ])),
            name: "Currency".into(),
            fields: vec![FieldDescriptor {
                name: "minor_units".into(),
                value_type: ValueType::U64,
                generation: Generation::None,
            }],
        }],
        statements: vec![],
    }
}

/// Any delta operation naming a closed relation is `ClosedRelationWrite`,
/// typed away before any encoding runs — the mis-shaped value below (one
/// value where the sealed arity is two) never even reaches the shape
/// check — and nothing reaches the delta: a closure that swallows the
/// refusal commits empty, so the state-changing generation never moves
/// and the store stays rowless.
#[test]
fn writes_to_a_closed_relation_are_refused_before_the_delta() {
    let dir = TempDir::new("db-closed-write");
    let db = Db::create(dir.path(), closed_schema())
        .expect("create")
        .expect("accepted");
    let currency = RelationId(0);

    let insert = db.write(|tx| tx.insert_dyn(currency, [&[Value::U64(9)]]).map(|_| ()));
    assert!(matches!(
        insert,
        Err(Error::ClosedRelationWrite { relation }) if relation == currency
    ));
    let delete = db.write(|tx| tx.delete_dyn(currency, [&[Value::U64(2)]]).map(|_| ()));
    assert!(matches!(
        delete,
        Err(Error::ClosedRelationWrite { relation }) if relation == currency
    ));

    // A collection naming a closed relation is refused the same way —
    // nothing enters the delta.
    let collection = db
        .write(|tx| {
            tx.insert_dyn(currency, vec![vec![Value::U64(9)]])
                .map(crate::MutationReport::changed)
        })
        .expect_err("closed relations refuse collection inserts");
    assert!(matches!(
        collection,
        Error::ClosedRelationWrite { relation } if relation == currency
    ));

    // The delta stayed empty: swallowing the refusal commits nothing —
    // no generation movement, no stored rows.
    let before = db.generation().expect("generation");
    db.write(|tx| {
        assert!(matches!(
            tx.insert_dyn(currency, [&[Value::U64(9)]]),
            Err(Error::ClosedRelationWrite { .. })
        ));
        Ok(())
    })
    .expect("the refusal is the operation's, not the transaction's")
    .unwrap();
    assert_eq!(db.generation().expect("generation"), before);
    // The read surface still answers — the extension, virtually: exactly
    // the two ground axioms, never a stored row (the store contains zero
    // vocabulary bytes; `verify_store` convicts any that appear).
    db.read(|snap| {
        let rows: Vec<Vec<Value>> = snap.scan(currency)?.collect::<crate::error::Result<_>>()?;
        assert_eq!(
            rows,
            vec![
                vec![Value::U64(0), Value::U64(2)],
                vec![Value::U64(1), Value::U64(2)],
            ]
        );
        Ok(())
    })
    .expect("read");
}

/// Point reads on a closed relation resolve against the sealed extension
/// — the closed auto-key (`Currency(id) -> Currency`, statement 0: no
/// fresh fields exist) probes no `U` namespace, and the error surface for
/// unknown ids is exactly the ordinary one.
#[test]
fn closed_point_reads_resolve_against_the_extension() {
    let dir = TempDir::new("db-closed-get");
    let db = Db::create(dir.path(), closed_schema())
        .expect("create")
        .expect("accepted");
    let currency = RelationId(0);
    let auto_key = StatementId(0);

    db.write(|tx| {
        // A known handle id: the full row (synthetic id ‖ intrinsics).
        let row = tx
            .get_dyn(currency, auto_key, &[Value::U64(1)])?
            .expect("Eur is row 1");
        assert_eq!(row, vec![Value::U64(1), Value::U64(2)]);
        // An id beyond the extension: absent, exactly like an ordinary
        // relation's missing key.
        assert_eq!(tx.get_dyn(currency, auto_key, &[Value::U64(9)])?, None);
        // The existing typed error surface, unchanged: unknown relation,
        // non-key statement, arity mismatch.
        assert!(matches!(
            tx.get_dyn(RelationId(7), auto_key, &[Value::U64(0)]),
            Err(Error::FactShape(FactShapeError::Id(
                DynIdError::UnknownRelation { .. }
            )))
        ));
        assert!(matches!(
            tx.get_dyn(currency, StatementId(9), &[Value::U64(0)]),
            Err(Error::FactShape(FactShapeError::Id(
                DynIdError::NotAKeyStatement { .. }
            )))
        ));
        assert!(matches!(
            tx.get_dyn(currency, auto_key, &[]),
            Err(Error::FactShape(FactShapeError::ArityMismatch { .. }))
        ));
        Ok(())
    })
    .expect("write")
    .unwrap();
}

/// The `Q` ratchet law at the public surface, across commits
/// (`docs/architecture/50-storage.md` § Key layout, the `Q` row): an
/// explicit-id insert advances the fresh high-water past the supplied
/// value AND the advance persists, so a later transaction's mint is
/// strictly greater — a copied id (the rebirth pattern: explicit-id
/// inserts of another store's rows) can never collide with a
/// subsequent fresh mint.
#[test]
fn an_explicit_id_insert_ratchets_the_persisted_fresh_high_water() {
    let dir = TempDir::new("db-fresh-ratchet");
    let db = Db::create(dir.path(), fresh_schema())
        .expect("create")
        .expect("accepted");
    let rel = RelationId(0);
    let field = bumbledb_theory::schema::FieldId(0);

    // Commit 1: an explicit id well past anything ever minted.
    db.write(|tx| {
        tx.insert_dyn(rel, [&[Value::U64(41), Value::U64(1)]])
            .map(|_| ())
    })
    .expect("explicit-id write")
    .unwrap();

    // Commit 2: the next mint is strictly greater — never a collision.
    let witness = db.fresh_field(rel, field).expect("fresh field");
    let minted = db
        .write(|tx| {
            let id = tx.reserve_at(witness, 1)?.start().expect("nonempty");
            tx.insert_dyn(rel, [&[Value::U64(id), Value::U64(2)]])?;
            Ok(id)
        })
        .expect("minting write")
        .unwrap()
        .value;
    assert!(minted > 41, "minted {minted} must exceed the copied id 41");

    // And the ratchet survives a reopen: the mark was flushed to `Q`,
    // not just held in memory.
    drop(db);
    let db = Db::open(dir.path(), fresh_schema()).expect("reopen");
    let witness = db.fresh_field(rel, field).expect("fresh field");
    let after_reopen = db
        .write(|tx| Ok(tx.reserve_at(witness, 1)?.start().expect("nonempty")))
        .expect("mint after reopen")
        .unwrap()
        .value;
    assert!(
        after_reopen > minted,
        "post-reopen mint {after_reopen} must exceed {minted}"
    );
}

/// The pooled point-read lane (docs/architecture/70-api.md § point
/// reads): `get_dyn_into` fills the caller's buffer on a hit — capacity
/// AND allocation retained across warm gets — clears it on a miss, and
/// answers byte-identically to the owned `get_dyn` on both the snapshot
/// and write-transaction surfaces.
#[test]
fn get_dyn_into_reuses_the_callers_buffer_on_both_surfaces() {
    let dir = TempDir::new("db-get-dyn-into");
    let db = Db::create(dir.path(), entry_schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(ENTRY, [&entry("a", 1)])?;
        tx.insert_dyn(ENTRY, [&entry("b", 2)]).map(|_| ())
    })
    .expect("seed")
    .unwrap();

    let mut out = Vec::new();
    db.read(|snap| {
        assert!(snap.get_dyn_into(ENTRY, ENTRY_KEY, &[Value::String("a".into())], &mut out)?);
        assert_eq!(out, entry("a", 1));
        let (capacity, ptr) = (out.capacity(), out.as_ptr());
        assert!(snap.get_dyn_into(ENTRY, ENTRY_KEY, &[Value::String("b".into())], &mut out)?);
        assert_eq!(out, entry("b", 2));
        assert_eq!(
            (out.capacity(), out.as_ptr()),
            (capacity, ptr),
            "the warm get reuses the caller's allocation"
        );
        assert!(!snap.get_dyn_into(
            ENTRY,
            ENTRY_KEY,
            &[Value::String("missing".into())],
            &mut out
        )?);
        assert!(out.is_empty(), "a miss leaves the buffer empty");
        assert_eq!(
            snap.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".into())])?,
            Some(entry("a", 1)),
            "the owned convenience is the same read"
        );
        Ok(())
    })
    .expect("snapshot reads");

    db.write(|tx| {
        assert!(tx.get_dyn_into(ENTRY, ENTRY_KEY, &[Value::String("a".into())], &mut out)?);
        assert_eq!(out, entry("a", 1));
        assert!(!tx.get_dyn_into(ENTRY, ENTRY_KEY, &[Value::String("zzz".into())], &mut out)?);
        assert!(out.is_empty());
        Ok(())
    })
    .expect("write-surface reads")
    .unwrap();
}

fn all_entries() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: AtomSource::Edb(ENTRY),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn read_instance_prepares_executes_and_gets() {
    let dir = TempDir::new("db-read-instance-query");
    let db = Db::create(dir.path(), entry_schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| tx.insert_dyn(ENTRY, [&entry("ada", 10)]).map(|_| ()))
        .expect("seed")
        .expect("accepted");

    db.read(|instance| {
        assert_eq!(instance.row_count(ENTRY)?, 1);
        assert_eq!(
            instance.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("ada".into())])?,
            Some(entry("ada", 10))
        );
        let mut prepared = instance.prepare(&all_entries())?;
        let mut out = crate::Answers::new();
        instance.execute(&mut prepared, &[] as &[crate::ParamArg], &mut out)?;
        assert_eq!(out.len(), 1);
        Ok(())
    })
    .expect("read");
}

fn mint_witness<S>(instance: &ReadInstance<'_, S>) -> Result<Witness<S>> {
    instance.witness()
}

#[test]
fn write_from_borrows_a_cloneable_witness() {
    let dir = TempDir::new("db-write-from-witness");
    let db = Db::create(dir.path(), named_schema())
        .expect("create")
        .expect("accepted");
    let named = RelationId(0);
    let witness = db.read(mint_witness).expect("mint witness");
    let retained = witness.clone();

    match db
        .write_from(&witness, |tx| {
            tx.insert_dyn(named, [&[Value::String("one".into())]])
                .map(|_| ())
        })
        .expect("first write_from")
    {
        ConditionalWrite::Accepted(_) => {}
        other => panic!("first write_from must accept: {other:?}"),
    }
    match db
        .write_from(&retained, |tx| {
            tx.insert_dyn(named, [&[Value::String("two".into())]])
                .map(|_| ())
        })
        .expect("clone after move")
    {
        ConditionalWrite::Moved { witnessed, current } => {
            assert_eq!(witnessed.value(), 0);
            assert_eq!(current.value(), 1);
        }
        other => panic!("stale clone must report Moved: {other:?}"),
    }

    let fresh = db.read(mint_witness).expect("fresh witness");
    match db.write_from(&fresh, |_| Ok(())).expect("no-op") {
        ConditionalWrite::Accepted(_) => {}
        other => panic!("same-generation reuse must accept: {other:?}"),
    }
    match db.write_from(&fresh, |_| Ok(())).expect("reuse") {
        ConditionalWrite::Accepted(_) => {}
        other => panic!("one witness justifies two writes: {other:?}"),
    }
}

#[test]
fn write_from_rejects_a_foreign_witness() {
    let dir = TempDir::new("db-write-from-foreign-a");
    let foreign_dir = TempDir::new("db-write-from-foreign-b");
    let db = Db::create(dir.path(), named_schema())
        .expect("create")
        .expect("accepted");
    let foreign = Db::create(foreign_dir.path(), named_schema())
        .expect("create foreign")
        .expect("accepted");
    let witness = foreign.read(mint_witness).expect("foreign witness");
    let err = db.write_from(&witness, |_| Ok(())).unwrap_err();
    assert!(matches!(err, Error::ForeignWitness), "{err:?}");
    assert_eq!(db.generation().expect("generation").value(), 0);
}
