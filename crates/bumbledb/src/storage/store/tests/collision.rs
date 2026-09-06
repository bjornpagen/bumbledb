//! HASH-02 / Q-COLLISION substrate: forced constant fingerprints through
//! insert, contains, delete, judgment and export. A collision adds lookup
//! work; it can never merge two facts, hide a competing proposal, or lose a
//! row. Long values above the LMDB key bound never enter a key.

use super::*;
use crate::storage::store::format::RowId;

const FORCED: [u8; 16] = [0xCC; 16];

fn forced_store(path: &std::path::Path) -> Store {
    Store::create_forced_fingerprint(path, &schema(), MapPolicy::default(), FORCED)
        .expect("forced-fingerprint store")
}

fn encoded_tag(body: &str) -> Vec<u8> {
    crate::canonical::CanonicalRow::encode(schema().relation(TAG).fields(), &tag(body), &work())
        .expect("canonical tag")
        .as_bytes()
        .to_vec()
}

#[test]
fn colliding_rows_stay_distinct_through_insert_contains_delete() {
    let (_dir, path) = store_dir("collision-crud");
    let store = forced_store(&path);
    // Three distinct rows, one forced bucket. One body is far above the
    // 511-byte LMDB key bound: long values live in row values, never keys.
    let long_body = "L".repeat(4096);
    let rows = [tag("alpha"), tag("beta"), tag(&long_body)];
    commit_changes(
        &store,
        &change_set(
            &schema(),
            &rows
                .iter()
                .map(|values| (TAG, values.clone()))
                .collect::<Vec<_>>(),
            &[],
        ),
    );
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(TAG).expect("count"), 3);
    for values in &rows {
        let bytes = crate::canonical::CanonicalRow::encode(
            schema().relation(TAG).fields(),
            values,
            &work(),
        )
        .expect("row")
        .as_bytes()
        .to_vec();
        assert!(snapshot.contains(TAG, &bytes, &work()).expect("contains"));
    }
    // A fourth row that was never inserted misses despite sharing the
    // forced bucket: exact bytes decide, not the fingerprint.
    assert!(
        !snapshot
            .contains(TAG, &encoded_tag("absent"), &work())
            .expect("absent probe")
    );
    drop(snapshot);
    // Deleting one colliding row leaves the other bucket residents intact.
    commit_changes(&store, &change_set(&schema(), &[], &[(TAG, tag("beta"))]));
    let snapshot = store.snapshot(&work()).expect("snapshot after delete");
    assert_eq!(snapshot.row_count(TAG).expect("count"), 2);
    assert!(
        snapshot
            .contains(TAG, &encoded_tag("alpha"), &work())
            .expect("survivor 1")
    );
    assert!(
        !snapshot
            .contains(TAG, &encoded_tag("beta"), &work())
            .expect("deleted")
    );
    assert!(
        snapshot
            .contains(TAG, &encoded_tag(&long_body), &work())
            .expect("survivor 3")
    );
}

#[test]
fn export_orders_home_and_fingerprint_collisions_with_bounded_memory_and_failure_cleanup() {
    use crate::work::{Resource, WorkError};

    for relation in [NOTE, TAG] {
        let (_dir, path) = store_dir("collision-export");
        let store = forced_store(&path);
        // TAG shares a forced fingerprint; NOTE shares one exact home.
        // The raw AdmitAll hook deliberately retains conflicting notes:
        // even an unadmitted bucket must export every distinct row exactly.
        let rows: Vec<_> = ["c", "a", "b"]
            .into_iter()
            .map(|letter| {
                let body = letter.repeat(4096);
                (
                    relation,
                    if relation == NOTE {
                        note(7, &body)
                    } else {
                        tag(&body)
                    },
                )
            })
            .collect();
        commit_changes(&store, &change_set(&schema(), &rows, &[]));
        let mut expected: Vec<_> = rows
            .iter()
            .map(|(_, values)| {
                crate::canonical::CanonicalRow::encode(
                    schema().relation(relation).fields(),
                    values,
                    &work(),
                )
                .unwrap()
                .as_bytes()
                .to_vec()
            })
            .collect();
        expected.sort();
        // Only two row-sized buffers fit, not this three-row bucket.
        let allowance = 2 * expected.iter().map(Vec::len).max().unwrap() as u64;
        let bounded = || {
            ExecutionPolicy {
                input_bytes: 1 << 20,
                working_bytes: allowance,
                scratch_bytes: 0,
                result_bytes: 0,
                rows: 100,
                work_units: 1 << 20,
                timeout: Duration::from_secs(60),
            }
            .start()
            .unwrap()
        };
        let snapshot = store.snapshot(&work()).expect("snapshot");
        let context = bounded();
        let mut exported = Vec::new();
        let report = snapshot
            .export(&context, &mut |found, row| {
                assert_eq!(found, relation);
                exported.push(row.to_vec());
                Ok(())
            })
            .expect("bounded export");
        assert_eq!(report.rows, 3);
        assert_eq!(exported, expected);
        assert_eq!(context.used(Resource::WorkingBytes), 0);
        let context = bounded();
        let mut calls = 0;
        let failed = snapshot.export(&context, &mut |_, _| {
            calls += 1;
            if calls == 2 {
                Err(StoreError::Allocation)
            } else {
                Ok(())
            }
        });
        assert!(matches!(failed, Err(StoreError::Allocation)));
        assert_eq!(calls, 2);
        assert_eq!(context.used(Resource::WorkingBytes), 0);
        let context = bounded();
        let mut calls = 0;
        let cancelled = snapshot.export(&context, &mut |_, _| {
            calls += 1;
            context.cancel();
            Ok(())
        });
        assert!(matches!(
            cancelled,
            Err(StoreError::Work(WorkError::Cancelled))
        ));
        assert_eq!(calls, 1);
        assert_eq!(context.used(Resource::WorkingBytes), 0);
    }
}

#[test]
fn judgment_sees_every_competing_proposal_in_the_same_exact_home() {
    // Exact scalar routes ignore the forced fingerprint. Unrelated keys
    // remain separate; conflicting rows must coexist in the same home
    // until the judge sees and rejects their complete proposed state.
    let (_dir, path) = store_dir("collision-judgment");
    let store = forced_store(&path);
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "one"))], &[]),
    );
    // An unrelated exact home is admitted.
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare(
            &change_set(&schema(), &[(NOTE, note(2, "two"))], &[]),
            &FirstFieldKey,
            &UniqueNoteId,
        )
        .expect("prepare unrelated")
    {
        Prepared::Admitted(prepared) => drop(
            prepared
                .seal(NO_HOST)
                .expect("seal")
                .commit()
                .expect("commit"),
        ),
        Prepared::Rejected(rows) => {
            panic!("unrelated exact homes must admit: {rows:?}")
        }
    }
    // A true duplicate id still rejects, with both rows as evidence.
    match owner
        .prepare(
            &change_set(&schema(), &[(NOTE, note(1, "one-conflicting"))], &[]),
            &FirstFieldKey,
            &UniqueNoteId,
        )
        .expect("prepare conflicting")
    {
        Prepared::Rejected(rows) => assert_eq!(rows.len(), 2),
        Prepared::Admitted(_) => panic!("true duplicate id must reject without overwriting a row"),
    }
}

#[test]
fn colliding_buckets_remain_enumerable_and_individually_deletable() {
    let (_dir, path) = store_dir("collision-bucket-enumeration");
    let store = forced_store(&path);
    let adds: Vec<_> = (0..16u64)
        .map(|id| (TAG, tag(&format!("row-{id}"))))
        .collect();
    commit_changes(&store, &change_set(&schema(), &adds, &[]));
    // Delete every even row individually; odd rows survive exactly.
    for id in (0..16u64).step_by(2) {
        commit_changes(
            &store,
            &change_set(&schema(), &[], &[(TAG, tag(&format!("row-{id}")))]),
        );
    }
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(TAG).expect("count"), 8);
    let rows: Vec<(RowId, Vec<u8>)> = snapshot
        .rows(TAG)
        .expect("cursor")
        .map(|entry| entry.map(|(locator, bytes)| (locator.id, bytes.to_vec())))
        .collect::<Result<_, _>>()
        .expect("rows");
    assert_eq!(rows.len(), 8);
    for id in (1..16u64).step_by(2) {
        assert!(
            snapshot
                .contains(TAG, &encoded_tag(&format!("row-{id}")), &work())
                .expect("odd survivor")
        );
    }
}

#[test]
fn export_bytes_ignore_local_ordinals_for_primary_conflicts_and_fingerprint_collisions() {
    let rows = [
        (NOTE, note(7, "third")),
        (TAG, tag("third")),
        (NOTE, note(7, "first")),
        (TAG, tag("first")),
        (NOTE, note(7, "second")),
        (TAG, tag("second")),
    ];
    let mut exports = Vec::new();
    let mut physical = Vec::new();
    for reverse in [false, true] {
        let (_dir, path) = store_dir("export-insertion-order");
        let store = forced_store(&path);
        let ordered: Vec<_> = if reverse {
            rows.iter().rev().collect()
        } else {
            rows.iter().collect()
        };
        for row in ordered {
            commit_changes(
                &store,
                &change_set(&schema(), std::slice::from_ref(row), &[]),
            );
        }
        let context = work();
        let snapshot = store.snapshot(&context).unwrap();
        let mut exported = Vec::new();
        snapshot
            .export(&context, &mut |relation, bytes| {
                exported.push((relation, bytes.to_vec()));
                Ok(())
            })
            .unwrap();
        exports.push(exported);
        physical.push(
            snapshot
                .rows(NOTE)
                .unwrap()
                .map(|entry| {
                    let (locator, row) = entry.unwrap();
                    assert_eq!(locator.home(), 7u64.to_be_bytes());
                    (locator.id, row.to_vec())
                })
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            store.inner.data.len(snapshot.read_txn()).unwrap(),
            9,
            "three primary rows plus three keyless rows and their memberships"
        );
    }
    assert_ne!(
        physical[0], physical[1],
        "the experiment actually permuted local row identities"
    );
    assert_eq!(
        exports[0], exports[1],
        "only logical rows determine exported bytes"
    );
    assert_eq!(exports[0].len(), rows.len());
}
