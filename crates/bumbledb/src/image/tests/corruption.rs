use super::{populated, schema, R};
use crate::error::{CorruptionError, Error};
use crate::image::build;
use crate::storage::env::Environment;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};
use crate::storage::read;
use crate::testutil::TempDir;

#[test]
fn scan_corruption_aborts_the_build() {
    let dir = TempDir::new("image-corrupt");
    let schema = schema();
    let env = populated(&dir, &schema);
    {
        let victim = {
            let txn = env.read_txn().expect("txn");
            read::scan(&txn, &schema, R)
                .expect("scan")
                .map(|e| e.expect("ok").0)
                .max()
                .expect("nonempty")
        };
        let mut wtxn = env.write_txn().expect("txn");
        let mut key: KeyBuf = [0; MAX_KEY];
        let len = keys::fact_key(&mut key, R, victim);
        env.data()
            .put(wtxn.raw_mut(), &key[..len], &[0xFF])
            .expect("put");
        wtxn.commit().expect("commit");
    }
    let txn = env.read_txn().expect("txn");
    let err = build(&txn, &schema, R).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::WrongFactWidth { .. })
        ),
        "{err:?}"
    );
}

/// A corrupt (astronomical) stored `S` row
/// count is typed Corruption before any slab allocation is
/// attempted — never an OOM abort.
#[test]
fn a_corrupt_row_count_errors_before_allocating() {
    let dir = TempDir::new("image-corrupt-row-count");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    {
        let mut wtxn = env.write_txn().expect("txn");
        let mut key: KeyBuf = [0; MAX_KEY];
        let len = keys::stat_key(&mut key, R, keys::StatKind::RowCount);
        env.data()
            .put(
                wtxn.raw_mut(),
                &key[..len],
                u64::MAX.to_le_bytes().as_slice(),
            )
            .expect("plant");
        wtxn.commit().expect("commit");
    }
    let txn = env.read_txn().expect("txn");
    let err = build(&txn, &schema, R).map(|_| ()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::MalformedValue("S row count"))
        ),
        "{err:?}"
    );
}
