//! The sweeper over the real store: a lawful store is coherent, every forged
//! physical desync is its own typed finding, and a state no writer judged is
//! convicted by the complete judgment.

use bumbledb_theory::schema::{
    FieldDescriptor, RelationId, SchemaDescriptor, StatementDescriptor, ValueType,
};

use crate::storage::store::VerifyCorruption;
use crate::storage::store::format::{K_NEXT_ROW_ID, RowId, relation_key};
use crate::storage::store::keys;
use crate::testutil::TempDir;
use crate::work::WorkContext;
use crate::{Db, Theory, Value};

const ENTRY: RelationId = RelationId(0);

#[derive(Clone, Copy)]
struct Ledger;

impl Theory for Ledger {
    fn descriptor(self) -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![bumbledb_theory::schema::RelationDescriptor {
                name: "entry".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "name".into(),
                        value_type: ValueType::String,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                    },
                ],
                extension: None,
            }],
            statements: vec![StatementDescriptor::Functionality {
                relation: ENTRY,
                projection: Box::new([bumbledb_theory::schema::FieldId(0)]),
            }],
        }
    }
}

fn row(name: &str, amount: i64) -> Vec<Value> {
    vec![Value::String(name.into()), Value::I64(amount)]
}

fn work() -> WorkContext {
    WorkContext::new()
}

fn create(dir: &TempDir) -> Db<Ledger> {
    Db::create(dir.path(), Ledger, work())
        .expect("create")
        .expect("empty theory admits")
}

fn insert(db: &Db<Ledger>, rows: impl IntoIterator<Item = Vec<Value>>) {
    db.write(work(), |tx| tx.insert_dyn(ENTRY, rows).map(|_| ()))
        .expect("write")
        .unwrap();
}

/// Forge raw bytes through the store's own handles.
fn forge(
    db: &Db<Ledger>,
    f: impl FnOnce(&crate::storage::store::store_env::StoreInner, &mut heed::RwTxn<'_>),
) {
    let store = &db.store;
    let mut wtxn = store.gated_write_txn(&work()).expect("fixture txn");
    f(&store.inner, &mut wtxn.txn);
    wtxn.commit().expect("fixture commit");
}

#[test]
fn a_lawful_store_sweeps_coherent_after_mixed_commits() {
    let dir = TempDir::new("verify-coherent");
    let db = create(&dir);
    insert(&db, [row("a", 1), row("b", 2)]);
    db.write(work(), |tx| {
        tx.delete_dyn(ENTRY, [row("a", 1)])?;
        tx.insert_dyn(ENTRY, [row("c", 3)])?;
        Ok(())
    })
    .expect("write")
    .unwrap();
    let report = db.verify_store(&work()).expect("sweep");
    assert!(report.is_coherent(), "{report:?}");
}

#[test]
fn a_moved_row_is_a_foreign_home() {
    let dir = TempDir::new("verify-foreign-home");
    let db = create(&dir);
    insert(&db, [row("a", 1)]);
    forge(&db, |inner, txn| {
        let (key, value) = {
            let mut iter = inner.rows.iter(txn).expect("iter");
            let (key, value) = iter.next().expect("one row").expect("entry");
            (key.to_vec(), value.to_vec())
        };
        let parsed = keys::parse(&key).expect("key");
        let mut moved = parsed.route;
        moved[0] ^= 0xFF;
        inner.rows.delete(txn, &key).expect("delete");
        inner
            .rows
            .put(txn, &keys::entry(parsed.prefix, &moved, parsed.row), &value)
            .expect("put");
    });
    let report = db.verify_store(&work()).expect("sweep");
    assert!(
        report
            .corruption
            .iter()
            .any(|found| matches!(found, VerifyCorruption::ForeignRowHome { relation, .. } if *relation == ENTRY)),
        "{report:?}"
    );
}

#[test]
fn stale_counts_a_behind_ratchet_and_a_dangling_entry_are_distinct_findings() {
    let dir = TempDir::new("verify-counters");
    let db = create(&dir);
    insert(&db, [row("a", 1), row("b", 2)]);
    forge(&db, |inner, txn| {
        let mut meta = [0u8; 16];
        meta[..8].copy_from_slice(&7u64.to_be_bytes());
        inner
            .meta
            .put(txn, &relation_key(ENTRY).expect("key"), &meta)
            .expect("count");
        inner
            .meta
            .put(txn, K_NEXT_ROW_ID, &1u64.to_be_bytes())
            .expect("ratchet");
        inner
            .rows
            .put(
                txn,
                &keys::entry([0, 0], &[0xAB; keys::HOME_LEN], RowId(9_999)),
                b"not a canonical row",
            )
            .expect("garbage row");
    });
    let report = db.verify_store(&work()).expect("sweep");
    assert!(
        report.corruption.iter().any(|found| matches!(
            found,
            VerifyCorruption::RowCountMismatch { relation, stored: 7, counted: 3 } if *relation == ENTRY
        )),
        "{report:?}"
    );
    assert!(
        report
            .corruption
            .iter()
            .any(|found| matches!(found, VerifyCorruption::RowIdRatchetBehind { next: 1, .. })),
        "{report:?}"
    );
    assert!(
        report
            .corruption
            .iter()
            .any(|found| matches!(found, VerifyCorruption::MalformedRow { row: 9_999, .. })),
        "{report:?}"
    );
    assert!(
        report.violations.is_none(),
        "structural faults skip judgment"
    );
}

#[test]
fn the_complete_judgment_convicts_a_state_no_writer_judged() {
    let dir = TempDir::new("verify-judgment");
    let db = create(&dir);
    let changes = {
        let mut builder = crate::ChangeSet::builder(db.schema(), work());
        builder.insert(ENTRY, &row("dup", 1)).expect("draft");
        builder.insert(ENTRY, &row("dup", 2)).expect("draft");
        builder.finish().expect("sealed")
    };
    let context = work();
    let mut session = db.host_writer(&context).expect("writer");
    session
        .apply_decided(std::slice::from_ref(&changes))
        .expect("apply")
        .seal(crate::host::HostChanges::NONE)
        .expect("seal")
        .commit()
        .expect("commit");
    drop(session);
    let report = db.verify_store(&work()).expect("sweep");
    assert!(report.corruption.is_empty(), "{report:?}");
    let violations = report.violations.expect("the duplicate key is convicted");
    assert_eq!(violations.len(), 1);
}
