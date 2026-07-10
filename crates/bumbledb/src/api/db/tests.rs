use super::*;
use crate::error::{Error, FactShapeError};
use crate::ir::Value;
use crate::schema::{
    FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, StatementDescriptor,
    StatementId, ValueType,
};
use crate::testutil::TempDir;

/// Named(name str) — a string-carrying relation for dictionary tests.
fn named_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Named".into(),
            fields: vec![FieldDescriptor {
                name: "name".into(),
                value_type: ValueType::String,
                generation: Generation::None,
            }],
        }],
        statements: vec![],
    }
    .validate()
    .expect("fixture")
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
    let schema = named_schema();
    let db = Db::create(dir.path(), &schema).expect("create");
    let named = RelationId(0);
    let count_named = |snap: &Snapshot<'_>| -> Result<u64> {
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

fn dict_entries(db: &Db<'_>) -> u64 {
    let rtxn = db.env.read_txn().expect("txn");
    db.env.dict().len(rtxn.raw()).expect("len")
}

/// The delete path never mints — a typo'd
/// delete leaves `_dict` byte-identical, at the storage level.
#[test]
fn a_typo_delete_leaves_the_dictionary_unchanged() {
    let dir = TempDir::new("db-mint-free-dict");
    let schema = named_schema();
    let db = Db::create(dir.path(), &schema).expect("create");
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
fn entry_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
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
    .validate()
    .expect("fixture")
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
    let schema = entry_schema();
    let db = Db::create(dir.path(), &schema).expect("create");

    db.write(|tx| {
        // Insert, then read back through the pending delta (the key
        // string exists only as a provisional intern id here).
        assert!(tx.insert_dyn(ENTRY, &entry("a", 1))?);
        assert_eq!(
            tx.get_dyn(ENTRY, ENTRY_KEY, &[Value::String("a".as_bytes().into())])?,
            Some(entry("a", 1))
        );
        // Delete: the guard map records absence.
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
    let schema = entry_schema();
    let db = Db::create(dir.path(), &schema).expect("create");
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
    let schema = entry_schema();
    let db = Db::create(dir.path(), &schema).expect("create");
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
    let schema = entry_schema();
    let db = Db::create(dir.path(), &schema).expect("create");
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

/// S(id serial, v) — the serial-minting relation for witness tests.
fn serial_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "S".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                    generation: Generation::Serial,
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
    .validate()
    .expect("fixture")
}

/// What the `schema!` macro would generate for `id: u64 as SId, serial` —
/// the typed mint path's proof-carrying newtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SId(u64);

impl Serial for SId {
    const RELATION: RelationId = RelationId(0);
    const FIELD: FieldId = FieldId(0);
    fn from_serial(raw: u64) -> Self {
        Self(raw)
    }
    fn serial(self) -> u64 {
        self.0
    }
}

/// The resolver is the one checking boundary of the untyped mint path:
/// ids and generation are data, so every mis-aimed resolution is a typed
/// `FactShape` error, never a panic.
#[test]
fn serial_field_rejects_non_witnesses_with_typed_errors() {
    let schema = serial_schema();
    assert_eq!(
        schema.serial_field(RelationId(0), FieldId(1)).unwrap_err(),
        FactShapeError::NotASerialField {
            relation: RelationId(0),
            field: FieldId(1),
        }
    );
    assert_eq!(
        schema.serial_field(RelationId(9), FieldId(0)).unwrap_err(),
        FactShapeError::UnknownRelation {
            relation: RelationId(9),
        }
    );
    assert_eq!(
        schema.serial_field(RelationId(0), FieldId(9)).unwrap_err(),
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
    let schema = serial_schema();
    let id_field = schema
        .serial_field(RelationId(0), FieldId(0))
        .expect("serial field");
    let db = Db::create(dir.path(), &schema).expect("create");
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

/// A mid-stream bulk-load failure surfaced through `?` (the
/// `From<BulkLoadError> for Error` conversion) still exposes the
/// committed count — the resumability payload the type exists for.
#[test]
fn a_bulk_load_error_keeps_its_committed_count_through_question_mark() {
    let dir = TempDir::new("db-bulk-load-count");
    let schema = named_schema();
    let db = Db::create(dir.path(), &schema).expect("create");
    let named = RelationId(0);

    // Exactly one full chunk of distinct facts commits; the mis-shaped
    // fact (wrong arity) fails the second chunk whole.
    let facts: Vec<Vec<Value>> = (0..BULK_CHUNK as u64)
        .map(|i| vec![Value::String(format!("v{i}").into_bytes().into())])
        .chain(std::iter::once(vec![]))
        .collect();
    let surfaced = (|| -> Result<u64> { Ok(db.bulk_load(named, facts)?) })();
    match surfaced.unwrap_err() {
        Error::BulkLoad { committed, error } => {
            assert_eq!(committed, BULK_CHUNK as u64, "the whole first chunk");
            assert!(matches!(*error, Error::FactShape(_)), "{error:?}");
        }
        other => panic!("expected Error::BulkLoad, got {other:?}"),
    }
}
