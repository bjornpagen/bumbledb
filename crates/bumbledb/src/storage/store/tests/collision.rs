//! Forced constant fingerprints through insert, contains, delete, judgment
//! and export: a collision adds lookup work; it never merges two rows,
//! hides a competing proposal, or loses a row.

use super::*;

const FORCED: [u8; 16] = [0xCC; 16];

fn forced_store(path: &std::path::Path) -> Store {
    Store::create_forced_fingerprint(path, &schema(), FORCED).expect("forced-fingerprint store")
}

fn exported(store: &Store) -> Vec<(RelationId, Vec<u8>)> {
    let snapshot = store.snapshot(&work()).expect("snapshot");
    let mut rows = Vec::new();
    snapshot
        .export(&work(), &mut |relation, row| {
            rows.push((relation, row.to_vec()));
            Ok(())
        })
        .expect("export");
    rows
}

#[test]
fn colliding_rows_stay_distinct_through_insert_contains_delete() {
    let (_dir, path) = store_dir("collision-crud");
    let store = forced_store(&path);
    let long_body = "L".repeat(4096);
    let rows = [tag("alpha"), tag("beta"), tag(&long_body)];
    let adds: Vec<_> = rows.iter().map(|values| (TAG, values.clone())).collect();
    commit_changes(&store, &change_set(&schema(), &adds, &[]));
    let snapshot = store.snapshot(&work()).expect("snapshot");
    assert_eq!(snapshot.row_count(TAG).expect("count"), 3);
    for values in &rows {
        assert!(
            snapshot
                .contains(TAG, &row_bytes(&schema(), TAG, values), &work())
                .expect("contains")
        );
    }
    assert!(
        !snapshot
            .contains(TAG, &row_bytes(&schema(), TAG, &tag("absent")), &work())
            .expect("absent probe")
    );
    drop(snapshot);
    commit_changes(&store, &change_set(&schema(), &[], &[(TAG, tag("beta"))]));
    let snapshot = store.snapshot(&work()).expect("snapshot after delete");
    assert_eq!(snapshot.row_count(TAG).expect("count"), 2);
    for (values, present) in [
        (tag("alpha"), true),
        (tag("beta"), false),
        (tag(&long_body), true),
    ] {
        assert_eq!(
            snapshot
                .contains(TAG, &row_bytes(&schema(), TAG, &values), &work())
                .expect("probe"),
            present
        );
    }
}

#[test]
fn export_orders_collision_buckets_by_bytes_whatever_the_insertion_order() {
    for relation in [NOTE, TAG] {
        let make = |letter: &str| {
            let body = letter.repeat(4096);
            (
                relation,
                if relation == NOTE {
                    note(7, &body)
                } else {
                    tag(&body)
                },
            )
        };
        let mut exports = Vec::new();
        for order in [["c", "a", "b"], ["b", "c", "a"]] {
            let (_dir, path) = store_dir("collision-export");
            let store = forced_store(&path);
            for letter in order {
                commit_changes(&store, &change_set(&schema(), &[make(letter)], &[]));
            }
            exports.push(exported(&store));
        }
        let mut expected: Vec<_> = ["a", "b", "c"]
            .into_iter()
            .map(|letter| {
                let (relation, values) = make(letter);
                (relation, row_bytes(&schema(), relation, &values))
            })
            .collect();
        expected.sort();
        assert_eq!(exports[0], expected);
        assert_eq!(exports[1], expected, "ordinals never enter the export");
    }
}

#[test]
fn judgment_sees_every_competing_proposal_in_one_bucket() {
    let (_dir, path) = store_dir("collision-judgment");
    let store = forced_store(&path);
    commit_changes(
        &store,
        &change_set(
            &schema(),
            &[(NOTE, note(1, "one")), (NOTE, note(2, "two"))],
            &[],
        ),
    );
    let violations = judged_commit(
        &store,
        &schema(),
        &change_set(&schema(), &[(NOTE, note(1, "rival"))], &[]),
    )
    .expect_err("the id key rejects");
    assert_eq!(violations[0].examples.len(), 2);
    judged_commit(
        &store,
        &schema(),
        &change_set(&schema(), &[(NOTE, note(3, "three"))], &[]),
    )
    .expect("a distinct id admits despite the shared bucket");
}
