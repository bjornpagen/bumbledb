//! ENG-003 (E-SNAPSHOT) and ENG-006 (E-TEXT): one owned read transaction
//! supplies rows, generation, attachment and export; inline text dies with
//! its row while pinned snapshots stay exact.

use super::*;
use crate::storage::GenerationId;

fn commit_with_attachment(store: &Store, changes: &ChangeSet, attachment: &[u8]) -> StoreCommit {
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare(changes, &FirstFieldKey, &AdmitAll)
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => prepared
            .seal(HostChanges {
                records: &[],
                attachment: AttachmentChange::Put(attachment),
            })
            .expect("seal")
            .commit()
            .expect("commit"),
        Prepared::Rejected(never) => match never {},
    }
}

#[test]
fn snapshot_rows_generation_and_attachment_name_one_transaction() {
    let (_dir, path) = store_dir("snap-coherent");
    let store = create_default(&path);
    let first = commit_with_attachment(
        &store,
        &change_set(&schema(), &[(NOTE, note(1, "alpha"))], &[]),
        b"stamp-at-1",
    );
    let pinned = store.snapshot(&work()).expect("pinned");
    // A later commit changes rows AND attachment.
    let second = commit_with_attachment(
        &store,
        &change_set(&schema(), &[(NOTE, note(2, "beta"))], &[]),
        b"stamp-at-2",
    );
    assert!(second.generation > first.generation);
    // The pinned snapshot is exactly the first commit's world: its rows,
    // ITS generation, ITS attachment — no mixed view is representable.
    assert_eq!(pinned.generation(), first.generation);
    assert_eq!(pinned.row_count(NOTE).expect("count"), 1);
    assert_eq!(
        pinned.attachment().expect("attachment"),
        Some(b"stamp-at-1".as_slice())
    );
    let fresh = store.snapshot(&work()).expect("fresh");
    assert_eq!(fresh.generation(), second.generation);
    assert_eq!(fresh.row_count(NOTE).expect("count"), 2);
    assert_eq!(
        fresh.attachment().expect("attachment"),
        Some(b"stamp-at-2".as_slice())
    );
}

#[test]
fn export_is_coherent_under_a_commit_landed_mid_export() {
    let (_dir, path) = store_dir("snap-export-race");
    let store = create_default(&path);
    for id in 0..8u64 {
        commit_changes(
            &store,
            &change_set(&schema(), &[(NOTE, note(id, &format!("body-{id}")))], &[]),
        );
    }
    let pinned = store.snapshot(&work()).expect("pinned");
    let expected_generation = pinned.generation();
    let mut emitted = 0u64;
    let mut committed_midway = false;
    let context = work();
    let report = pinned
        .export(&context, &mut |relation, row| {
            assert_eq!(relation, NOTE);
            assert!(!row.is_empty());
            emitted += 1;
            if !committed_midway {
                // Deterministic pause substitute: land a concurrent commit
                // while the export is mid-stream (chapter 10, E-SNAPSHOT).
                committed_midway = true;
                commit_changes(
                    &store,
                    &change_set(&schema(), &[(NOTE, note(1_000, "late"))], &[]),
                );
            }
            Ok(())
        })
        .expect("export");
    // The export names its one snapshot: 8 rows, the pinned generation,
    // and no late row — even though a commit landed mid-export.
    assert_eq!(emitted, 8);
    assert_eq!(report.rows, 8);
    assert_eq!(report.generation, expected_generation);
    let fresh = store.snapshot(&work()).expect("fresh");
    assert_eq!(fresh.row_count(NOTE).expect("count"), 9);
}

#[test]
fn deleted_text_has_no_live_entry_after_delete_export_reopen() {
    // ENG-006 / E-TEXT: there is no dictionary; text lives inline in its
    // row and disappears from the live state with it.
    let (_dir, path) = store_dir("snap-text-lifetime");
    let unique = "unique-deleted-text-4d3a9c";
    {
        let store = create_default(&path);
        commit_changes(
            &store,
            &change_set(
                &schema(),
                &[(NOTE, note(7, unique)), (TAG, tag("keep"))],
                &[],
            ),
        );
        let pinned = store.snapshot(&work()).expect("pinned before delete");
        commit_changes(
            &store,
            &change_set(&schema(), &[], &[(NOTE, note(7, unique))]),
        );
        // The old pinned snapshot still resolves the deleted text exactly
        // (LMDB MVCC), while the live state no longer contains it.
        assert!(
            pinned
                .contains(NOTE, row_bytes(&note(7, unique)), &work())
                .expect("pinned contains")
        );
        let live = store.snapshot(&work()).expect("live");
        assert!(
            !live
                .contains(NOTE, row_bytes(&note(7, unique)), &work())
                .expect("live contains")
        );
    }
    // Reopen: the exported live state carries no deleted-text row. This is
    // live-state lifetime, not a secure-erasure claim about freed pages.
    let store = open_default(&path);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    let mut exported = Vec::new();
    snapshot
        .export(&work(), &mut |relation, row| {
            exported.push((relation, row.to_vec()));
            Ok(())
        })
        .expect("export");
    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0].0, TAG);
    for (_, row) in &exported {
        assert!(!contains_subslice(row, unique.as_bytes()));
    }
}

#[test]
fn there_is_no_dictionary_database_in_the_successor_store() {
    let (_dir, path) = store_dir("snap-no-dict");
    let store = create_default(&path);
    commit_changes(
        &store,
        &change_set(
            &schema(),
            &[(NOTE, note(1, "repeated")), (NOTE, note(2, "repeated"))],
            &[],
        ),
    );
    let snapshot = store.snapshot(&work()).expect("snapshot");
    // Repeated text is stored inline per row — set semantics still hold.
    assert_eq!(snapshot.row_count(NOTE).expect("count"), 2);
    // No `_dict` database exists in the environment.
    let rtxn = store.inner.env.read_txn().expect("read txn");
    let dict: Option<heed::Database<heed::types::Bytes, heed::types::Bytes>> = store
        .inner
        .env
        .open_database(&rtxn, Some("_dict"))
        .expect("open probe");
    assert!(dict.is_none());
}

#[test]
fn export_orders_by_relation_then_fingerprint_then_bytes() {
    let (_dir, path) = store_dir("snap-export-order");
    let store = create_default(&path);
    commit_changes(
        &store,
        &change_set(
            &schema(),
            &[
                (TAG, tag("zeta")),
                (NOTE, note(9, "nine")),
                (TAG, tag("alpha")),
                (NOTE, note(3, "three")),
            ],
            &[],
        ),
    );
    let snapshot = store.snapshot(&work()).expect("snapshot");
    let mut relations = Vec::new();
    snapshot
        .export(&work(), &mut |relation, _| {
            relations.push(relation);
            Ok(())
        })
        .expect("export");
    // Relation-major order; physical row ids and insert order are invisible.
    assert_eq!(relations, vec![NOTE, NOTE, TAG, TAG]);
}

#[test]
fn snapshot_age_is_exposed_for_growth_diagnostics() {
    let (_dir, path) = store_dir("snap-age");
    let store = create_default(&path);
    let snapshot = store.snapshot(&work()).expect("snapshot");
    std::thread::sleep(std::time::Duration::from_millis(5));
    assert!(snapshot.age() >= std::time::Duration::from_millis(5));
}

#[test]
fn generation_starts_at_zero_and_moves_only_on_change() {
    let (_dir, path) = store_dir("snap-generation");
    let store = create_default(&path);
    assert_eq!(
        store.committed_generation(&work()).expect("generation"),
        GenerationId::initial()
    );
    let first = commit_changes(&store, &change_set(&schema(), &[(NOTE, note(1, "x"))], &[]));
    assert!(first.changed);
    // Re-adding the same fact is a no-op: no generation movement.
    let noop = commit_changes(&store, &change_set(&schema(), &[(NOTE, note(1, "x"))], &[]));
    assert!(!noop.changed);
    assert_eq!(noop.generation, first.generation);
    assert_eq!(noop.application.added, 0);
}

#[test]
fn census_and_page_stats_read_one_coherent_snapshot() {
    // SPACE-01 seam (P14): the entry walk classifies by the real namespace
    // tags and the page stats stay self-consistent — both from the pinned
    // snapshot's one transaction, unmoved by a later commit.
    let (_dir, path) = store_dir("snap-census");
    let store = create_default(&path);
    let context = work();
    let mut owner = store.writer(&context).expect("writer");
    match owner
        .prepare(
            &change_set(
                &schema(),
                &[(NOTE, note(1, "alpha")), (NOTE, note(2, "beta"))],
                &[],
            ),
            &FirstFieldKey,
            &AdmitAll,
        )
        .expect("prepare")
    {
        Prepared::Admitted(prepared) => {
            prepared
                .seal(HostChanges {
                    records: &[HostRecordChange::Put {
                        key: b"receipt/1",
                        value: b"opaque-bytes",
                    }],
                    attachment: AttachmentChange::Put(b"census-stamp"),
                })
                .expect("seal")
                .commit()
                .expect("commit");
        }
        Prepared::Rejected(never) => match never {},
    }
    drop(owner);
    let pinned = store.snapshot(&work()).expect("pinned");
    // A later commit is invisible to the pinned walk.
    commit_changes(
        &store,
        &change_set(&schema(), &[(NOTE, note(3, "gamma"))], &[]),
    );
    let mut data_rows = 0u64;
    let mut data_membership = 0u64;
    let mut data_determinants = 0u64;
    let mut meta_host_records = 0u64;
    let mut meta_attachment = 0u64;
    let mut meta_other = 0u64;
    let mut live_key_bytes = 0u64;
    let mut live_value_bytes = 0u64;
    pinned
        .entry_census(&context, &mut |is_meta, tag, key_len, value_len| {
            live_key_bytes += key_len as u64;
            live_value_bytes += value_len as u64;
            if is_meta {
                if tag == crate::storage::store::format::K_HOST_RECORD_TAG {
                    meta_host_records += 1;
                } else if tag == crate::storage::store::format::K_ATTACHMENT[0] {
                    meta_attachment += 1;
                } else {
                    meta_other += 1;
                }
            } else {
                match tag {
                    crate::storage::store::keys::TAG_ROW => data_rows += 1,
                    crate::storage::store::keys::TAG_MEMBERSHIP => data_membership += 1,
                    crate::storage::store::keys::TAG_DETERMINANT => data_determinants += 1,
                    other => panic!("unclassified data namespace tag {other}"),
                }
            }
            Ok(())
        })
        .expect("census");
    // The pinned view has exactly the two committed rows, each with one
    // membership entry and (this fixture indexes the key statement) one
    // determinant entry; the sealed host record and attachment are present.
    assert_eq!(data_rows, 2);
    assert_eq!(data_membership, 2);
    assert_eq!(data_determinants, 2);
    assert_eq!(meta_host_records, 1);
    assert_eq!(meta_attachment, 1);
    // Core meta entries exist (family, layout, ids, generation, counters).
    assert!(meta_other >= 5, "core meta entries walked: {meta_other}");
    assert!(live_key_bytes > 0 && live_value_bytes > 0);
    let stats = pinned.page_stats().expect("page stats");
    assert!(stats.page_size >= 4096 && stats.page_size.is_power_of_two());
    assert!(stats.depth >= 1);
    assert!(stats.leaf_pages >= 1);
    // Every walked entry is an LMDB data item in some tree; the tree entry
    // counts include the database directory's own records too.
    let walked = data_rows
        + data_membership
        + data_determinants
        + meta_host_records
        + meta_attachment
        + meta_other;
    assert!(stats.entries >= walked, "{} >= {walked}", stats.entries);
    // Distinct quantities stay distinct and self-consistent: the two meta
    // pages, every live tree page and the derived freelist together are the
    // populated page span, which the file length covers.
    let report = store.map_report(&context).expect("map report");
    let populated_pages =
        2 + stats.branch_pages + stats.leaf_pages + stats.overflow_pages + stats.free_pages;
    assert!(
        populated_pages * stats.page_size <= report.populated_file_bytes,
        "{populated_pages} pages of {} bytes exceed the {}-byte file",
        stats.page_size,
        report.populated_file_bytes
    );
    assert_eq!(u64::from(report.page_size), stats.page_size);
}

pub(super) fn row_bytes(values: &[Value]) -> &'static [u8] {
    // Encode once and leak: test helper for exact-contains probes.
    let encoded =
        crate::canonical::CanonicalRow::encode(schema().relation(NOTE).fields(), values, &work())
            .expect("canonical row");
    Box::leak(encoded.as_bytes().to_vec().into_boxed_slice())
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len().max(1))
        .any(|window| window == needle)
}
