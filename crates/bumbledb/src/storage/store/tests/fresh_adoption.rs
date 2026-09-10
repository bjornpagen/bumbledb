//! Fresh-destination adoption: metadata-only history refuses.

use super::*;
use crate::schema::{
    FieldDescriptor, RelationDescriptor, Schema, SchemaDescriptor, StatementDescriptor,
};
use crate::storage::store::judge_bridge::{SchemaJudge, UnindexedRows};
use crate::storage::store::{AttachmentChange, HostChanges, HostRecordChange};
use bumbledb_theory::schema::{FieldId, RelationId};

fn keyed_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Item".into(),
            fields: vec![FieldDescriptor {
                name: "id".into(),
                value_type: ValueType::U64,
            }],
            extension: None,
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::from([FieldId(0)]),
        }],
    }
    .validate()
    .expect("valid")
}

#[test]
fn metadata_only_destination_refuses_adoption() {
    let dir = TempDir::new("fresh-refuse-meta");
    let schema = keyed_schema();
    let (dest, fresh) = Store::create(&dir.path().join("dest"), &schema, MapPolicy::default())
        .expect("create dest");
    let (source, _fresh) = Store::create(&dir.path().join("source"), &schema, MapPolicy::default())
        .expect("create source");
    let work = work();

    {
        let mut owner = dest.writer(&work).expect("writer");
        let changes = ChangeSet::builder(&schema, work.clone())
            .finish()
            .expect("empty");
        let prepared = match owner
            .prepare_incremental(
                crate::schema::judge::LawfulParent::established(),
                &changes,
                &UnindexedRows,
                &SchemaJudge::new(&schema),
            )
            .expect("prepare")
        {
            Prepared::Admitted(p) => p,
            Prepared::Rejected { rejection: v, .. } => panic!("{v:?}"),
        };
        prepared
            .seal(HostChanges {
                records: &[HostRecordChange::Put {
                    key: b"receipt",
                    value: b"noop",
                }],
                attachment: AttachmentChange::Keep,
            })
            .expect("seal")
            .commit()
            .expect("commit");
    }

    let snapshot = source.snapshot(&work).expect("snapshot");
    let err = dest
        .adopt_snapshot(&snapshot, fresh, &UnindexedRows, &work)
        .expect_err("metadata-only destination must refuse");
    assert!(
        matches!(err, StoreError::DestinationExists { .. }),
        "typed refusal, got {err:?}"
    );
}

#[test]
fn fresh_create_adopts_complete_snapshot() {
    let dir = TempDir::new("fresh-adopt-ok");
    let schema = keyed_schema();
    let (source, _fresh) = Store::create(&dir.path().join("source"), &schema, MapPolicy::default())
        .expect("create source");
    let work = work();
    commit_row(&source, &schema, &work, 7);
    let snapshot = source.snapshot(&work).expect("snapshot");

    let (dest, fresh) = Store::create(&dir.path().join("dest"), &schema, MapPolicy::default())
        .expect("create dest");
    dest.adopt_snapshot(&snapshot, fresh, &UnindexedRows, &work)
        .expect("fresh destination adopts");
    let snap = dest.snapshot(&work).expect("read dest");
    assert_eq!(snap.row_count(RelationId(0)).expect("count"), 1);
    drop(snap);
    commit_row(&dest, &schema, &work, 8);
    let snap = dest.snapshot(&work).expect("after adoption insert");
    assert_eq!(snap.row_count(RelationId(0)).expect("count"), 2);
    assert_eq!(snap.rows(RelationId(0)).expect("rows").count(), 2);
}

fn commit_row(store: &Store, schema: &Schema, work: &WorkContext, id: u64) {
    let mut builder = ChangeSet::builder(schema, work.clone());
    builder
        .insert(RelationId(0), &[Value::U64(id)])
        .expect("stage");
    let changes = builder.finish().expect("seal");
    let mut owner = store.writer(work).expect("writer");
    let prepared = match owner
        .prepare_incremental(
            crate::schema::judge::LawfulParent::established(),
            &changes,
            &UnindexedRows,
            &SchemaJudge::new(schema),
        )
        .expect("prepare")
    {
        Prepared::Admitted(p) => p,
        Prepared::Rejected { rejection: v, .. } => panic!("{v:?}"),
    };
    prepared
        .seal(HostChanges {
            records: &[],
            attachment: AttachmentChange::Keep,
        })
        .expect("seal")
        .commit()
        .expect("commit");
}

type Entries = Vec<(Vec<u8>, Vec<u8>)>;

fn entries(snapshot: &super::super::OwnedSnapshot, metadata: bool) -> Entries {
    let inner = snapshot.store_inner();
    let range = if metadata {
        inner.meta.iter(snapshot.read_txn())
    } else {
        inner.data.iter(snapshot.read_txn())
    };
    range
        .expect("iterate physical entries")
        .map(|entry| {
            let (key, value) = entry.expect("physical entry");
            (key.to_vec(), value.to_vec())
        })
        .collect()
}

fn attach_receipt(store: &Store, schema: &Schema, context: &WorkContext, attachment: &[u8]) {
    let mut owner = store.writer(context).expect("writer");
    let empty = change_set(schema, &[], &[]);
    let prepared = match owner.prepare(&empty, &NoIndex, &AdmitAll).expect("prepare") {
        Prepared::Admitted(prepared) => prepared,
        Prepared::Rejected {
            rejection: never, ..
        } => match never {},
    };
    prepared
        .seal(HostChanges {
            records: &host_put(b"receipt/1", b"settled"),
            attachment: AttachmentChange::Put(attachment),
        })
        .expect("seal")
        .commit()
        .expect("commit");
}

#[test]
fn physical_compaction_preserves_pinned_indexes_metadata_and_sparse_row_ids() {
    use super::super::format::{K_NEXT_ROW_ID, K_STORE_ID, RowId};

    let dir = TempDir::new("physical-compact-coherent");
    let schema = schema();
    let context = work();
    let source = create_default(&dir.path().join("source"));
    commit_changes(
        &source,
        &change_set(
            &schema,
            &[
                (NOTE, note(1, "kept")),
                (NOTE, note(2, "removed")),
                (TAG, tag("removed")),
            ],
            &[],
        ),
    );
    commit_changes(
        &source,
        &change_set(
            &schema,
            &[],
            &[(NOTE, note(2, "removed")), (TAG, tag("removed"))],
        ),
    );
    attach_receipt(&source, &schema, &context, b"attached");
    let pinned = source.snapshot(&context).expect("pin source");
    let expected_data = entries(&pinned, false);
    assert!(
        expected_data.windows(2).all(|pair| pair[0].0 < pair[1].0),
        "the custom comparator preserves the byte order required by compact APPEND"
    );
    let expected_meta = entries(&pinned, true);
    let next_row = expected_meta
        .iter()
        .find(|(key, _)| key == K_NEXT_ROW_ID)
        .map(|(_, value)| u64::from_be_bytes(value.as_slice().try_into().expect("counter")))
        .expect("next row counter");
    commit_changes(
        &source,
        &change_set(&schema, &[(NOTE, note(99, "later"))], &[]),
    );

    let dest_path = dir.path().join("dest");
    let (dest, fresh) = Store::create(&dest_path, &schema, MapPolicy::default()).expect("dest");
    let destination_identity = dest.snapshot(&context).expect("new dest").identity().store;
    dest.compact_snapshot(&pinned, fresh, &context)
        .expect("compact pinned view");
    {
        let copied = dest.snapshot(&context).expect("copied snapshot");
        assert_eq!(entries(&copied, false), expected_data);
        assert_eq!(
            entries(&copied, true)
                .into_iter()
                .filter(|(key, _)| key != K_STORE_ID)
                .collect::<Entries>(),
            expected_meta
                .into_iter()
                .filter(|(key, _)| key != K_STORE_ID)
                .collect::<Entries>()
        );
        assert_eq!(copied.identity().store, destination_identity);
        assert_ne!(copied.identity().store, pinned.identity().store);
        assert_eq!(copied.generation(), pinned.generation());
    }
    drop(dest);
    let dest = Store::open(&dest_path, &schema, MapPolicy::default()).expect("reopen compacted");
    commit_changes(
        &dest,
        &change_set(&schema, &[(NOTE, note(100, "new"))], &[]),
    );
    let copied = dest.snapshot(&context).expect("after new insert");
    assert_eq!(copied.row_count(NOTE).expect("count"), 2);
    assert!(
        copied
            .rows(NOTE)
            .expect("rows")
            .any(|entry| entry.expect("row").0.id == RowId(next_row))
    );
    let after = entries(&copied, false);
    assert!(
        after.windows(2).all(|pair| pair[0].0 < pair[1].0),
        "reopened compacted trees retain the same order after insertion"
    );
    for entry in expected_data {
        assert!(
            after.contains(&entry),
            "a new insert must not overwrite copied rows/indexes"
        );
    }
}

#[test]
fn physical_compaction_grows_and_copies_overflow_values_without_reencoding() {
    let dir = TempDir::new("physical-compact-growth");
    let schema = schema();
    let context = work();
    let (source, _) =
        Store::create(&dir.path().join("source"), &schema, tiny_map()).expect("source");
    commit_changes(
        &source,
        &change_set(
            &schema,
            &[(NOTE, note(1, &"x".repeat(2 * 1024 * 1024)))],
            &[],
        ),
    );
    let pinned = source.snapshot(&context).expect("snapshot");
    let (dest, fresh) = Store::create(&dir.path().join("dest"), &schema, tiny_map()).expect("dest");
    let initial = dest.current_map_bytes();
    dest.compact_snapshot(&pinned, fresh, &context)
        .expect("grow and compact");
    assert!(dest.current_map_bytes() > initial);
    assert_eq!(
        entries(&dest.snapshot(&context).expect("dest snapshot"), false),
        entries(&pinned, false)
    );
}

#[test]
fn physical_compaction_cancellation_rolls_back_data_and_fresh_metadata() {
    use crate::work::WorkError;
    let dir = TempDir::new("physical-compact-abort");
    let schema = schema();
    let source = create_default(&dir.path().join("source"));
    commit_changes(
        &source,
        &change_set(&schema, &[(NOTE, note(1, "kept"))], &[]),
    );
    // The only overflow value is metadata. Its first chunk is copied after
    // the data tree was populated and the fresh metadata tree was cleared.
    attach_receipt(&source, &schema, &work(), &[0xAB; 8192]);
    let pinned = source.snapshot(&work()).unwrap();
    for before_start in [true, false] {
        let (dest, fresh) = Store::create(
            &dir.path().join(if before_start {
                "before"
            } else {
                "mid-metadata"
            }),
            &schema,
            MapPolicy::default(),
        )
        .unwrap();
        let initial_metadata = entries(&dest.snapshot(&work()).unwrap(), true);
        let context = WorkContext::new();
        let interrupted = super::super::copy::cancellation_test::after_overflow_chunks(1);
        if before_start {
            context.cancel();
        }
        assert_eq!(
            dest.compact_snapshot(&pinned, fresh, &context),
            Err(StoreError::Work(WorkError::Cancelled))
        );
        assert_eq!(
            super::super::copy::cancellation_test::Guard::copied(),
            usize::from(!before_start)
        );
        drop(interrupted);
        let copied = dest.snapshot(&work()).unwrap();
        assert!(entries(&copied, false).is_empty());
        assert_eq!(entries(&copied, true), initial_metadata);
    }
}

#[test]
fn physical_compaction_reindexes_test_only_fingerprint_policy_changes() {
    let dir = TempDir::new("physical-compact-fingerprint");
    let schema = schema();
    let source = Store::create_forced_fingerprint(
        &dir.path().join("source"),
        &schema,
        MapPolicy::default(),
        [0xCC; 16],
    )
    .expect("forced source");
    let values = note(7, "collision-safe");
    commit_changes(
        &source,
        &change_set(&schema, &[(NOTE, values.clone())], &[]),
    );
    let context = work();
    let pinned = source.snapshot(&context).expect("snapshot");
    let (dest, fresh) =
        Store::create(&dir.path().join("dest"), &schema, MapPolicy::default()).expect("dest");
    dest.compact_snapshot(&pinned, fresh, &context)
        .expect("logical fallback");
    let row =
        crate::canonical::CanonicalRow::encode(schema.relation(NOTE).fields(), &values, &context)
            .expect("canonical row");
    assert!(
        dest.snapshot(&context)
            .expect("dest snapshot")
            .contains(NOTE, row.as_bytes(), &context)
            .expect("membership reindexed")
    );
}
