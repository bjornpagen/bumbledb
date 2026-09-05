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

fn encoded_note(id: u64, body: &str) -> Vec<u8> {
    crate::canonical::CanonicalRow::encode(
        schema().relation(NOTE).fields(),
        &note(id, body),
        &work(),
    )
    .expect("canonical note")
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
    let rows = [note(1, "alpha"), note(2, "beta"), note(3, &long_body)];
    commit_changes(
        &store,
        &change_set(
            &schema(),
            &rows
                .iter()
                .map(|values| (NOTE, values.clone()))
                .collect::<Vec<_>>(),
            &[],
        ),
    );
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 3);
    for values in &rows {
        let bytes = crate::canonical::CanonicalRow::encode(
            schema().relation(NOTE).fields(),
            values,
            &work(),
        )
        .expect("row")
        .as_bytes()
        .to_vec();
        assert!(snapshot.contains(NOTE, &bytes, &work()).expect("contains"));
    }
    // A fourth row that was never inserted misses despite sharing the
    // forced bucket: exact bytes decide, not the fingerprint.
    assert!(
        !snapshot
            .contains(NOTE, &encoded_note(4, "alpha"), &work())
            .expect("absent probe")
    );
    drop(snapshot);
    // Deleting one colliding row leaves the other bucket residents intact.
    commit_changes(
        &store,
        &change_set(&schema(), &[], &[(NOTE, note(2, "beta"))]),
    );
    let snapshot = store.snapshot(&work()).expect("snapshot after delete");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 2);
    assert!(
        snapshot
            .contains(NOTE, &encoded_note(1, "alpha"), &work())
            .expect("survivor 1")
    );
    assert!(
        !snapshot
            .contains(NOTE, &encoded_note(2, "beta"), &work())
            .expect("deleted")
    );
    assert!(
        snapshot
            .contains(NOTE, &encoded_note(3, &long_body), &work())
            .expect("survivor 3")
    );
}

#[test]
fn export_orders_a_collision_bucket_by_full_canonical_bytes() {
    let (_dir, path) = store_dir("collision-export");
    let store = forced_store(&path);
    // Insert out of canonical byte order; ids chosen so canonical BE bytes
    // order 1 < 2 < 3 regardless of insertion sequence.
    commit_changes(
        &store,
        &change_set(
            &schema(),
            &[
                (NOTE, note(3, "c")),
                (NOTE, note(1, "a")),
                (NOTE, note(2, "b")),
            ],
            &[],
        ),
    );
    let snapshot = store.snapshot(&work()).expect("snapshot");
    let mut exported = Vec::new();
    snapshot
        .export(&work(), &mut |relation, row| {
            assert_eq!(relation, NOTE);
            exported.push(row.to_vec());
            Ok(())
        })
        .expect("export");
    assert_eq!(exported.len(), 3);
    let mut sorted = exported.clone();
    sorted.sort();
    assert_eq!(exported, sorted, "bucket export is canonical-byte ordered");
    assert_eq!(exported[0], encoded_note(1, "a"));
    assert_eq!(exported[2], encoded_note(3, "c"));
}

#[test]
fn judgment_sees_every_competing_proposal_in_a_forced_bucket() {
    // ENG-005 physical precondition under forced determinant collisions:
    // unrelated notes share the determinant bucket, yet the judge's exact
    // recheck neither rejects unrelated rows nor misses true conflicts.
    let (_dir, path) = store_dir("collision-judgment");
    let store = forced_store(&path);
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "one"))], &[]),
    );
    // Unrelated id in the same forced bucket: admitted.
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
            panic!("forced bucket sharing must not reject unrelated ids: {rows:?}")
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
        Prepared::Admitted(_) => panic!("true duplicate id must reject under collisions"),
    }
}

#[test]
fn colliding_buckets_remain_enumerable_and_individually_deletable() {
    let (_dir, path) = store_dir("collision-bucket-enumeration");
    let store = forced_store(&path);
    let adds: Vec<_> = (0..16u64)
        .map(|id| (NOTE, note(id, &format!("row-{id}"))))
        .collect();
    commit_changes(&store, &change_set(&schema(), &adds, &[]));
    // Delete every even row individually; odd rows survive exactly.
    for id in (0..16u64).step_by(2) {
        commit_changes(
            &store,
            &change_set(&schema(), &[], &[(NOTE, note(id, &format!("row-{id}")))]),
        );
    }
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 8);
    let rows: Vec<(RowId, Vec<u8>)> = snapshot
        .rows(NOTE)
        .expect("cursor")
        .map(|entry| entry.map(|(id, bytes)| (id, bytes.to_vec())))
        .collect::<Result<_, _>>()
        .expect("rows");
    assert_eq!(rows.len(), 8);
    for id in (1..16u64).step_by(2) {
        assert!(
            snapshot
                .contains(NOTE, &encoded_note(id, &format!("row-{id}")), &work())
                .expect("odd survivor")
        );
    }
}
