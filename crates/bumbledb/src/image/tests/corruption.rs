use super::{populated, schema, R};
use crate::error::{CorruptionError, Error};
use crate::image::build;
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

/// The reopen-trust ceiling: a hand-corrupted `S` row count that is
/// large but plausible (2^40 passes every checked size computation)
/// exceeds the `_data` entry-count witness and is typed
/// `CounterDesync` before any slab allocation — never a multi-terabyte
/// `vec!` / OOM abort. The test returning at all is the process-alive
/// assertion.
#[test]
fn a_corrupt_row_count_above_the_store_witness_is_counter_desync() {
    const CLAIMED: u64 = 1 << 40;
    let dir = TempDir::new("image-corrupt-row-count");
    let schema = schema();
    let env = populated(&dir, &schema);
    {
        let mut wtxn = env.write_txn().expect("txn");
        let mut key: KeyBuf = [0; MAX_KEY];
        let len = keys::stat_key(&mut key, R, keys::StatKind::RowCount);
        env.data()
            .put(
                wtxn.raw_mut(),
                &key[..len],
                CLAIMED.to_le_bytes().as_slice(),
            )
            .expect("plant");
        wtxn.commit().expect("commit");
    }
    let txn = env.read_txn().expect("txn");
    let err = build(&txn, &schema, R).map(|_| ()).unwrap_err();
    match err {
        Error::Corruption(CorruptionError::CounterDesync {
            relation,
            claimed,
            witness,
        }) => {
            assert_eq!(relation, R);
            assert_eq!(claimed, CLAIMED);
            // The witness over-approximates the 10 real rows (the DBI
            // spans every namespace) but stays store-sized.
            assert!((10..CLAIMED).contains(&witness), "witness {witness}");
        }
        other => panic!("expected CounterDesync, got {other:?}"),
    }
}
