//! Fresh-destination adoption (CORE-015): metadata-only history refuses.

use super::*;
use crate::schema::{
    FieldDescriptor, RelationDescriptor, Schema, SchemaDescriptor, StatementDescriptor,
    ValidateDescriptor as _,
};
use crate::storage::store::judge_bridge::{SchemaJudge, UnindexedRows};
use crate::storage::store::{HostRecordChange, HostChanges, AttachmentChange};
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
            Prepared::Rejected(v) => panic!("{v:?}"),
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
}

fn commit_row(store: &Store, schema: &Schema, work: &WorkContext, id: u64) {
    let changes = ChangeSet::builder(schema, work.clone())
        .insert(RelationId(0), &[Value::U64(id)])
        .expect("stage")
        .finish()
        .expect("seal");
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
        Prepared::Rejected(v) => panic!("{v:?}"),
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
