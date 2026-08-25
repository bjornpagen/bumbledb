//! The retention law, the checkpoint backlink walk, and point-in-time
//! restore: gc exemption exactness, PITR to recorded vectors, and the
//! by-time mapping.

mod lane_d_support;

use std::collections::BTreeMap;

use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, StatementDescriptor,
    ValueType,
};
use bumbledb::{SchemaDescriptor, Value};
use bumbledb_log::braids::braids;
use bumbledb_log::gc::{
    Gc, PublishClock, Restore, RestoreRefusal, gc, gc_at, restore_by_time, restore_to_vector,
};
use bumbledb_log::manifest::{ckpt_doc_key, ckpt_mdb_key, log_key};
use bumbledb_log::replica::{Opened, Replica, Vector};
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use lane_d_support::{
    NOTE, RECIPE, TestLog, insert_note, insert_recipe, kitchen_braid, note_braid, temp_dir, theory,
};

fn open_replica(
    root: &std::path::Path,
    dir: &std::path::Path,
) -> Replica<SchemaDescriptor, FsStore> {
    match Replica::open(FsStore::new(root.to_path_buf()), "", dir, theory()).expect("open") {
        Opened::Ready(replica) => *replica,
        Opened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

fn restored(
    outcome: Restore<SchemaDescriptor>,
) -> (
    bumbledb::Db<SchemaDescriptor>,
    bumbledb_log::replica::Vector,
) {
    match outcome {
        Restore::Restored { db, vector } => (*db, vector),
        Restore::Refused(refusal) => panic!("restore refused: {refusal:?}"),
    }
}

fn sweep_at(
    log: &lane_d_support::TestLog,
    window_ms: u64,
    now_ms: u64,
    publish_ms: u64,
) -> bumbledb_log::gc::Sweep {
    match gc_at(
        &log.store,
        "",
        &log.codec,
        window_ms,
        PublishClock { now_ms, publish_ms },
    )
    .expect("gc")
    {
        Gc::Swept(sweep) => sweep,
        other => panic!("expected a sweep, got {other:?}"),
    }
}

#[test]
fn nothing_is_eligible_while_the_manifest_says_null() {
    let root = temp_dir("gc_null");
    let mut log = TestLog::new(root, "");
    let kitchen = kitchen_braid(&log.codec);
    log.publish(kitchen, &[insert_recipe(1)], 100);
    assert_eq!(
        gc(&log.store, "", &log.codec, 0, u64::MAX).expect("gc"),
        Gc::NothingEligible
    );
    assert!(
        log.store
            .get(&log_key("", kitchen, 1))
            .expect("get")
            .is_some()
    );
}

#[test]
fn the_exemption_law_is_exact() {
    let root = temp_dir("gc_exempt");
    let local = temp_dir("gc_exempt_local");
    let scratch = temp_dir("gc_exempt_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);
    let notes = note_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 1_000);
    log.publish(kitchen, &[insert_recipe(2)], 2_000);
    log.publish(kitchen, &[insert_recipe(3)], 3_000);
    log.publish(kitchen, &[insert_recipe(4)], 4_000);
    log.publish(notes, &[insert_note(1, "keep me")], 1_500);
    let builder = open_replica(&root, &local.join("builder"));
    log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);

    // Window 1500 at now 4000 against publish 2000: the below-floor
    // kitchen prefix is old as one unit; kitchen 4 and notes 1 are the
    // floor and survive. The walk names slots in ascending order.
    let swept = sweep_at(&log, 1_500, 4_000, 2_000);
    assert_eq!(
        swept.log_deleted,
        vec![
            log_key("", kitchen, 1).to_string(),
            log_key("", kitchen, 2).to_string(),
            log_key("", kitchen, 3).to_string(),
        ],
        "the walk deletes upward through the old prefix"
    );
    assert_eq!(swept.checkpoints_deleted, Vec::<[u8; 32]>::new());
    assert_eq!(swept.swept_below.get(&kitchen).copied(), Some(4));
    assert!(
        log.store
            .get(&log_key("", kitchen, 1))
            .expect("get")
            .is_none()
    );
    assert!(
        log.store
            .get(&log_key("", kitchen, 2))
            .expect("get")
            .is_none()
    );
    assert!(
        log.store
            .get(&log_key("", kitchen, 3))
            .expect("get")
            .is_none()
    );
    assert!(
        log.store
            .get(&log_key("", kitchen, 4))
            .expect("get")
            .is_some()
    );
    assert!(
        log.store
            .get(&log_key("", notes, 1))
            .expect("get")
            .is_some()
    );

    // A second sweep under the same clock deletes nothing more: the
    // prefix is already `[0, marker)`.
    let swept = sweep_at(&log, 1_500, 4_000, 2_000);
    assert_eq!(swept.log_deleted, Vec::<String>::new());
    assert_eq!(swept.checkpoints_deleted, Vec::<[u8; 32]>::new());
}

#[test]
fn old_checkpoints_die_behind_the_current_one() {
    let root = temp_dir("gc_ckpt");
    let local = temp_dir("gc_ckpt_local");
    let scratch = temp_dir("gc_ckpt_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 1_000);
    let builder = open_replica(&root, &local.join("b1"));
    let old_digest = log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);

    log.publish(kitchen, &[insert_recipe(2)], 900_000);
    let builder = open_replica(&root, &local.join("b2"));
    let current_digest = log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);

    let swept = sweep_at(&log, 10_000, 1_000_000, 1_000);
    assert_eq!(swept.checkpoints_deleted, vec![old_digest]);
    assert!(
        log.store
            .get(&ckpt_doc_key("", &old_digest))
            .expect("get")
            .is_none(),
        "json and mdb die as one unit"
    );
    assert!(
        log.store
            .get(&ckpt_mdb_key("", &old_digest))
            .expect("get")
            .is_none(),
        "json and mdb die as one unit"
    );
    assert!(
        log.store
            .get(&ckpt_doc_key("", &current_digest))
            .expect("get")
            .is_some(),
        "the current checkpoint is always exempt"
    );
    assert!(
        log.store
            .get(&ckpt_mdb_key("", &current_digest))
            .expect("get")
            .is_some(),
        "the current checkpoint is always exempt"
    );

    // Restoring behind the deleted checkpoint is beyond retention now.
    match restore_to_vector(
        &log.store,
        "",
        &local.join("beyond"),
        &theory(),
        &Vector::from(BTreeMap::from([(kitchen, 1)])),
    )
    .expect("restore")
    {
        Restore::Refused(RestoreRefusal::BeyondRetention { digest }) => {
            assert_eq!(digest, old_digest);
        }
        Restore::Refused(other) => panic!("wrong refusal: {other:?}"),
        Restore::Restored { .. } => panic!("a gc'd base must refuse"),
    }
}

#[test]
fn restore_lands_exactly_on_the_recorded_vector() {
    let root = temp_dir("gc_pitr");
    let local = temp_dir("gc_pitr_local");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);
    let notes = note_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 100);
    log.publish(kitchen, &[insert_recipe(2)], 200);
    log.publish(kitchen, &[insert_recipe(3)], 300);
    log.publish(notes, &[insert_note(1, "first")], 150);
    log.publish(notes, &[insert_note(2, "second")], 250);

    let target = Vector::from(BTreeMap::from([(kitchen, 2), (notes, 1)]));
    let (db, vector) = restored(
        restore_to_vector(&log.store, "", &local.join("r"), &theory(), &target).expect("restore"),
    );
    assert_eq!(vector, target, "the restored vector is the reported truth");
    assert_eq!(db.generation().expect("generation").value(), 3);
    db.read(|instance| {
        assert!(instance.contains_dyn(RECIPE, &[Value::U64(2)])?);
        assert!(!instance.contains_dyn(RECIPE, &[Value::U64(3)])?);
        assert!(instance.contains_dyn(NOTE, &[Value::U64(1), Value::String("first".into())])?);
        assert!(!instance.contains_dyn(NOTE, &[Value::U64(2), Value::String("second".into())])?);
        Ok(())
    })
    .expect("read");

    // The restored directory is a valid replica directory: opening it
    // as a replica resumes from the restored vector.
    drop(db);
    let replica = open_replica(&root, &local.join("r"));
    assert!(replica.vector().at(kitchen) >= 2);
}

#[test]
fn restore_seeds_from_a_checkpoint_when_one_qualifies() {
    let root = temp_dir("gc_pitr_seed");
    let local = temp_dir("gc_pitr_seed_local");
    let scratch = temp_dir("gc_pitr_seed_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 100);
    log.publish(kitchen, &[insert_recipe(2)], 200);
    let builder = open_replica(&root, &local.join("builder"));
    log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);
    log.publish(kitchen, &[insert_recipe(3)], 300);

    // The tail below the checkpoint dies; a restore at or above the
    // checkpoint vector still works because the base carries it.
    log.store.delete(&log_key("", kitchen, 1)).expect("delete");
    log.store.delete(&log_key("", kitchen, 2)).expect("delete");

    let target = Vector::from(BTreeMap::from([(kitchen, 3), (note_braid(&log.codec), 0)]));
    let (db, vector) = restored(
        restore_to_vector(&log.store, "", &local.join("r"), &theory(), &target).expect("restore"),
    );
    assert_eq!(vector, target);
    assert_eq!(db.generation().expect("generation").value(), 3);

    // Below the checkpoint the slots are gone: a restore there names
    // the missing slot.
    match restore_to_vector(
        &log.store,
        "",
        &local.join("r2"),
        &theory(),
        &Vector::from(BTreeMap::from([(kitchen, 1)])),
    )
    .expect("restore")
    {
        Restore::Refused(RestoreRefusal::SlotMissing { slot, .. }) => assert_eq!(slot, 1),
        Restore::Refused(other) => panic!("wrong refusal: {other:?}"),
        Restore::Restored { .. } => panic!("gc'd slots cannot replay"),
    }
}

#[test]
fn by_time_restore_maps_the_instant_per_braid() {
    let root = temp_dir("gc_bytime");
    let local = temp_dir("gc_bytime_local");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);
    let notes = note_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 100);
    log.publish(kitchen, &[insert_recipe(2)], 200);
    log.publish(kitchen, &[insert_recipe(3)], 300);
    log.publish(notes, &[insert_note(1, "early")], 150);
    log.publish(notes, &[insert_note(2, "late")], 250);

    let (_, vector) = restored(
        restore_by_time(&log.store, "", &local.join("t250"), &theory(), 250).expect("restore"),
    );
    assert_eq!(
        vector,
        Vector::from(BTreeMap::from([(kitchen, 2), (notes, 2)]))
    );

    let (_, vector) = restored(
        restore_by_time(&log.store, "", &local.join("t120"), &theory(), 120).expect("restore"),
    );
    assert_eq!(
        vector,
        Vector::from(BTreeMap::from([(kitchen, 1), (notes, 0)]))
    );

    let (db, vector) = restored(
        restore_by_time(&log.store, "", &local.join("t0"), &theory(), 0).expect("restore"),
    );
    assert_eq!(
        vector,
        Vector::from(BTreeMap::from([(kitchen, 0), (notes, 0)]))
    );
    assert_eq!(db.generation().expect("generation").value(), 0);
}

fn braid_nine() -> bumbledb_log::braids::BraidId {
    let field = FieldDescriptor {
        name: "id".into(),
        value_type: ValueType::U64,
        generation: Generation::None,
    };
    let relations = (0..10)
        .map(|index| RelationDescriptor {
            name: format!("r{index}").into(),
            fields: vec![field.clone()],
            extension: (index < 9).then_some(Box::from([])),
        })
        .collect();
    let descriptor = SchemaDescriptor {
        relations,
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(9),
            projection: Box::from([FieldId(0)]),
        }],
    };
    braids(&descriptor).parse(9).expect("ordinary braid")
}

#[test]
fn restore_refuses_a_braid_the_schema_never_minted() {
    let root = temp_dir("gc_unknown_braid");
    let local = temp_dir("gc_unknown_braid_local");
    let log = TestLog::new(root.clone(), "");
    match restore_to_vector(
        &log.store,
        "",
        &local.join("r"),
        &theory(),
        &Vector::from(BTreeMap::from([(braid_nine(), 0)])),
    )
    .expect("restore")
    {
        Restore::Refused(RestoreRefusal::UnknownBraid { got }) => assert_eq!(got, 9),
        Restore::Refused(other) => panic!("wrong refusal: {other:?}"),
        Restore::Restored { .. } => panic!("unknown braid must refuse"),
    }
}

#[test]
fn an_interrupted_sweep_resumes_past_a_hole() {
    let root = temp_dir("gc_hole");
    let local = temp_dir("gc_hole_local");
    let scratch = temp_dir("gc_hole_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 1_000);
    log.publish(kitchen, &[insert_recipe(2)], 2_000);
    log.publish(kitchen, &[insert_recipe(3)], 3_000);
    log.publish(kitchen, &[insert_recipe(4)], 4_000);
    let builder = open_replica(&root, &local.join("builder"));
    log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);

    log.store
        .delete(&log_key("", kitchen, 2))
        .expect("plant a hole");

    let swept = sweep_at(&log, 1_500, 4_000, 2_000);
    assert_eq!(
        swept.log_deleted,
        vec![
            log_key("", kitchen, 1).to_string(),
            log_key("", kitchen, 3).to_string(),
        ],
        "a hole advances the marker; the walk continues upward"
    );
    assert_eq!(swept.swept_below.get(&kitchen).copied(), Some(4));
    assert!(
        log.store
            .get(&log_key("", kitchen, 1))
            .expect("get")
            .is_none()
    );
    assert!(
        log.store
            .get(&log_key("", kitchen, 3))
            .expect("get")
            .is_none()
    );
    assert!(
        log.store
            .get(&log_key("", kitchen, 4))
            .expect("get")
            .is_some(),
        "the floor stays"
    );
}

#[test]
fn a_writer_claimed_timestamp_does_not_age_the_slot() {
    let root = temp_dir("gc_clock");
    let local = temp_dir("gc_clock_local");
    let scratch = temp_dir("gc_clock_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);

    // The writer stamps the batch at the observation instant; the
    // checkpointer stamped the covering checkpoint far earlier.
    log.publish(kitchen, &[insert_recipe(1)], 1_000_000);
    log.publish(kitchen, &[insert_recipe(2)], 1_000_000);
    let builder = open_replica(&root, &local.join("builder"));
    log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);

    let swept = sweep_at(&log, 10_000, 1_000_000, 1_000);
    assert_eq!(
        swept.log_deleted,
        vec![log_key("", kitchen, 1).to_string()],
        "age is the publish clock, not the batch header"
    );
    assert!(
        log.store
            .get(&log_key("", kitchen, 1))
            .expect("get")
            .is_none()
    );
    assert!(
        log.store
            .get(&log_key("", kitchen, 2))
            .expect("get")
            .is_some(),
        "the floor stays"
    );
}

#[test]
fn a_missing_checkpoint_document_still_drops_its_mdb() {
    let root = temp_dir("gc_unit");
    let local = temp_dir("gc_unit_local");
    let scratch = temp_dir("gc_unit_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 1_000);
    let builder = open_replica(&root, &local.join("b1"));
    let old_digest = log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);

    log.publish(kitchen, &[insert_recipe(2)], 2_000);
    let builder = open_replica(&root, &local.join("b2"));
    log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);

    log.store
        .delete(&ckpt_doc_key("", &old_digest))
        .expect("drop document");
    assert!(
        log.store
            .get(&ckpt_mdb_key("", &old_digest))
            .expect("get")
            .is_some(),
        "mdb remains after the document disappears"
    );

    let swept = sweep_at(&log, 10_000, 1_000_000, 1_000);
    assert!(
        log.store
            .get(&ckpt_mdb_key("", &old_digest))
            .expect("get")
            .is_none(),
        "the unit delete drops the orphan mdb"
    );
    assert_eq!(swept.checkpoints_deleted, Vec::<[u8; 32]>::new());
}
