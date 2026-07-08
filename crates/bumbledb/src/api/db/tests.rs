use super::*;
use crate::ir::Value;
use crate::schema::{FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValueType};
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
            constraints: vec![],
        }],
    }
    .validate()
    .expect("fixture")
}

/// The reader cache (docs/silicon/12), semantics pinned:
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
    let err: Result<()> = db.read(|_| Err(crate::error::Error::Overflow { find: 7 }));
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

/// PRD 01 (docs/hardening): the delete path never mints — a typo'd
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
