//! D04 / D06 / D26: complete admission, no-clobber install, unpublished cleanup.

use super::*;
use crate::schema::tests::{closed, containment, fd, field, row, side};
use crate::schema::{
    ContainmentId, FieldDescriptor, RelationDescriptor, Schema, SchemaDescriptor,
    StatementDescriptor, ValidateDescriptor as _,
};
use crate::storage::store::staging::{InstallOutcome, UnreadyStore};
use crate::storage::store::{
    AttachmentChange, HostChanges, HostRecordChange, HostWindow, StoreError, UnindexedRows,
};
use bumbledb_theory::schema::{FieldId, RelationId};

fn keyed_users() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "User".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "email".into(),
                    value_type: ValueType::String,
                },
            ],
            extension: None,
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::from([FieldId(1)]),
        }],
    }
    .validate()
    .expect("keyed users")
}

fn nonempty_required() -> Schema {
    SchemaDescriptor {
        relations: vec![
            closed(
                "Required",
                vec![field("parent", ValueType::U64)],
                vec![row("need", vec![Value::U64(1)])],
            ),
            RelationDescriptor {
                extension: None,
                name: "Parent".into(),
                fields: vec![field("id", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0)]),
            containment(
                side(RelationId(0), &[FieldId(0)]),
                side(RelationId(1), &[FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("nonempty-required")
}

fn user(id: u64, email: &str) -> Vec<Value> {
    vec![Value::U64(id), Value::String(email.into())]
}

/// D26: two key-conflicting tuples then admit with no delta must reject.
#[test]
fn d26_conflicting_populated_stage_rejects_with_empty_delta() {
    let (_dir, path) = store_dir("d26-conflict-admit");
    let schema = keyed_users();
    let work = work();
    let unready = UnreadyStore::begin(&path, &schema, MapPolicy::default(), &work).expect("begin");
    let first = change_set(&schema, &[(RelationId(0), user(1, "dup@ex"))], &[]);
    let second = change_set(&schema, &[(RelationId(0), user(2, "dup@ex"))], &[]);
    unready
        .populate(&work, |stage, work| {
            stage.apply(&first, work)?;
            stage.apply(&second, work)?;
            Ok(())
        })
        .expect("populate");
    match unready.admit(&schema, &work) {
        Err(StoreError::JudgeRefused { .. }) => {}
        other => panic!("invalid populated stage must reject complete admit, got {other:?}"),
    }
    assert!(!path.exists(), "destination stays absent after abandoned admit");
}

/// D26 positive dual: empty nonempty-required staging rejects; valid rows admit.
#[test]
fn d26_nonempty_required_survives_populate_admit_install_reopen() {
    let (_dir, path) = store_dir("d26-nonempty-required");
    let schema = nonempty_required();
    let work = work();
    let unready = UnreadyStore::begin(&path, &schema, MapPolicy::default(), &work).expect("begin");
    unready
        .admit(&schema, &work)
        .expect_err("empty nonempty-required staging rejects");
    assert!(!path.exists());

    let unready = UnreadyStore::begin(&path, &schema, MapPolicy::default(), &work).expect("begin");
    let fill = change_set(&schema, &[(RelationId(1), vec![Value::U64(1)])], &[]);
    unready
        .populate(&work, |stage, work| {
            stage.apply(&fill, work)?;
            Ok(())
        })
        .expect("populate");
    let admitted = unready.admit(&schema, &work).expect("filled admits");
    match admitted.install(&schema, MapPolicy::default(), &work) {
        InstallOutcome::Installed(store) => {
            assert_eq!(
                store
                    .snapshot(&work)
                    .expect("snap")
                    .row_count(RelationId(1))
                    .expect("count"),
                1
            );
            drop(store);
        }
        other => panic!("expected Installed, got {other:?}"),
    }
    let reopened = Store::open(&path, &schema, MapPolicy::default()).expect("reopen");
    assert_eq!(
        reopened
            .snapshot(&work)
            .expect("snap")
            .row_count(RelationId(1))
            .expect("count"),
        1
    );
}

/// D06: two installers never overwrite; loser keeps unpublished cleanup.
#[test]
fn d06_second_installer_cannot_clobber_and_cleanup_spares_winner() {
    let (_dir, path) = store_dir("d06-two-installers");
    let schema = schema();
    let work = work();
    let first = UnreadyStore::begin(&path, &schema, MapPolicy::default(), &work).expect("first");
    let second = UnreadyStore::begin(&path, &schema, MapPolicy::default(), &work).expect("second");
    let rows = change_set(&schema, &[(NOTE, note(1, "winner"))], &[]);
    first
        .populate(&work, |stage, work| stage.apply(&rows, work).map(|_| ()))
        .expect("populate first");
    let admitted = first.admit(&schema, &work).expect("admit first");
    match admitted.install(&schema, MapPolicy::default(), &work) {
        InstallOutcome::Installed(_) => {}
        other => panic!("first installer publishes, got {other:?}"),
    }
    match second.admit(&schema, &work).expect("second still unready").install(
        &schema,
        MapPolicy::default(),
        &work,
    ) {
        InstallOutcome::NotInstalled { cleanup, detail } => {
            assert!(matches!(detail, StoreError::DestinationExists { .. }));
            assert!(path.exists(), "winner remains");
            cleanup.abandon();
            assert!(path.exists(), "cleanup must not delete the winning successor");
        }
        other => panic!("second installer must not overwrite, got {other:?}"),
    }
}

/// D06: ready destination cannot be populated; begin refuses it.
#[test]
fn d06_ready_destination_population_is_refused() {
    let (_dir, path) = store_dir("d06-ready-dest");
    let schema = schema();
    let work = work();
    drop(Store::create(&path, &schema, MapPolicy::default()).expect("create").0);
    match UnreadyStore::begin(&path, &schema, MapPolicy::default(), &work) {
        Err(StoreError::DestinationExists { .. }) => {}
        other => panic!("ready dest must refuse begin, got {other:?}"),
    }
}

/// D06: metadata-only destination is not fresh (zero rows insufficient).
#[test]
fn d06_zero_row_host_metadata_is_not_fresh() {
    let (_dir, dest_path) = store_dir("d06-meta-dest");
    let (_dir2, src_path) = store_dir("d06-meta-src");
    let schema = keyed_users();
    let work = work();
    let (dest, _fresh) =
        Store::create(&dest_path, &schema, MapPolicy::default()).expect("dest");
    {
        let mut owner = dest.writer(&work).expect("writer");
        let empty = ChangeSet::builder(&schema, work.clone())
            .finish()
            .expect("empty");
        let prepared = match owner
            .prepare_incremental(
                crate::schema::judge::LawfulParent::established(),
                &empty,
                &UnindexedRows,
                &crate::storage::store::SchemaJudge::new(&schema),
            )
            .expect("prepare")
        {
            Prepared::Admitted(prepared) => prepared,
            Prepared::Rejected(v) => panic!("{v:?}"),
        };
        prepared
            .seal(HostChanges {
                records: &host_put(b"receipt", b"noop"),
                attachment: AttachmentChange::Keep,
            })
            .expect("seal")
            .commit()
            .expect("commit");
    }
    let source = Store::create(&src_path, &schema, MapPolicy::default())
        .expect("source")
        .0;
    let snapshot = source.snapshot(&work).expect("snap");
    let err = dest
        .adopt_vacant_snapshot(&snapshot, &UnindexedRows, &work)
        .expect_err("metadata-only dest refuses");
    assert!(matches!(err, StoreError::DestinationExists { .. }));
}

/// Ordinary admitted writes call `prepare_incremental` with a real
/// `LawfulParent`. A row commits; an empty follow-up is a no-op under
/// that parent, not complete admit. Verification NotRun.
#[test]
fn ordinary_admitted_write_uses_prepare_incremental_under_lawful_parent() {
    let parent = crate::schema::judge::LawfulParent::established();
    let (_dir, path) = store_dir("ordinary-incremental-parent");
    let schema = keyed_users();
    let work = work();
    let store = Store::create(&path, &schema, MapPolicy::default())
        .expect("create")
        .0;
    let added = change_set(&schema, &[(RelationId(0), user(1, "a@ex"))], &[]);
    {
        let mut owner = store.writer(&work).expect("writer");
        match owner
            .prepare_incremental(
                parent,
                &added,
                &UnindexedRows,
                &crate::storage::store::SchemaJudge::new(&schema),
            )
            .expect("prepare")
        {
            Prepared::Admitted(prepared) => {
                prepared
                    .seal(HostChanges {
                        records: &[],
                        attachment: AttachmentChange::Keep,
                    })
                    .expect("seal")
                    .commit()
                    .expect("commit");
            }
            Prepared::Rejected(violations) => panic!("{violations:?}"),
        }
    }
    let snap = store.snapshot(&work).expect("snap");
    assert_eq!(snap.row_count(RelationId(0)).expect("count"), 1);
    let empty = ChangeSet::builder(&schema, work.clone())
        .finish()
        .expect("empty");
    let mut owner = store.writer(&work).expect("writer");
    match owner
        .prepare_incremental(
            parent,
            &empty,
            &UnindexedRows,
            &crate::storage::store::SchemaJudge::new(&schema),
        )
        .expect("empty incremental")
    {
        Prepared::Admitted(prepared) => {
            let commit = prepared
                .seal(HostChanges {
                    records: &[],
                    attachment: AttachmentChange::Keep,
                })
                .expect("seal")
                .commit()
                .expect("commit");
            assert!(!commit.changed, "empty delta under a lawful parent is a no-op");
        }
        Prepared::Rejected(violations) => panic!("{violations:?}"),
    }
}

/// Projection-id exhaustion is `StoreError::Compile`, never corruption.
/// Verification NotRun.
#[test]
fn compile_exhaustion_is_store_error_compile_not_corruption() {
    let err = StoreError::from(crate::schema::CompileError::ProjectionIdExhausted);
    assert!(matches!(
        err,
        StoreError::Compile(crate::schema::CompileError::ProjectionIdExhausted)
    ));
    assert!(!matches!(err, StoreError::Corruption(_)));
}

/// Host metadata written on unready is visible only through inspect, not
/// at dest. Complete-admit failure leaves dest unpublished. Verification
/// NotRun.
#[test]
fn unready_host_metadata_is_invisible_until_admit() {
    let (_dir, path) = store_dir("unready-host-invisible");
    let schema = keyed_users();
    let work = work();
    let unready = UnreadyStore::begin(&path, &schema, MapPolicy::default(), &work).expect("begin");
    let first = change_set(&schema, &[(RelationId(0), user(1, "dup@ex"))], &[]);
    let second = change_set(&schema, &[(RelationId(0), user(2, "dup@ex"))], &[]);
    unready
        .populate(&work, |stage, work| {
            stage.apply(&first, work)?;
            stage.apply(&second, work)?;
            stage
                .put_host(
                    HostChanges {
                        records: &host_put(b"binding", b"genesis"),
                        attachment: AttachmentChange::Put(b"control"),
                    },
                    work,
                )?;
            Ok(())
        })
        .expect("populate");
    unready
        .inspect(&work, |reader, work| {
            assert_eq!(
                reader.host_record(b"binding").expect("host"),
                Some(&b"genesis"[..])
            );
            assert_eq!(reader.attachment().expect("ctl"), Some(&b"control"[..]));
            let mut saw = false;
            reader.host_scan(b"bind", work, &mut |key, value| {
                saw = key == b"binding" && value == b"genesis";
                Ok(())
            })?;
            assert!(saw, "unready host_scan must see the staged binding");
            Ok(())
        })
        .expect("inspect");
    assert!(
        !path.exists(),
        "host metadata written on unready must not publish dest"
    );
    match unready.admit(&schema, &work) {
        Err(StoreError::JudgeRefused { .. }) => {}
        other => panic!("conflict must refuse complete admit, got {other:?}"),
    }
    assert!(
        !path.exists(),
        "admit failure leaves destination unpublished"
    );
}

/// Batched host deletes on unready are not installed; admit failure never
/// yields admitted ownership and dest stays unpublished. Verification
/// NotRun.
#[test]
fn unready_batched_host_deletes_stay_invisible_until_admit() {
    let (_dir, path) = store_dir("unready-host-batch-invisible");
    let schema = keyed_users();
    let work = work();
    let unready = UnreadyStore::begin(&path, &schema, MapPolicy::default(), &work).expect("begin");
    let first = change_set(&schema, &[(RelationId(0), user(1, "dup@ex"))], &[]);
    let second = change_set(&schema, &[(RelationId(0), user(2, "dup@ex"))], &[]);
    let receipts = [
        HostRecordChange::Put {
            key: b"ra",
            value: b"1",
        },
        HostRecordChange::Put {
            key: b"rb",
            value: b"2",
        },
        HostRecordChange::Put {
            key: b"rc",
            value: b"3",
        },
    ];
    unready
        .populate(&work, |stage, work| {
            stage.apply(&first, work)?;
            stage.apply(&second, work)?;
            stage.put_host(
                HostChanges {
                    records: &receipts,
                    attachment: AttachmentChange::Keep,
                },
                work,
            )?;
            Ok(())
        })
        .expect("populate");
    // ra+1 is 3 bytes; the next record would exceed the cap after one visit.
    let window = unready
        .delete_host_batch(b"r", None, &work, 3)
        .expect("first window");
    assert!(
        matches!(
            window,
            HostWindow::More {
                records: 1,
                bytes: 3,
                ..
            }
        ),
        "one charged window, not every receipt key: {window:?}"
    );
    unready
        .inspect(&work, |reader, _work| {
            assert_eq!(reader.host_record(b"ra").expect("ra"), None);
            assert_eq!(reader.host_record(b"rb").expect("rb"), Some(&b"2"[..]));
            assert_eq!(reader.host_record(b"rc").expect("rc"), Some(&b"3"[..]));
            Ok(())
        })
        .expect("inspect");
    assert!(
        !path.exists(),
        "batched deletes must not install dest (unpublished, not a readiness name)"
    );
    match unready.admit(&schema, &work) {
        Err(StoreError::JudgeRefused { .. }) => {}
        Ok(_) => panic!("admit must not yield AdmittedStore on a conflicting stage"),
        other => panic!("conflict must refuse complete admit, got {other:?}"),
    }
    assert!(
        !path.exists(),
        "admit failure leaves destination unpublished"
    );
}

/// Closed Source[a,b] ⊆ ordinary Target[b,a]. Source projected values are
/// (a=1,b=2). Target's functionality key is ordered (a,b). The lawful
/// cover is the target row (a=2,b=1): Target[b,a]=(1,2) matches the
/// source projection. A same-order decoy Target(a=1,b=2) matches source
/// field order, not the containment. Deleting the lawful cover must
/// refuse on the production unready→admit path (UnindexedRows: no
/// closed-source index). Verification NotRun.
fn closed_source_permuted_target() -> Schema {
    SchemaDescriptor {
        relations: vec![
            closed(
                "Source",
                vec![field("a", ValueType::U64), field("b", ValueType::U64)],
                vec![row("pair", vec![Value::U64(1), Value::U64(2)])],
            ),
            RelationDescriptor {
                extension: None,
                name: "Target".into(),
                fields: vec![field("a", ValueType::U64), field("b", ValueType::U64)],
            },
        ],
        statements: vec![
            fd(RelationId(1), &[FieldId(0), FieldId(1)]),
            containment(
                side(RelationId(0), &[FieldId(1), FieldId(2)]),
                side(RelationId(1), &[FieldId(1), FieldId(0)]),
            ),
        ],
    }
    .validate()
    .expect("closed Source[a,b] ⊆ Target[b,a]")
}

/// D04 production-store: grouping order is logical. Physical index
/// availability must not change whether the lawful target (a=2,b=1) is
/// required. Authored now; verification NotRun.
#[test]
fn d04_closed_source_permuted_target_deletion_refuses_on_production_store() {
    let schema = closed_source_permuted_target();
    let work = work();
    let source = RelationId(0);
    let target = RelationId(1);
    let lawful = vec![Value::U64(2), Value::U64(1)];
    let decoy = vec![Value::U64(1), Value::U64(2)];
    let law = schema.containment(ContainmentId(0));

    let (_ok_dir, ok_path) = store_dir("d04-closed-permuted-lawful");
    let covered =
        UnreadyStore::begin(&ok_path, &schema, MapPolicy::default(), &work).expect("begin");
    let insert_lawful = change_set(&schema, &[(target, lawful.clone())], &[]);
    covered
        .populate(&work, |stage, work| {
            stage.apply(&insert_lawful, work)?;
            Ok(())
        })
        .expect("populate lawful");
    let admitted = covered.admit(&schema, &work).expect("lawful cover admits");
    drop(admitted);
    assert!(!ok_path.exists(), "control never installed dest");

    let (_dir, path) = store_dir("d04-closed-permuted-delete");
    let unready = UnreadyStore::begin(&path, &schema, MapPolicy::default(), &work).expect("begin");
    let insert_both = change_set(
        &schema,
        &[(target, lawful.clone()), (target, decoy.clone())],
        &[],
    );
    let delete_lawful = change_set(&schema, &[], &[(target, lawful)]);
    unready
        .populate(&work, |stage, work| {
            stage.apply(&insert_both, work)?;
            stage.apply(&delete_lawful, work)?;
            Ok(())
        })
        .expect("populate then delete lawful cover");
    unready
        .inspect(&work, |reader, _work| {
            assert_eq!(
                reader.snapshot().row_count(source).expect("source count"),
                0,
                "closed source stays schema-sealed, never stored"
            );
            assert_eq!(
                reader.snapshot().row_count(target).expect("target count"),
                1,
                "decoy remains after the lawful cover is deleted"
            );
            let theory = reader.snapshot().compiled();
            assert!(
                theory.source_projection(law.id).is_none(),
                "closed source is not given a grouping index"
            );
            let binding = theory
                .target_binding(law.id)
                .expect("ordinary target uses L01 intern coordinates");
            assert_eq!(
                binding.physical_values(&[Value::U64(1), Value::U64(2)]).as_deref(),
                Some(&[Value::U64(2), Value::U64(1)][..]),
                "Target[b,a]=(1,2) interns as key-order (a=2,b=1)"
            );
            assert_ne!(
                binding.physical_values(&[Value::U64(2), Value::U64(1)]),
                binding.physical_values(&[Value::U64(1), Value::U64(2)]),
                "same-order decoy Target(a=1,b=2) is not the lawful cover"
            );
            Ok(())
        })
        .expect("inspect");
    assert!(!path.exists(), "dest is unpublished, not a readiness name");
    match unready.admit(&schema, &work) {
        Err(StoreError::JudgeRefused { statement, .. }) => {
            assert_eq!(statement, law.id, "containment names the stranded source");
        }
        Ok(_) => panic!("deleting lawful target (a=2,b=1) must not yield AdmittedStore"),
        other => panic!("expected JudgeRefused, got {other:?}"),
    }
    assert!(!path.exists(), "refused admit leaves dest unpublished");
}
