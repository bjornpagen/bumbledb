use super::*;
use crate::error::{Error, FactShapeError};
use crate::ir::Value;
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
    let db = Db::create(dir.path(), named_schema()).expect("create");
    let named = RelationId(0);
    let count_named = |snap: &Snapshot<'_, SchemaDescriptor>| -> Result<u64> {
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
        tx.insert_dyn(named, &[Value::String("first".as_bytes().into())])
            .map(|_| ())
    })
    .expect("write");
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
            crate::error::OverflowKind::Aggregate { find: 7 },
        ))
    });
    assert!(err.is_err());
    let again = db.read(|snap| count_named(snap)).expect("read after error");
    assert_eq!(again, 1);

    // (d) reader-table hygiene under 10k reads interleaved with
    // writes (every write invalidates; every read re-parks).
    for i in 0..100u64 {
        db.write(|tx| {
            tx.insert_dyn(named, &[Value::String(format!("n{i}").as_bytes().into())])
                .map(|_| ())
        })
        .expect("write");
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
    let db = Db::create(dir.path(), named_schema()).expect("create");
    let named = RelationId(0);
    db.write(|tx| {
        tx.insert_dyn(named, &[Value::String("real".as_bytes().into())])
            .map(|_| ())
    })
    .expect("seed");
    let entries = dict_entries(&db);
    assert_eq!(entries, 2, "one value: forward + reverse entries");

    db.write(|tx| {
        let changed = tx.delete_dyn(named, &[Value::String("ghost".as_bytes().into())])?;
        assert!(!changed, "a never-interned value matches no fact");
        Ok(())
    })
    .expect("typo delete");
    assert_eq!(dict_entries(&db), entries, "the dictionary grew on a miss");

    // Deleting the real fact still works — the committed-dict arm.
    db.write(|tx| {
        let changed = tx.delete_dyn(named, &[Value::String("real".as_bytes().into())])?;
        assert!(changed);
        Ok(())
    })
    .expect("real delete");
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
    vec![Value::String(name.as_bytes().into()), Value::I64(amount)]
}

/// The dynamic read-your-writes matrix: every pre-commit `get_dyn` answer
/// equals the post-commit one (the final-state view the judgment phase
/// judges — `70-api.md`), including for a fact whose string key was
/// interned in this very transaction.
#[test]
fn get_dyn_reads_its_own_writes_exactly_as_a_later_transaction_does() {
    let dir = TempDir::new("db-get-dyn-ryw");
    let db = Db::create(dir.path(), entry_schema()).expect("create");

    db.write(|tx| {
        // Insert, then read back through the pending delta (the key
        // string exists only as a provisional intern id here).
        assert!(tx.insert_dyn(ENTRY, &entry("a", 1))?);
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".as_bytes().into())])?,
            Some(entry("a", 1))
        );
        // Delete: the determinant map records absence.
        assert!(tx.delete_dyn(ENTRY, &entry("a", 1))?);
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".as_bytes().into())])?,
            None
        );
        // Delete + reinsert(modified): the key tuple re-establishes with
        // the new fact.
        assert!(tx.insert_dyn(ENTRY, &entry("a", 2))?);
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".as_bytes().into())])?,
            Some(entry("a", 2))
        );
        Ok(())
    })
    .expect("write");

    // The post-commit answer is byte-identical.
    db.write(|tx| {
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".as_bytes().into())])?,
            Some(entry("a", 2))
        );
        Ok(())
    })
    .expect("read back");
}

/// Committed-state fallthrough: a fact committed in a prior transaction
/// and untouched in this delta is found through the `U` → `F` path.
#[test]
fn get_dyn_falls_through_to_committed_state() {
    let dir = TempDir::new("db-get-dyn-committed");
    let db = Db::create(dir.path(), entry_schema()).expect("create");
    db.write(|tx| tx.insert_dyn(ENTRY, &entry("seed", 42)).map(|_| ()))
        .expect("seed");

    db.write(|tx| {
        // Touch a *different* tuple so the delta is nonempty but the
        // probed key has no overlay.
        tx.insert_dyn(ENTRY, &entry("other", 1))?;
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("seed".as_bytes().into())])?,
            Some(entry("seed", 42))
        );
        Ok(())
    })
    .expect("read");
}

/// A never-interned string key value proves no fact carries it: `Ok(None)`
/// and the dictionary next-id is untouched (the delete-path mint-free
/// contract, extended to point reads).
#[test]
fn get_dyn_with_a_never_interned_key_answers_none_without_minting() {
    let dir = TempDir::new("db-get-dyn-mint-free");
    let db = Db::create(dir.path(), entry_schema()).expect("create");
    db.write(|tx| tx.insert_dyn(ENTRY, &entry("real", 1)).map(|_| ()))
        .expect("seed");
    let entries = dict_entries(&db);

    db.write(|tx| {
        assert_eq!(
            tx.get_dyn(
                ENTRY,
                ENTRY_KEY,
                &[Value::String("ghost".as_bytes().into())]
            )?,
            None
        );
        assert_eq!(
            tx.delta.dict_next(),
            None,
            "the point read minted a provisional id"
        );
        Ok(())
    })
    .expect("probe");
    assert_eq!(dict_entries(&db), entries, "the dictionary grew on a miss");
}

/// The dynamic surface is data: a wrong statement id, arity, or value
/// type is a typed `FactShape` error, never a panic.
#[test]
fn get_dyn_rejects_mis_shaped_requests_with_typed_errors() {
    let dir = TempDir::new("db-get-dyn-shape");
    let db = Db::create(dir.path(), entry_schema()).expect("create");
    db.write(|tx| {
        // Out-of-range statement id.
        let err = tx
            .get_dyn(
                ENTRY,
                StatementId(7),
                &[Value::String("x".as_bytes().into())],
            )
            .unwrap_err();
        assert!(
            matches!(
                err,
                Error::FactShape(FactShapeError::NotAKeyStatement {
                    relation: ENTRY,
                    statement: StatementId(7),
                })
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
                    expected: 1,
                    supplied: 2,
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
    .expect("probe");
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
    let db = Db::create(dir.path(), fresh_schema()).expect("create");
    assert_eq!(
        db.fresh_field(RelationId(0), FieldId(1)).unwrap_err(),
        FactShapeError::NotAFreshField {
            relation: RelationId(0),
            field: FieldId(1),
        }
    );
    assert_eq!(
        db.fresh_field(RelationId(9), FieldId(0)).unwrap_err(),
        FactShapeError::UnknownRelation {
            relation: RelationId(9),
        }
    );
    assert_eq!(
        db.fresh_field(RelationId(0), FieldId(9)).unwrap_err(),
        FactShapeError::UnknownField {
            relation: RelationId(0),
            field: FieldId(9),
        }
    );
}

/// Resolve once, mint per row: one witness mints across many `alloc_at`
/// calls, interleaves with the typed path in one sequence, and the
/// sequence persists across transactions.
#[test]
fn a_witness_mints_the_same_sequence_as_the_typed_path() {
    let dir = TempDir::new("db-alloc-witness");
    let db = Db::create(dir.path(), fresh_schema()).expect("create");
    let id_field = db
        .fresh_field(RelationId(0), FieldId(0))
        .expect("fresh field");
    db.write(|tx| {
        assert_eq!(tx.alloc_at(id_field)?, 0);
        assert_eq!(tx.alloc::<SId>()?, SId(1), "one sequence, two surfaces");
        assert_eq!(tx.alloc_at(id_field)?, 2);
        assert_eq!(tx.alloc_at(id_field)?, 3);
        assert_eq!(tx.alloc::<SId>()?, SId(4));
        Ok(())
    })
    .expect("mint");
    // A committed sequence never re-issues: the witness continues where
    // the first transaction stopped.
    db.write(|tx| {
        assert_eq!(tx.alloc_at(id_field)?, 5);
        Ok(())
    })
    .expect("mint again");
}

/// The panic gap, closed (`EscapedIdBurn`, the drop guard in
/// `db/write.rs`): the never-reissue law binds EVERY termination of a
/// write — a PANICKING closure included. `alloc` hands the id to the
/// host before the commit's fate is known
/// (`lean/Bumbledb/Txn/Fresh.lean: never_reissue_observable`), so the
/// unwound transaction persists no data but burns its escaped mint —
/// the api-level mirror of the storage-level
/// `fresh_ids_allocated_in_a_rejected_txn_are_burned`.
#[test]
fn a_panicking_write_burns_its_escaped_fresh_ids() {
    let dir = TempDir::new("db-panic-fresh-burn");
    let db = Db::create(dir.path(), fresh_schema()).expect("create");
    let id_field = db
        .fresh_field(RelationId(0), FieldId(0))
        .expect("fresh field");
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: Result<()> = db.write(|tx| {
            assert_eq!(tx.alloc_at(id_field)?, 0, "the id escapes to the host");
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
            tx.alloc_at(id_field)?,
            1,
            "0 was issued and never re-issues, the panic notwithstanding"
        );
        Ok(())
    })
    .expect("the writer surface survives the panic");
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
    drop(Db::create(dir.path(), named_schema()).expect("create"));
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
    let foreign = Db::create(foreign_dir.path(), fresh_schema()).expect("create foreign");
    let witness = foreign
        .fresh_field(RelationId(0), FieldId(0))
        .expect("fresh in ITS OWN schema");
    let dir = TempDir::new("db-foreign-witness");
    // A different schema at this store: field 0 of relation 0 is a
    // plain String column, not fresh.
    let db = Db::create(dir.path(), named_schema()).expect("create");
    let outcome = db.write(|tx| tx.alloc_at(witness).map(|_| ()));
    match outcome.unwrap_err() {
        Error::FactShape(FactShapeError::NotAFreshField { relation, field }) => {
            assert_eq!(relation, RelationId(0));
            assert_eq!(field, FieldId(0));
        }
        other => panic!("a foreign witness must refuse typed, not mint: {other:?}"),
    }
    // The aborted transaction persisted nothing — the store's clock
    // never moved.
    assert_eq!(db.generation().expect("generation").value(), 0);
}

/// A mid-stream bulk-load failure surfaced through `?` (the
/// `From<BulkLoadError> for Error` conversion) still exposes the
/// committed count — the resumability payload the type exists for.
#[test]
fn a_bulk_load_error_keeps_its_committed_count_through_question_mark() {
    let dir = TempDir::new("db-bulk-load-count");
    let db = Db::create(dir.path(), named_schema()).expect("create");
    let named = RelationId(0);

    // Exactly one full chunk of distinct facts commits; the mis-shaped
    // fact (wrong arity) fails the second chunk whole.
    let facts: Vec<Vec<Value>> = (0..BULK_CHUNK as u64)
        .map(|i| vec![Value::String(format!("v{i}").into_bytes().into())])
        .chain(std::iter::once(vec![]))
        .collect();
    let surfaced: Result<u64> = db.bulk_load_dyn(named, facts).map_err(Error::from);
    match surfaced.unwrap_err() {
        Error::BulkLoad { committed, error } => {
            assert_eq!(committed, BULK_CHUNK as u64, "the whole first chunk");
            assert!(matches!(*error, Error::FactShape(_)), "{error:?}");
        }
        other => panic!("expected Error::BulkLoad, got {other:?}"),
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
    let db = Db::create(dir.path(), closed_schema()).expect("create");
    let currency = RelationId(0);

    let insert = db.write(|tx| tx.insert_dyn(currency, &[Value::U64(9)]).map(|_| ()));
    assert!(matches!(
        insert,
        Err(Error::ClosedRelationWrite { relation }) if relation == currency
    ));
    let delete = db.write(|tx| tx.delete_dyn(currency, &[Value::U64(2)]).map(|_| ()));
    assert!(matches!(
        delete,
        Err(Error::ClosedRelationWrite { relation }) if relation == currency
    ));

    // `bulk_load` shares `insert_dyn`'s per-fact entry: the first fact is
    // refused and no chunk commits.
    let bulk = db
        .bulk_load_dyn(currency, vec![vec![Value::U64(9)]])
        .expect_err("closed relations refuse bulk loads");
    assert_eq!(bulk.committed, 0);
    assert!(matches!(
        bulk.error,
        Error::ClosedRelationWrite { relation } if relation == currency
    ));

    // The delta stayed empty: swallowing the refusal commits nothing —
    // no generation movement, no stored rows.
    let before = db.generation().expect("generation");
    db.write(|tx| {
        assert!(matches!(
            tx.insert_dyn(currency, &[Value::U64(9)]),
            Err(Error::ClosedRelationWrite { .. })
        ));
        Ok(())
    })
    .expect("the refusal is the operation's, not the transaction's");
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
    let db = Db::create(dir.path(), closed_schema()).expect("create");
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
            Err(Error::FactShape(FactShapeError::UnknownRelation { .. }))
        ));
        assert!(matches!(
            tx.get_dyn(currency, StatementId(9), &[Value::U64(0)]),
            Err(Error::FactShape(FactShapeError::NotAKeyStatement { .. }))
        ));
        assert!(matches!(
            tx.get_dyn(currency, auto_key, &[]),
            Err(Error::FactShape(FactShapeError::ArityMismatch { .. }))
        ));
        Ok(())
    })
    .expect("write");
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
    let db = Db::create(dir.path(), fresh_schema()).expect("create");
    let rel = RelationId(0);
    let field = bumbledb_theory::schema::FieldId(0);

    // Commit 1: an explicit id well past anything ever minted.
    db.write(|tx| {
        tx.insert_dyn(rel, &[Value::U64(41), Value::U64(1)])
            .map(|_| ())
    })
    .expect("explicit-id write");

    // Commit 2: the next mint is strictly greater — never a collision.
    let witness = db.fresh_field(rel, field).expect("fresh field");
    let minted = db
        .write(|tx| {
            let id = tx.alloc_at(witness)?;
            tx.insert_dyn(rel, &[Value::U64(id), Value::U64(2)])?;
            Ok(id)
        })
        .expect("minting write");
    assert!(minted > 41, "minted {minted} must exceed the copied id 41");

    // And the ratchet survives a reopen: the mark was flushed to `Q`,
    // not just held in memory.
    drop(db);
    let db = Db::open(dir.path(), fresh_schema()).expect("reopen");
    let witness = db.fresh_field(rel, field).expect("fresh field");
    let after_reopen = db
        .write(|tx| tx.alloc_at(witness))
        .expect("mint after reopen");
    assert!(
        after_reopen > minted,
        "post-reopen mint {after_reopen} must exceed {minted}"
    );
}
