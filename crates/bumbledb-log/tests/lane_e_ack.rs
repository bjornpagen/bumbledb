//! Ack modes: `published` default; `local` moving the ack to the end
//! of the local apply with `durability` in the outcome. The loss window
//! is the one pending batch by construction — the pending slot is
//! structurally depth-1, so no knob exists to widen or narrow it.

mod lane_e_support;

use bumbledb::SchemaDescriptor;
use bumbledb_log::manifest::log_key;
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{
    AckMode, Commit, Durability, Error, Options, Writer, WriterOpened, WriterStep,
};
use lane_e_support::{CrashOnce, NOTE, codec, note_braid, note_row, temp_dir, theory};

fn local_options(writer_id: u64) -> Options {
    Options {
        writer_id,
        ack: AckMode::Local,
    }
}

fn ready<
    S: bumbledb_log::store::ObjectStore + 'static,
    H: bumbledb_log::writer::StepHook + 'static,
>(
    opened: WriterOpened<SchemaDescriptor, S, H>,
) -> Writer<SchemaDescriptor, S, H> {
    match opened {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

#[test]
fn local_ack_returns_local_pending_and_publishes_behind() {
    let root = temp_dir("local");
    let dir = root.join("w");
    let writer = ready(
        Writer::open(
            FsStore::new(root.clone()),
            "",
            &dir,
            theory(),
            local_options(41),
        )
        .expect("open writer"),
    );
    let outcome = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "fast")]);
            Ok(())
        })
        .expect("commit");
    assert!(
        matches!(
            outcome,
            Commit::Accepted {
                generation: 1,
                durability: Durability::LocalPending,
                ..
            }
        ),
        "the ack moved to the end of the local apply"
    );
    writer.quiesce();
    let codec = codec();
    let braid = note_braid(&codec);
    let store = FsStore::new(root);
    assert!(
        store.get(&log_key("", braid, 1)).expect("get").is_some(),
        "publication followed the ack"
    );
    assert_eq!(writer.backlog(), None);
}

#[test]
fn crashed_publisher_retains_pending_and_the_next_commit_publishes() {
    let root = temp_dir("detached_crash");
    let dir = root.join("w");
    // The second ApplyLocal is the detached publisher's own replay.
    let writer = ready(
        Writer::open_hooked(
            FsStore::new(root.clone()),
            "",
            &dir,
            theory(),
            local_options(42),
            CrashOnce::new(WriterStep::ApplyLocal, 1),
        )
        .expect("open writer"),
    );
    let outcome = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1, "acked")]);
            Ok(())
        })
        .expect("commit acks before the publisher runs");
    assert!(matches!(
        outcome,
        Commit::Accepted {
            durability: Durability::LocalPending,
            ..
        }
    ));
    writer.quiesce();
    let codec = codec();
    let braid = note_braid(&codec);
    assert_eq!(
        writer.backlog(),
        Some(braid),
        "the publisher crashed before the slot existed"
    );
    let store = FsStore::new(root.clone());
    assert!(store.get(&log_key("", braid, 1)).expect("get").is_none());

    let second = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(2, "later")]);
            Ok(())
        })
        .expect("the next commit publishes the backlog first");
    assert!(matches!(second, Commit::Accepted { generation: 2, .. }));
    writer.quiesce();
    assert_eq!(writer.backlog(), None);
    assert_eq!(writer.vector()[&braid], 2);
    let slot1 = store
        .get(&log_key("", braid, 1))
        .expect("get")
        .expect("retained batch published");
    let batch = codec.decode(&slot1.bytes).expect("decode");
    assert_eq!(batch.header.writer, 42);
}

#[test]
fn body_errors_propagate_without_touching_state() {
    let root = temp_dir("body_err");
    let dir = root.join("w");
    let writer = ready(
        Writer::open(FsStore::new(root), "", &dir, theory(), Options::new(43))
            .expect("open writer"),
    );
    let err = writer
        .commit::<()>(|_batch| Err(Error::EmptyCommit))
        .expect_err("the body's own refusal propagates");
    assert!(matches!(err, Error::EmptyCommit));
    let codec = codec();
    assert_eq!(writer.vector()[&note_braid(&codec)], 0);
}
