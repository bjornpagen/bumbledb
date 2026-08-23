//! Pending recovery at open: the three forced arms, driven by the
//! fault-injection seam. Every crash prefix recovers through idempotent
//! replay and create-or-compare, with the one instrument (generation vs
//! vector sum) making every call.

mod lane_e_support;

use bumbledb::SchemaDescriptor;
use bumbledb_log::codec::OpKind;
use bumbledb_log::manifest::log_key;
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{Commit, Error, NoFaults, Options, Writer, WriterOpened, WriterStep};
use lane_e_support::{
    CrashOnce, NOTE, STEP, TestLog, codec, insert, note_braid, note_row, step_row, temp_dir, theory,
};

type FsWriter<H> = Writer<SchemaDescriptor, FsStore, H>;

fn open_crashing(
    root: std::path::PathBuf,
    dir: &std::path::Path,
    hook: CrashOnce,
) -> FsWriter<CrashOnce> {
    match Writer::open_hooked(
        FsStore::new(root),
        "",
        dir,
        theory(),
        Options::new(31),
        hook,
    )
    .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

fn reopen(root: std::path::PathBuf, dir: &std::path::Path) -> FsWriter<NoFaults> {
    match Writer::open(FsStore::new(root), "", dir, theory(), Options::new(31))
        .expect("reopen writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("reopen refused: {refusal:?}"),
    }
}

#[test]
fn arm_one_resurrected_never_judged_batch_rejects_and_publishes_nothing() {
    let root = temp_dir("arm1");
    let dir = root.join("w");
    let writer = open_crashing(
        root.clone(),
        &dir,
        CrashOnce::new(WriterStep::PendingWrite, 0),
    );
    let err = writer
        .commit(|batch| {
            batch.insert(STEP, [step_row(9, "orphan")]);
            Ok(())
        })
        .expect_err("crash injected");
    assert!(matches!(
        err,
        Error::InjectedCrash {
            step: WriterStep::PendingWrite
        }
    ));
    drop(writer);

    let recovered = reopen(root.clone(), &dir);
    assert_eq!(recovered.backlog(), None, "pending cleared");
    let codec = codec();
    let braid = codec.braids().braid_of(STEP).expect("step braid");
    let store = FsStore::new(root);
    assert!(
        store.get(&log_key("", braid, 1)).expect("get").is_none(),
        "a born-rejected batch never reaches the log"
    );
    recovered.with_db(|db| {
        db.read(|instance| {
            assert!(!instance.contains_dyn(STEP, &step_row(9, "orphan"))?);
            Ok(())
        })
        .expect("read");
    });
}

#[test]
fn arm_two_born_noop_clears_and_publishes_nothing() {
    let root = temp_dir("arm2");
    let dir = root.join("w");
    let writer = open_crashing(
        root.clone(),
        &dir,
        CrashOnce::new(WriterStep::PendingWrite, 1),
    );
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(NOTE, [note_row(1, "first")]);
                Ok(())
            })
            .expect("setup commit"),
        Commit::Accepted { generation: 1, .. }
    ));
    let err = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "first")]);
            Ok(())
        })
        .expect_err("crash injected on the duplicate");
    assert!(matches!(err, Error::InjectedCrash { .. }));
    drop(writer);

    let recovered = reopen(root.clone(), &dir);
    assert_eq!(recovered.backlog(), None);
    let codec = codec();
    let braid = note_braid(&codec);
    assert_eq!(recovered.vector()[&braid], 1);
    let store = FsStore::new(root);
    assert!(
        store.get(&log_key("", braid, 2)).expect("get").is_none(),
        "the publish law again: a born no-op publishes nothing"
    );
}

#[test]
fn arm_three_applied_unpublished_commit_publishes_at_recovery() {
    let root = temp_dir("arm3");
    let dir = root.join("w");
    let writer = open_crashing(
        root.clone(),
        &dir,
        CrashOnce::new(WriterStep::ApplyLocal, 0),
    );
    let err = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "survives")]);
            Ok(())
        })
        .expect_err("crash injected");
    assert!(matches!(
        err,
        Error::InjectedCrash {
            step: WriterStep::ApplyLocal
        }
    ));
    drop(writer);

    let recovered = reopen(root.clone(), &dir);
    assert_eq!(recovered.backlog(), None, "published and cleared");
    let codec = codec();
    let braid = note_braid(&codec);
    assert_eq!(recovered.vector()[&braid], 1);
    let store = FsStore::new(root);
    let slot = store
        .get(&log_key("", braid, 1))
        .expect("get")
        .expect("recovery published the applied commit");
    let batch = codec.decode(&slot.bytes).expect("decode");
    assert_eq!(batch.ops.len(), 1);
    assert_eq!(batch.ops[0].kind, OpKind::Insert);
    recovered.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(NOTE, &note_row(1, "survives"))?);
            Ok(())
        })
        .expect("read");
    });
}

#[test]
fn crash_mid_publish_absorbs_the_byte_equal_slot() {
    let root = temp_dir("midpub");
    let dir = root.join("w");
    let writer = open_crashing(root.clone(), &dir, CrashOnce::new(WriterStep::PutLog, 0));
    let err = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(2, "created")]);
            Ok(())
        })
        .expect_err("crash injected after the create landed");
    assert!(matches!(
        err,
        Error::InjectedCrash {
            step: WriterStep::PutLog
        }
    ));
    drop(writer);

    let recovered = reopen(root.clone(), &dir);
    assert_eq!(recovered.backlog(), None);
    let codec = codec();
    let braid = note_braid(&codec);
    assert_eq!(recovered.vector()[&braid], 1);
    let store = FsStore::new(root);
    assert!(store.get(&log_key("", braid, 2)).expect("get").is_none());
}

#[test]
fn crash_between_chain_advance_and_pending_clear_recovers() {
    let root = temp_dir("advance");
    let dir = root.join("w");
    let writer = open_crashing(
        root.clone(),
        &dir,
        CrashOnce::new(WriterStep::ChainAdvance, 0),
    );
    let err = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(3, "advanced")]);
            Ok(())
        })
        .expect_err("crash injected");
    assert!(matches!(err, Error::InjectedCrash { .. }));
    drop(writer);

    let recovered = reopen(root, &dir);
    assert_eq!(recovered.backlog(), None);
    let codec = codec();
    assert_eq!(recovered.vector()[&note_braid(&codec)], 1);
}

#[test]
fn stale_writer_catches_up_through_history_then_attempts_at_tip() {
    let root = temp_dir("stale");
    let dir = root.join("w");
    let writer = open_crashing(
        root.clone(),
        &dir,
        CrashOnce::new(WriterStep::ApplyLocal, 0),
    );
    let err = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "mine")]);
            Ok(())
        })
        .expect_err("crash injected");
    assert!(matches!(err, Error::InjectedCrash { .. }));
    drop(writer);

    let codec = codec();
    let braid = note_braid(&codec);
    let mut log = TestLog::attach(root.clone(), "");
    log.publish(braid, &[insert(NOTE, note_row(100, "w1"))], 5);
    log.publish(braid, &[insert(NOTE, note_row(101, "w2"))], 6);

    let recovered = reopen(root.clone(), &dir);
    assert_eq!(recovered.backlog(), None, "republished at the tip");
    assert_eq!(recovered.vector()[&braid], 3);
    let store = FsStore::new(root);
    let slot3 = store
        .get(&log_key("", braid, 3))
        .expect("get")
        .expect("our commit landed at tip+1");
    let batch = codec.decode(&slot3.bytes).expect("decode");
    assert_eq!(batch.header.writer, 31);
    recovered.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(NOTE, &note_row(1, "mine"))?);
            assert!(instance.contains_dyn(NOTE, &note_row(100, "w1"))?);
            assert!(instance.contains_dyn(NOTE, &note_row(101, "w2"))?);
            Ok(())
        })
        .expect("read");
    });
}
