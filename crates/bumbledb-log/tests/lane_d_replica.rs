//! The replica lifecycle over `FsStore`: bootstrap, open, reopen,
//! catch-up, the crash-window absorption, the wholeness phantom
//! discard, tip-vs-hole in both directions, the gc-safety heartbeat
//! bound, wedged partial service, rejected replay in both phases,
//! pending resolution, `wait_for`, and per-braid refresh.

mod lane_d_support;

use std::collections::BTreeMap;
use std::path::Path;

use bumbledb::{Db, SchemaDescriptor, Value};
use bumbledb_log::apply::{ApplyRefusal, ChainCause, apply};
use bumbledb_log::codec::BatchHeader;
use bumbledb_log::manifest::log_key;
use bumbledb_log::replica::{Corruption, Opened, Provenance, Refreshed, Replica, Vector, Waited};
use bumbledb_log::sidecar::{Chain, ChainEntry, Pending, SidecarRead};
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use lane_d_support::{
    NOTE, RECIPE, TestLog, insert_note, insert_recipe, insert_step, kitchen_braid, note_braid,
    temp_dir, theory,
};

type TestReplica = Replica<SchemaDescriptor, FsStore>;

fn ready(opened: Opened<SchemaDescriptor, FsStore>) -> TestReplica {
    match opened {
        Opened::Ready(replica) => *replica,
        Opened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

fn open_at(root: &Path, dir: &Path) -> TestReplica {
    ready(Replica::open(FsStore::new(root.to_path_buf()), "", dir, theory()).expect("open"))
}

fn generation(replica: &TestReplica) -> u64 {
    replica
        .db()
        .expect("db")
        .generation()
        .expect("generation")
        .value()
}

fn contains_recipe(replica: &TestReplica, id: u64) -> bool {
    replica
        .db()
        .expect("db")
        .read(|instance| instance.contains_dyn(RECIPE, &[Value::U64(id)]))
        .expect("read")
}

#[test]
fn bootstrap_at_the_zero_vector() {
    let root = temp_dir("rep_bootstrap");
    let local = temp_dir("rep_bootstrap_local");
    let log = TestLog::new(root.clone(), "");
    let replica = open_at(&root, &local.join("r"));
    assert_eq!(replica.provenance(), Provenance::Bootstrap);
    assert_eq!(generation(&replica), 0);
    assert_eq!(
        replica.vector(),
        Vector::from(BTreeMap::from([
            (kitchen_braid(&log.codec), 0),
            (note_braid(&log.codec), 0),
        ]))
    );
    assert!(replica.wedged().is_empty());
}

#[test]
fn open_catches_up_and_reopen_resumes_from_the_local_dir() {
    let root = temp_dir("rep_open");
    let local = temp_dir("rep_open_local");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);
    let notes = note_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 100);
    log.publish(
        kitchen,
        &[insert_recipe(2), insert_step(1, "dice the onion")],
        200,
    );
    log.publish(notes, &[insert_note(1, "shop for onions")], 150);

    let dir = local.join("r");
    let replica = open_at(&root, &dir);
    assert_eq!(generation(&replica), 3);
    assert_eq!(
        replica.vector(),
        Vector::from(BTreeMap::from([(kitchen, 2), (notes, 1)]))
    );
    assert!(contains_recipe(&replica, 2));
    drop(replica);

    log.publish(notes, &[insert_note(2, "preheat")], 300);
    let replica = open_at(&root, &dir);
    assert_eq!(replica.provenance(), Provenance::LocalDir);
    assert_eq!(generation(&replica), 4);
    assert_eq!(replica.vector().at(notes), 2);
}

#[test]
fn refresh_picks_up_new_slots() {
    let root = temp_dir("rep_refresh");
    let local = temp_dir("rep_refresh_local");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 100);
    let mut replica = open_at(&root, &local.join("r"));
    assert_eq!(generation(&replica), 1);

    log.publish(kitchen, &[insert_recipe(2)], 200);
    match replica.refresh().expect("refresh") {
        Refreshed::Vector(vector) => assert_eq!(vector.at(kitchen), 2),
        Refreshed::Refused(refusal) => panic!("refresh refused: {refusal:?}"),
    }
    assert!(contains_recipe(&replica, 2));
}

#[test]
fn crash_window_reopen_absorbs_and_the_vector_catches_up() {
    let root = temp_dir("rep_crash");
    let local = temp_dir("rep_crash_local");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 100);
    let dir = local.join("r");
    let replica = open_at(&root, &dir);
    assert_eq!(generation(&replica), 1);
    drop(replica);

    // The crash window: the engine committed slot 1 but the sidecar
    // bump never landed. Rewind the sidecar one step by hand.
    let braids = log.codec.braids();
    let mut chain = match Chain::read(&dir, braids) {
        SidecarRead::Read(chain) => chain,
        other => panic!("expected Read, got {}", other.identity()),
    };
    chain.entries_mut().insert(kitchen, ChainEntry::GENESIS);
    chain.write_atomic(&dir).expect("rewind sidecar");

    let replica = open_at(&root, &dir);
    assert_eq!(replica.provenance(), Provenance::LocalDir);
    assert_eq!(generation(&replica), 1, "the engine absorbed the replay");
    assert_eq!(replica.vector().at(kitchen), 1, "the vector caught up");
}

#[test]
fn a_phantom_generation_discards_the_directory() {
    let root = temp_dir("rep_phantom");
    let local = temp_dir("rep_phantom_local");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 100);
    let dir = local.join("r");
    let replica = open_at(&root, &dir);
    assert_eq!(generation(&replica), 1);
    drop(replica);

    // A local commit the log never assigned and no pending accounts
    // for: the store is torn by definition.
    let db: Db<SchemaDescriptor> = Db::open(&dir, theory()).expect("raw open");
    db.write(|tx| {
        tx.insert_dyn(NOTE, [[Value::U64(99), Value::String("phantom".into())]])?;
        Ok(())
    })
    .expect("write")
    .expect("accepted");
    drop(db);
    std::fs::write(dir.join("marker"), b"pre-discard").expect("plant marker");

    let replica = open_at(&root, &dir);
    assert_ne!(
        replica.provenance(),
        Provenance::LocalDir,
        "the torn directory was discarded and re-pulled"
    );
    assert!(!dir.join("marker").exists(), "the directory was rebuilt");
    assert_eq!(generation(&replica), 1);
    assert!(
        !replica
            .db()
            .expect("db")
            .read(|instance| {
                instance.contains_dyn(NOTE, &[Value::U64(99), Value::String("phantom".into())])
            })
            .expect("read"),
        "the phantom fact died with the directory"
    );
}

#[test]
fn tip_vs_hole_a_404_below_the_floor_is_a_gap_and_above_it_the_tip() {
    let root = temp_dir("rep_gap");
    let local = temp_dir("rep_gap_local");
    let scratch = temp_dir("rep_gap_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);
    let notes = note_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 100);
    log.publish(notes, &[insert_note(1, "first")], 110);
    let stale_dir = local.join("stale");
    let stale = open_at(&root, &stale_dir);
    assert_eq!(generation(&stale), 2);
    drop(stale);

    log.publish(kitchen, &[insert_recipe(2)], 200);
    log.publish(kitchen, &[insert_recipe(3)], 300);
    let builder = open_at(&root, &local.join("builder"));
    assert_eq!(generation(&builder), 4);
    log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);

    // gc-eligible tail below the floor (kitchen 3, notes 1).
    log.store
        .delete(&log_key("", kitchen, 1))
        .expect("delete slot");
    log.store
        .delete(&log_key("", kitchen, 2))
        .expect("delete slot");

    // The stale dir probes kitchen slot 2: 404 at or below the floor is
    // a gap, so the directory discards and re-opens from the checkpoint.
    let healed = open_at(&root, &stale_dir);
    assert_eq!(healed.provenance(), Provenance::Checkpoint);
    assert_eq!(generation(&healed), 4);
    assert_eq!(
        healed.vector(),
        Vector::from(BTreeMap::from([(kitchen, 3), (notes, 1)]))
    );
    assert!(contains_recipe(&healed, 3));
    // And the same 404 above the floor is the tip, honestly: a fresh
    // refresh finds nothing and changes nothing.
    let mut healed = healed;
    match healed.refresh().expect("refresh") {
        Refreshed::Vector(vector) => assert_eq!(vector.at(kitchen), 3),
        Refreshed::Refused(refusal) => panic!("refresh refused: {refusal:?}"),
    }
}

#[test]
fn the_heartbeat_bounds_hole_detection_staleness_by_law() {
    let root = temp_dir("rep_heartbeat");
    let local = temp_dir("rep_heartbeat_local");
    let scratch = temp_dir("rep_heartbeat_scratch");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 100);
    let mut replica = open_at(&root, &local.join("r"));
    assert_eq!(replica.vector().at(kitchen), 1);

    // Behind the replica's back: two more slots, a checkpoint, and a gc
    // of the tail below it.
    log.publish(kitchen, &[insert_recipe(2)], 200);
    log.publish(kitchen, &[insert_recipe(3)], 300);
    let builder = open_at(&root, &local.join("builder"));
    log.checkpoint(builder.db().expect("db"), &scratch);
    drop(builder);
    log.store
        .delete(&log_key("", kitchen, 2))
        .expect("delete slot");

    // Passes 1..=15: the stale floor reads the 404 at slot 2 as the
    // tip — wrong, but bounded.
    for pass in 1..=15 {
        match replica.refresh().expect("refresh") {
            Refreshed::Vector(vector) => {
                assert_eq!(vector.at(kitchen), 1, "stale at pass {pass}");
            }
            Refreshed::Refused(refusal) => panic!("refresh refused: {refusal:?}"),
        }
    }
    // Pass 16 begins with the manifest poll: the floor moves to 3, the
    // 404 at slot 2 becomes a gap, and the replica heals from the
    // checkpoint.
    match replica.refresh().expect("refresh") {
        Refreshed::Vector(vector) => assert_eq!(vector.at(kitchen), 3),
        Refreshed::Refused(refusal) => panic!("refresh refused: {refusal:?}"),
    }
    assert_eq!(replica.provenance(), Provenance::Checkpoint);
    assert!(contains_recipe(&replica, 3));
}

#[test]
fn a_poisoned_braid_wedges_while_the_others_keep_serving() {
    let root = temp_dir("rep_wedge");
    let local = temp_dir("rep_wedge_local");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);
    let notes = note_braid(&log.codec);

    log.publish(notes, &[insert_note(1, "honest")], 100);
    // A poisoned kitchen slot: its backlink cites a base that never
    // existed.
    let poisoned = log
        .codec
        .encode(
            &BatchHeader {
                fingerprint: *log.codec.fingerprint(),
                braid: kitchen,
                braid_gen: 1,
                prev: [0x77; 32],
                writer: 13,
                timestamp: 100,
            },
            &[insert_recipe(1)],
        )
        .expect("encode");
    log.publish_raw(kitchen, &poisoned, 100);

    let mut replica = open_at(&root, &local.join("r"));
    assert_eq!(replica.vector().at(notes), 1);
    assert_eq!(replica.vector().at(kitchen), 0);
    match replica.wedged().get(&kitchen) {
        Some(Corruption::Refused(ApplyRefusal::ChainMismatch {
            cause: ChainCause::Prev { .. },
            writer,
            ..
        })) => assert_eq!(*writer, 13),
        other => panic!("expected the wedge, got {other:?}"),
    }

    // The healthy braid keeps refreshing past the wedge.
    log.publish(notes, &[insert_note(2, "still serving")], 200);
    match replica.refresh().expect("refresh") {
        Refreshed::Vector(vector) => {
            assert_eq!(vector.at(notes), 2);
            assert_eq!(vector.at(kitchen), 0);
        }
        Refreshed::Refused(refusal) => panic!("refresh refused: {refusal:?}"),
    }
}

#[test]
fn rejected_replay_discards_in_the_open_phase_and_wedges_once_whole() {
    let root = temp_dir("rep_rejected");
    let local = temp_dir("rep_rejected_local");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 100);
    let dir = local.join("r");
    let replica = open_at(&root, &dir);
    assert_eq!(generation(&replica), 1);
    drop(replica);

    // A dishonest writer publishes a slot the engine rejects (a step
    // whose recipe does not exist).
    log.publish(kitchen, &[insert_step(99, "impossible")], 200);

    // The pre-existing dir is in the unproven open phase: the rejection
    // discards it, the re-pull bootstraps a whole store, and the same
    // rejection there earns the corruption-class verdict.
    let replica = open_at(&root, &dir);
    assert_eq!(replica.provenance(), Provenance::Bootstrap);
    assert_eq!(replica.vector().at(kitchen), 1);
    match replica.wedged().get(&kitchen) {
        Some(Corruption::ReplayDiverged { slot, .. }) => assert_eq!(*slot, 2),
        other => panic!("expected the diverged wedge, got {other:?}"),
    }
}

#[test]
fn pending_resolution_keeps_the_identity_and_clears_on_publication() {
    let root = temp_dir("rep_pending");
    let local = temp_dir("rep_pending_local");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 100);
    let dir = local.join("r");
    let replica = open_at(&root, &dir);
    drop(replica);

    // Simulate a writer that applied slot 2 locally, fsynced it into
    // the pending slot, and crashed before the network PUT.
    let (slot, bytes) = log.encode(kitchen, &[insert_recipe(2)], 200);
    assert_eq!(slot, 2);
    let db: Db<SchemaDescriptor> = Db::open(&dir, theory()).expect("raw open");
    let braids = log.codec.braids();
    let mut chain = match Chain::read(&dir, braids) {
        SidecarRead::Read(chain) => chain,
        other => panic!("expected Read, got {}", other.identity()),
    };
    let applied = apply(&db, &mut chain, &log.codec, kitchen, 2, &bytes).expect("apply");
    assert!(matches!(
        applied,
        bumbledb_log::apply::Applied::Advanced { generation: 2 }
    ));
    drop(db);
    // The sidecar the crash left behind: chain still at slot 1 (the
    // manual apply never rewrote the file), the batch in pending.
    let crashed = match Chain::read(&dir, braids) {
        SidecarRead::Read(chain) => chain,
        other => panic!("expected Read, got {}", other.identity()),
    };
    assert_eq!(crashed.position(kitchen).g, 1);
    let Chain::Settled { entries } = crashed else {
        panic!("replica sidecar is Settled");
    };
    let crashed = Chain::Pending {
        entries,
        batch: Pending {
            braid: kitchen,
            slot: 2,
            bytes: bytes.clone(),
        },
    };
    crashed.write_atomic(&dir).expect("write crashed sidecar");

    // A pure replica opens the writer's dir: the applied-but-unpublished
    // pending is exactly the identity's last term.
    let mut replica = open_at(&root, &dir);
    assert_eq!(generation(&replica), 2);
    assert_eq!(replica.vector().at(kitchen), 1);

    // The slot lands in the log; the next refresh resolves the pending
    // and the replay absorbs into the same commit.
    log.publish_raw(kitchen, &bytes, 200);
    match replica.refresh().expect("refresh") {
        Refreshed::Vector(vector) => assert_eq!(vector.at(kitchen), 2),
        Refreshed::Refused(refusal) => panic!("refresh refused: {refusal:?}"),
    }
    assert_eq!(generation(&replica), 2);
    assert!(contains_recipe(&replica, 2));
}

#[test]
fn wait_for_dominates_pointwise_and_refresh_advances_every_braid() {
    let root = temp_dir("rep_wait");
    let local = temp_dir("rep_wait_local");
    let mut log = TestLog::new(root.clone(), "");
    let kitchen = kitchen_braid(&log.codec);
    let notes = note_braid(&log.codec);

    log.publish(kitchen, &[insert_recipe(1)], 100);
    let mut replica = open_at(&root, &local.join("r"));

    log.publish(kitchen, &[insert_recipe(2)], 200);
    log.publish(notes, &[insert_note(1, "target")], 210);
    let target = Vector::from(BTreeMap::from([(kitchen, 2), (notes, 1)]));
    match replica.wait_for(&target).expect("wait") {
        Waited::Reached(vector) => {
            assert!(vector.at(kitchen) >= 2);
            assert!(vector.at(notes) >= 1);
        }
        other => panic!("expected the target, got {other:?}"),
    }

    log.publish(kitchen, &[insert_recipe(3)], 300);
    log.publish(notes, &[insert_note(2, "caught up")], 310);
    match replica.refresh().expect("refresh") {
        Refreshed::Vector(vector) => {
            assert_eq!(vector.at(kitchen), 3);
            assert_eq!(vector.at(notes), 2);
        }
        Refreshed::Refused(refusal) => panic!("refused: {refusal:?}"),
    }
}
