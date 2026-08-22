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

    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "ReadInstance::generation is not HRTB enough for Db::read"
    )]
    let g1 = db.read(|snap| snap.generation()).expect("read");
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "ReadInstance::generation is not HRTB enough for Db::read"
    )]
    let g2 = db.read(|snap| snap.generation()).expect("read");
    assert_eq!(g1, g2, "parked reuse serves the same snapshot");

    let err: Result<()> = db.read(|_| {
        Err(crate::error::Error::Overflow(
            crate::error::OverflowKind::Aggregate { find: FindIndex(7) },
        ))
    });
    assert!(err.is_err());
    let again = db.read(|snap| count_named(snap)).expect("read after error");
    assert_eq!(again, 1);

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

    db.write(|tx| {
        let changed = tx.delete_dyn(named, [&[Value::String("real".into())]])?;
        assert_eq!(changed.changed(), 1);
        Ok(())
    })
    .expect("real delete")
    .unwrap();
}

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

#[test]
fn get_dyn_reads_its_own_writes_exactly_as_a_later_transaction_does() {
    let dir = TempDir::new("db-get-dyn-ryw");
    let db = Db::create(dir.path(), entry_schema())
        .expect("create")
        .expect("accepted");

    db.write(|tx| {
        assert_eq!(tx.insert_dyn(ENTRY, [&entry("a", 1)])?.changed(), 1);
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".into())])?,
            Some(entry("a", 1))
        );

        assert_eq!(tx.delete_dyn(ENTRY, [&entry("a", 1)])?.changed(), 1);
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".into())])?,
            None
        );

        assert_eq!(tx.insert_dyn(ENTRY, [&entry("a", 2)])?.changed(), 1);
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".into())])?,
            Some(entry("a", 2))
        );
        Ok(())
    })
    .expect("write")
    .unwrap();

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
            tx.delta().dict_next(),
            None,
            "the point read minted a provisional id"
        );
        Ok(())
    })
    .expect("probe")
    .unwrap();
    assert_eq!(dict_entries(&db), entries, "the dictionary grew on a miss");
}

#[test]
fn get_dyn_rejects_mis_shaped_requests_with_typed_errors() {
    let dir = TempDir::new("db-get-dyn-shape");
    let db = Db::create(dir.path(), entry_schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
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

    db.write(|tx| {
        assert_eq!(tx.reserve_at(id_field, 1)?.start().expect("nonempty"), 5);
        Ok(())
    })
    .expect("mint again")
    .unwrap();
}

/// The panic gap, closed (`EscapedIdBurn`, the drop guard in `db/write.rs`):
/// the never-reissue law binds EVERY termination of a write — a PANICKING
/// closure included. `reserve` hands the id to the host before the commit's
/// fate is known (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`), so
/// the unwound transaction persists no data but burns its escaped mint — the
/// api-level mirror of the storage-level
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

    assert_eq!(db.generation().expect("generation").value(), 0);

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

/// The drop-order lock window: `Db`'s fields drop in declaration order, and a
/// parked reader's transaction owns its own env clone — if the `Environment`
/// (and with it the advisory lock) dropped before `read_cache`, another handle
/// could acquire the lock while heed still holds the path open, and its
/// `Db::open` would surface heed's `EnvAlreadyOpened` as an untyped `Lmdb`
/// error — breaking a retry loop keyed on the typed `EnvironmentLocked`.
#[test]
fn dropping_the_handle_never_leaks_an_env_already_opened_window() {
    let dir = TempDir::new("db-drop-order");
    drop(
        Db::create(dir.path(), named_schema())
            .expect("create")
            .expect("accepted"),
    );

    // hundred on the M2 Max; the budget keeps the race real without

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

/// The refusal aborts the transaction whole: no Q entry, no state change.
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

    assert_eq!(db.generation().expect("generation").value(), 0);
}

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

/// Currency { `minor_units`: u64 } = { Usd(2), Eur(2) }: the closed fixture for
/// the write-refusal tests (hand-built — the macro grammar for closed relations
/// is the emission PRD's).
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

/// Any delta operation naming a closed relation is `ClosedRelationWrite`, typed
/// away before any encoding runs — the mis-shaped value below (one value where
/// the sealed arity is two) never even reaches the shape check — and nothing
/// reaches the delta: a closure that swallows the refusal commits empty, so the
/// state-changing generation never moves and the store stays rowless.
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

#[test]
fn closed_point_reads_resolve_against_the_extension() {
    let dir = TempDir::new("db-closed-get");
    let db = Db::create(dir.path(), closed_schema())
        .expect("create")
        .expect("accepted");
    let currency = RelationId(0);
    let auto_key = StatementId(0);

    db.write(|tx| {
        let row = tx
            .get_dyn(currency, auto_key, &[Value::U64(1)])?
            .expect("Eur is row 1");
        assert_eq!(row, vec![Value::U64(1), Value::U64(2)]);

        assert_eq!(tx.get_dyn(currency, auto_key, &[Value::U64(9)])?, None);

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

#[test]
fn an_explicit_id_insert_ratchets_the_persisted_fresh_high_water() {
    let dir = TempDir::new("db-fresh-ratchet");
    let db = Db::create(dir.path(), fresh_schema())
        .expect("create")
        .expect("accepted");
    let rel = RelationId(0);
    let field = bumbledb_theory::schema::FieldId(0);

    db.write(|tx| {
        tx.insert_dyn(rel, [&[Value::U64(41), Value::U64(1)]])
            .map(|_| ())
    })
    .expect("explicit-id write")
    .unwrap();

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
        assert_eq!(instance.count(ENTRY)?, 1);
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

/// The exact-count law on the store surface: after mixed inserts and deletes
/// across commits, `count` equals the scan length in the SAME lease — the API
/// twin of the storage pin
/// (`storage/read/tests.rs::row_count_equals_scan_count_after_mixed_commits`).
#[test]
fn count_equals_scan_length_after_mixed_commits_and_reads_the_sealed_extension() {
    let dir = TempDir::new("db-count");
    let db = Db::create(dir.path(), entry_schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        tx.insert_dyn(ENTRY, [&entry("a", 1), &entry("b", 2), &entry("c", 3)])
            .map(|_| ())
    })
    .expect("seed")
    .unwrap();
    db.write(|tx| {
        tx.delete_dyn(ENTRY, [&entry("b", 2)])?;
        tx.insert_dyn(ENTRY, [&entry("d", 4)]).map(|_| ())
    })
    .expect("mixed commit")
    .unwrap();
    db.read(|instance| {
        let mut scanned = 0u64;
        for row in instance.scan(ENTRY)? {
            row?;
            scanned += 1;
        }
        assert_eq!(instance.count(ENTRY)?, scanned, "count is the scan length");
        assert_eq!(scanned, 3);
        Ok(())
    })
    .expect("read");

    let closed_dir = TempDir::new("db-count-closed");
    let closed = Db::create(closed_dir.path(), closed_schema())
        .expect("create")
        .expect("accepted");
    closed
        .read(|instance| {
            assert_eq!(
                instance.count(RelationId(0))?,
                2,
                "a closed relation's count IS its sealed extension length"
            );
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

crate::schema! {
    pub OneRep;
    relation Sample {
        flag: bool,
        count: u64,
        delta: i64,
        memo: str,
        tag: bytes<3>,
        window: interval<u64>,
        span: interval<i64>,
    }
}

fn sample_facts() -> Vec<Sample<'static>> {
    let iv_u = |a, b| crate::Interval::<u64>::new(a, b).expect("nonempty");
    let iv_i = |a, b| crate::Interval::<i64>::new(a, b).expect("nonempty");
    vec![
        Sample {
            flag: true,
            count: 7,
            delta: -3,
            memo: "alpha",
            tag: [1, 2, 3],
            window: iv_u(1, 5),
            span: iv_i(-4, 4),
        },
        Sample {
            flag: false,
            count: 8,
            delta: 9,
            memo: "beta",
            tag: [9, 9, 9],
            window: iv_u(0, 2),
            span: iv_i(-1, 0),
        },
        Sample {
            flag: true,
            count: 7,
            delta: -3,
            memo: "alpha",
            tag: [1, 2, 3],
            window: iv_u(2, 6),
            span: iv_i(0, 3),
        },
    ]
}

fn sample_value_rows() -> Vec<Vec<Value>> {
    sample_facts()
        .iter()
        .map(|fact| {
            vec![
                Value::Bool(fact.flag),
                Value::U64(fact.count),
                Value::I64(fact.delta),
                Value::String(fact.memo.into()),
                Value::FixedBytes(Box::from(fact.tag.as_slice())),
                Value::IntervalU64(fact.window),
                Value::IntervalI64(fact.span),
            ]
        })
        .collect()
}

fn sorted_scan(db: &Db<OneRep>) -> Vec<Vec<Value>> {
    let mut rows: Vec<Vec<Value>> = db
        .read(|snap| snap.scan(Sample::RELATION)?.collect::<Result<_>>())
        .expect("scan");
    rows.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    rows
}

fn coherent(db: &Db<OneRep>) {
    let report = db.verify_store().expect("verify");
    assert!(
        matches!(
            report.verdict,
            crate::verify_store::StoreVerdict::Coherent { .. }
        ),
        "{report:?}"
    );
}

#[test]
fn the_three_write_lanes_produce_identical_stores() {
    let typed_dir = TempDir::new("db-lanes-typed");
    let dyn_dir = TempDir::new("db-lanes-dyn");
    let accepted_dir = TempDir::new("db-lanes-accepted");
    let typed_db = Db::create(typed_dir.path(), OneRep)
        .expect("create")
        .expect("accepted");
    let dyn_db = Db::create(dyn_dir.path(), OneRep)
        .expect("create")
        .expect("accepted");
    let accepted_db = Db::create(accepted_dir.path(), OneRep)
        .expect("create")
        .expect("accepted");

    let typed_report = typed_db
        .write(|tx| tx.insert(&sample_facts()))
        .expect("typed insert")
        .unwrap()
        .value;
    let dyn_report = dyn_db
        .write(|tx| tx.insert_dyn(Sample::RELATION, sample_value_rows()))
        .expect("dyn insert")
        .unwrap()
        .value;
    let accepted_report = accepted_db
        .write(|tx| {
            let fields = accepted_db.schema().relation(Sample::RELATION).fields();
            let mut builder = CollectionBuilder::new(Sample::RELATION, fields);
            for fact in sample_facts() {
                builder.push_bool(fact.flag)?;
                builder.push_u64(fact.count)?;
                builder.push_i64(fact.delta)?;
                builder.push_str(fact.memo)?;
                builder.push_bytes(&fact.tag)?;
                builder.push_interval_u64(fact.window)?;
                builder.push_interval_i64(fact.span)?;
            }
            tx.insert_accepted(&builder.seal()?)
        })
        .expect("accepted insert")
        .unwrap()
        .value;

    assert_eq!(typed_report, dyn_report);
    assert_eq!(typed_report, accepted_report);
    assert_eq!(typed_report.submitted(), 3);
    assert_eq!(typed_report.changed(), 3);

    let typed_rows = sorted_scan(&typed_db);
    assert_eq!(typed_rows.len(), 3);
    assert_eq!(typed_rows, sorted_scan(&dyn_db));
    assert_eq!(typed_rows, sorted_scan(&accepted_db));
    assert_eq!(dict_entries(&typed_db), dict_entries(&dyn_db));
    assert_eq!(dict_entries(&typed_db), dict_entries(&accepted_db));
    let generation = typed_db.generation().expect("generation");
    assert_eq!(generation, dyn_db.generation().expect("generation"));
    assert_eq!(generation, accepted_db.generation().expect("generation"));
    coherent(&typed_db);
    coherent(&dyn_db);
    coherent(&accepted_db);
}

/// The collection is `Send` by construction — built on the caller's thread,
/// consumable on the transaction's (invariant 5 of 20).
#[test]
fn accepted_collection_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<AcceptedCollection>();
}

/// Law 2, accepted lane: an empty collection is `MutationReport::EMPTY` before
/// ANY refusal — unknown relation, closed relation, and a poisoned transaction
/// all answer the empty report, exactly as the dyn lane always has.
#[test]
fn an_empty_accepted_collection_is_lawful_before_any_refusal() {
    let dir = TempDir::new("db-accepted-empty");
    let db = Db::create(dir.path(), fresh_schema())
        .expect("create")
        .expect("accepted");
    let fields = db.schema().relation(RelationId(0)).fields();
    // Unknown relation, empty: no engine request, no refusal.
    let unknown = CollectionBuilder::new(RelationId(99), fields)
        .seal()
        .expect("empty seals lawfully");

    let outcome = db.write(|tx| {
        assert_eq!(
            tx.insert_accepted(&unknown)
                .expect("empty is no engine request")
                .submitted(),
            0
        );
        tx.insert_dyn(RelationId(0), [&[Value::U64(u64::MAX), Value::U64(0)]])?;
        let exhausted = tx.reserve::<SId>(1).expect_err("exhausted");
        assert!(matches!(exhausted, Error::FreshExhausted { .. }));
        let empty = CollectionBuilder::new(RelationId(0), fields)
            .seal()
            .expect("empty seals lawfully");
        assert_eq!(
            tx.insert_accepted(&empty)
                .expect("empty precedes the poison refusal")
                .submitted(),
            0
        );
        // Nonempty against the poisoned transaction: typed refusal.
        let row = AcceptedCollection::from_value_rows(
            RelationId(0),
            fields,
            [&[Value::U64(1), Value::U64(2)]],
        )
        .expect("shape-lawful");
        let err = tx.insert_accepted(&row).expect_err("poisoned");
        assert!(matches!(err, Error::TransactionPoisoned { .. }), "{err:?}");
        Ok(())
    });
    assert!(matches!(outcome, Err(Error::TransactionPoisoned { .. })));
}

/// Laws 5 and the roster walls, accepted lane: closed refusal is typed before
/// the delta; an unknown relation is `UnknownRelation`; a collection whose
/// sealed arity disagrees with the target roster is the authoritative second
/// wall's `ArityMismatch`.
#[test]
fn accepted_collections_hit_the_same_walls_as_the_dyn_lane() {
    let dir = TempDir::new("db-accepted-walls");
    let db = Db::create(dir.path(), closed_schema())
        .expect("create")
        .expect("accepted");
    let currency = RelationId(0);
    let sealed_fields = db.schema().relation(currency).fields();

    db.write(|tx| {
        // Closed refusal, both dispositions.
        let row = AcceptedCollection::from_value_rows(
            currency,
            sealed_fields,
            [&[Value::U64(0), Value::U64(2)]],
        )
        .expect("shape-lawful against the sealed roster");
        for err in [
            tx.insert_accepted(&row).expect_err("closed"),
            tx.delete_accepted(&row).expect_err("closed"),
        ] {
            assert!(
                matches!(err, Error::ClosedRelationWrite { relation } if relation == currency),
                "{err:?}"
            );
        }

        let foreign = AcceptedCollection::from_value_rows(
            RelationId(99),
            sealed_fields,
            [&[Value::U64(0), Value::U64(2)]],
        )
        .expect("the constructor judges shape, not the roster");
        let err = tx.insert_accepted(&foreign).expect_err("unknown");
        assert!(
            matches!(
                err,
                Error::FactShape(FactShapeError::Id(DynIdError::UnknownRelation { .. }))
            ),
            "{err:?}"
        );
        Ok(())
    })
    .expect("walls refuse operations, not the transaction")
    .unwrap();
}

/// The arity re-verification (the authoritative second wall): a collection
/// sealed against a narrower roster than the target's is refused with the same
/// typed `ArityMismatch` the dyn parse would raise.
#[test]
fn an_accepted_collection_of_foreign_arity_is_refused_at_apply() {
    let dir = TempDir::new("db-accepted-arity-wall");
    let db = Db::create(dir.path(), entry_schema())
        .expect("create")
        .expect("accepted");
    let fields = db.schema().relation(ENTRY).fields();

    let mut builder = CollectionBuilder::new(ENTRY, &fields[..1]);
    builder.push_str("ada").expect("string column");
    let narrow = builder.seal().expect("complete against its roster");
    db.write(|tx| {
        let err = tx.insert_accepted(&narrow).expect_err("arity wall");
        assert!(
            matches!(
                err,
                Error::FactShape(FactShapeError::ArityMismatch {
                    relation: ENTRY,
                    mismatch: Mismatch {
                        witnessed: 1,
                        required: 2,
                    },
                })
            ),
            "{err:?}"
        );
        Ok(())
    })
    .expect("the wall refuses the operation, not the transaction")
    .unwrap();
}

/// The roster ECHO (the second wall's type half): a collection sealed against a
/// forged roster of the SAME arity but different value types is refused at
/// apply with the typed `TypeMismatch` naming the first differing field — arity
/// re-anchoring alone would have admitted it and the encoder's positional arms
/// would have written wrong-width fact bytes.
#[test]
fn an_accepted_collection_of_foreign_types_is_refused_at_apply() {
    let dir = TempDir::new("db-accepted-type-wall");
    let db = Db::create(dir.path(), entry_schema())
        .expect("create")
        .expect("accepted");

    let forged = [
        FieldDescriptor {
            name: "name".into(),
            value_type: ValueType::U64,
            generation: Generation::None,
        },
        FieldDescriptor {
            name: "amount".into(),
            value_type: ValueType::U64,
            generation: Generation::None,
        },
    ];
    let mut builder = CollectionBuilder::new(ENTRY, &forged);
    builder.push_u64(7).expect("u64 against the forged roster");
    builder.push_u64(9).expect("u64 against the forged roster");
    let foreign = builder.seal().expect("complete against its roster");
    db.write(|tx| {
        let err = tx.insert_accepted(&foreign).expect_err("type wall");
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
    .expect("the wall refuses the operation, not the transaction")
    .unwrap();
}

#[test]
fn accepted_reports_are_exact_and_delete_never_mints() {
    let dir = TempDir::new("db-accepted-exact");
    let db = Db::create(dir.path(), entry_schema())
        .expect("create")
        .expect("accepted");
    let fields = db.schema().relation(ENTRY).fields();
    db.write(|tx| {
        let twice = AcceptedCollection::from_value_rows(
            ENTRY,
            fields,
            [&entry("ada", 1), &entry("ada", 1)],
        )
        .expect("shape-lawful");
        let report = tx.insert_accepted(&twice)?;
        assert_eq!(report.submitted(), 2);
        assert_eq!(report.changed(), 1);
        Ok(())
    })
    .expect("seed")
    .unwrap();

    let before = dict_entries(&db);
    db.write(|tx| {
        let ghost =
            AcceptedCollection::from_value_rows(ENTRY, fields, [&entry("never-interned", 9)])
                .expect("shape-lawful");
        let report = tx.delete_accepted(&ghost)?;
        assert_eq!(report.submitted(), 1);
        assert_eq!(report.changed(), 0, "a dictionary miss proves absence");
        let real = AcceptedCollection::from_value_rows(ENTRY, fields, [&entry("ada", 1)])
            .expect("shape-lawful");
        assert_eq!(tx.delete_accepted(&real)?.changed(), 1);
        Ok(())
    })
    .expect("delete lane")
    .unwrap();
    assert_eq!(
        dict_entries(&db),
        before,
        "the delete disposition interned nothing"
    );
}

fn nullary_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Marker".into(),
            fields: vec![],
        }],
        statements: vec![],
    }
}

#[test]
fn a_nullary_accepted_collection_applies_in_constant_time() {
    let dir = TempDir::new("db-accepted-nullary");
    let db = Db::create(dir.path(), nullary_schema())
        .expect("create")
        .expect("accepted");
    let marker = RelationId(0);
    let fields = db.schema().relation(marker).fields();
    let huge = CollectionBuilder::new(marker, fields)
        .seal_nullary(u64::MAX)
        .expect("a fieldless collection seals from the stated count");
    assert_eq!(huge.rows(), u64::MAX);
    db.write(|tx| {
        let report = tx.insert_accepted(&huge)?;
        assert_eq!(report.submitted(), u64::MAX, "submitted is exact");
        assert_eq!(report.changed(), 1, "the one empty tuple entered once");
        Ok(())
    })
    .expect("insert lane")
    .unwrap();
    db.write(|tx| {
        let report = tx.delete_accepted(&huge)?;
        assert_eq!(report.submitted(), u64::MAX, "submitted is exact");
        assert_eq!(report.changed(), 1, "the delete twin: one op, changed <= 1");
        Ok(())
    })
    .expect("delete lane")
    .unwrap();

    let entry_fields = [
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
    ];
    let err = CollectionBuilder::new(marker, &entry_fields)
        .seal_nullary(1)
        .expect_err("widthful roster");
    assert!(
        matches!(
            err,
            Error::FactShape(FactShapeError::ArityMismatch {
                relation,
                mismatch: Mismatch {
                    witnessed: 0,
                    required: 2,
                },
            }) if relation == marker
        ),
        "{err:?}"
    );
}

#[test]
fn a_fieldless_row_push_is_refused_typed() {
    let dir = TempDir::new("db-nullary-push");
    let db = Db::create(dir.path(), nullary_schema())
        .expect("create")
        .expect("accepted");
    let marker = RelationId(0);
    let fields = db.schema().relation(marker).fields();
    // The mixed sequence is refused AT THE PUSH, with the zero-width

    let mut builder = CollectionBuilder::new(marker, fields);
    let err = builder.push_value_row(&[]).expect_err("one spelling");
    assert!(
        matches!(
            err,
            Error::FactShape(FactShapeError::ArityMismatch {
                relation,
                mismatch: Mismatch {
                    witnessed: 1,
                    required: 0,
                },
            }) if relation == marker
        ),
        "{err:?}"
    );

    let rows: [&[Value]; 3] = [&[], &[], &[]];
    let coll = AcceptedCollection::from_value_rows(marker, fields, rows)
        .expect("empty slices are the fieldless roster's arity-lawful rows");
    assert_eq!(coll.rows(), 3);
    db.write(|tx| {
        let report = tx.insert_dyn(marker, rows)?;
        assert_eq!(report.submitted(), 3, "submitted is exact");
        assert_eq!(report.changed(), 1, "the one empty tuple entered once");
        Ok(())
    })
    .expect("dyn lane")
    .unwrap();

    // mismatch it always was, judged before any count accrues.
    let wide: [&[Value]; 2] = [&[], &[Value::U64(1)]];
    let err = AcceptedCollection::from_value_rows(marker, fields, wide).expect_err("widthful row");
    assert!(
        matches!(
            err,
            Error::FactShape(FactShapeError::ArityMismatch {
                relation,
                mismatch: Mismatch {
                    witnessed: 1,
                    required: 0,
                },
            }) if relation == marker
        ),
        "{err:?}"
    );
}

#[test]
fn the_collection_builder_is_the_one_shape_judgment() {
    let schema = crate::schema::ValidateDescriptor::validate(crate::Theory::descriptor(OneRep))
        .expect("valid");
    let rel = Sample::RELATION;
    let fields = schema.relation(rel).fields();
    let type_mismatch = |err: Error, field: u16| {
        assert!(
            matches!(
                err,
                Error::FactShape(FactShapeError::TypeMismatch { relation, field: f })
                    if relation == rel && f == FieldId(field)
            ),
            "{err:?}"
        );
    };

    type_mismatch(
        CollectionBuilder::new(rel, fields)
            .push_u64(1)
            .expect_err("bool column"),
        0,
    );
    let mut builder = CollectionBuilder::new(rel, fields);
    builder.push_bool(true).expect("flag");
    type_mismatch(builder.push_i64(-1).expect_err("u64 column"), 1);
    builder.push_u64(7).expect("count");
    type_mismatch(builder.push_bool(false).expect_err("i64 column"), 2);
    builder.push_i64(-3).expect("delta");
    type_mismatch(builder.push_bytes(&[1, 2, 3]).expect_err("str column"), 3);
    builder.push_str("alpha").expect("memo");

    type_mismatch(builder.push_bytes(&[1, 2]).expect_err("bytes<3>"), 4);
    type_mismatch(builder.push_str("not-bytes").expect_err("bytes<3>"), 4);
    builder.push_bytes(&[1, 2, 3]).expect("tag");

    type_mismatch(
        builder
            .push_interval_i64(crate::Interval::<i64>::new(-1, 1).expect("nonempty"))
            .expect_err("interval<u64> column"),
        5,
    );
    builder
        .push_interval_u64(crate::Interval::<u64>::new(1, 5).expect("nonempty"))
        .expect("window");
    type_mismatch(
        builder
            .push_interval_u64(crate::Interval::<u64>::new(1, 5).expect("nonempty"))
            .expect_err("interval<i64> column"),
        6,
    );
    builder
        .push_interval_i64(crate::Interval::<i64>::new(-4, 4).expect("nonempty"))
        .expect("span");

    builder.push_bool(true).expect("second row opens");
    let err = builder.seal().expect_err("partial row");
    assert!(
        matches!(
            err,
            Error::FactShape(FactShapeError::ArityMismatch {
                relation,
                mismatch: Mismatch {
                    witnessed: 1,
                    required: 7,
                },
            }) if relation == rel
        ),
        "{err:?}"
    );

    let ok = sample_value_rows().remove(0);
    let short = vec![Value::Bool(true)];
    let err = AcceptedCollection::from_value_rows(rel, fields, [&ok, &short])
        .expect_err("arity per row first");
    assert!(
        matches!(
            err,
            Error::FactShape(FactShapeError::ArityMismatch {
                relation,
                mismatch: Mismatch {
                    witnessed: 1,
                    required: 7,
                },
            }) if relation == rel
        ),
        "{err:?}"
    );
    let mut wrong = ok.clone();
    wrong[3] = Value::U64(9);
    let err = AcceptedCollection::from_value_rows(rel, fields, [&ok, &wrong])
        .expect_err("type-kind per cell");
    type_mismatch(err, 3);
}
