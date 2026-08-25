//! Pending recovery at open: the three forced arms, driven by the
//! fault-injection seam. Every crash prefix recovers through idempotent
//! replay and create-or-compare, with the one instrument (generation vs
//! vector sum) making every call.

mod lane_e_support;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use bumbledb::SchemaDescriptor;
use bumbledb_log::codec::{BatchHeader, OpKind};
use bumbledb_log::manifest::log_key;
use bumbledb_log::sidecar::{Chain, Pending, SidecarRead};
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
    assert_eq!(recovered.backlog(), None, "published at the tip");
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

const RECOVERY_ROLE: &str = "LANE_E_SCRIPTED_PENDING";
const RECOVERY_ROOT: &str = "LANE_E_SCRIPTED_ROOT";
const RECOVERY_DIR: &str = "LANE_E_SCRIPTED_DIR";

fn plant_scripted_pending(root: &Path, dir: &Path) {
    drop(reopen(root.to_path_buf(), dir));
    let codec = codec();
    let braid = note_braid(&codec);
    let bytes = codec
        .encode(
            &BatchHeader {
                fingerprint: *codec.fingerprint(),
                braid,
                braid_gen: 1,
                prev: [0u8; 32],
                writer: 31,
                timestamp: 1,
            },
            &[insert(NOTE, note_row(7, "scripted"))],
        )
        .expect("encode scripted pending");
    let settled = match Chain::read(dir, codec.braids()) {
        SidecarRead::Read(chain) => chain,
        other => panic!("expected Read after birth, got {}", other.identity()),
    };
    let Chain::Settled { entries } = settled else {
        panic!("birth is Settled");
    };
    Chain::Pending {
        entries,
        batch: Pending {
            braid,
            slot: 1,
            bytes,
        },
    }
    .write_atomic(dir)
    .expect("plant Pending at slot 1");
}

fn run_scripted_recovery_child() {
    let root = PathBuf::from(std::env::var_os(RECOVERY_ROOT).expect("root env"));
    let dir = PathBuf::from(std::env::var_os(RECOVERY_DIR).expect("dir env"));
    let recovered = reopen(root.clone(), &dir);
    assert_eq!(
        recovered.backlog(),
        None,
        "open published the Pending arm to Settled"
    );
    let codec = codec();
    let braid = note_braid(&codec);
    assert_eq!(recovered.vector()[&braid], 1);
    recovered.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(NOTE, &note_row(7, "scripted"))?);
            Ok(())
        })
        .expect("read");
    });
    let store = FsStore::new(root);
    assert!(
        store
            .get(&log_key("", braid, 1))
            .expect("get slot 1")
            .is_some(),
        "the scripted slot is present"
    );
    assert!(
        store
            .get(&log_key("", braid, 2))
            .expect("get slot 2")
            .is_none(),
        "recovery never double-publishes"
    );
    println!("LANE_E_RECOVERY recovered arm=Settled generation=1 slot=present");
}

/// Finding 122: the pending batch is written by the test script to a
/// known slot, then a second process recovers it. No sleep-and-hope.
#[test]
fn scripted_pending_recovers_in_a_second_process() {
    if std::env::var(RECOVERY_ROLE).is_ok() {
        run_scripted_recovery_child();
        return;
    }

    let root = temp_dir("scripted_mp");
    let dir = root.join("w");
    plant_scripted_pending(&root, &dir);
    match Chain::read(&dir, codec().braids()) {
        SidecarRead::Read(Chain::Pending { batch, .. }) => {
            assert_eq!(batch.slot, 1, "the test script wrote Pending at slot 1")
        }
        other => panic!("expected planted Pending, got {other:?}"),
    }

    let exe = std::env::current_exe().expect("current test binary");
    let child = Command::new(&exe)
        .args([
            "scripted_pending_recovers_in_a_second_process",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(RECOVERY_ROLE, "recover")
        .env(RECOVERY_ROOT, root.as_os_str())
        .env(RECOVERY_DIR, dir.as_os_str())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn recovery child");
    let out = child.wait_with_output().expect("child exit");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "recovery child failed: {stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("LANE_E_RECOVERY recovered arm=Settled generation=1 slot=present"),
        "child names Settled and the present slot: {stdout}"
    );

    match Chain::read(&dir, codec().braids()) {
        SidecarRead::Read(Chain::Settled { .. }) => {}
        other => panic!("parent re-read is Settled, got {other:?}"),
    }
}
