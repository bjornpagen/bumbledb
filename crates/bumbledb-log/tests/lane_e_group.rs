//! Group commit: one loop per braid, concurrent commits partition and
//! queue, a drain packs into ONE batch and one transaction by law —
//! the composite may accept what solo runs would reject — and a
//! composite rejection falls back one-by-one in queue order.

mod lane_e_support;

use std::sync::Barrier;
use std::time::Duration;

use bumbledb::SchemaDescriptor;
use bumbledb_log::manifest::log_key;
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{AckMode, Commit, Options, Writer, WriterOpened};
use lane_e_support::{RECIPE, STEP, codec, kitchen_braid, recipe_row, step_row, temp_dir, theory};

fn open_lingering(
    root: std::path::PathBuf,
    dir: &std::path::Path,
) -> Writer<SchemaDescriptor, FsStore> {
    let options = Options {
        writer_id: 51,
        ack: AckMode::Published,
        linger: Duration::from_millis(300),
    };
    match Writer::open(FsStore::new(root), "", dir, theory(), options).expect("open writer") {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

#[test]
fn a_drain_packs_concurrent_commits_into_one_transaction() {
    let root = temp_dir("pack");
    let dir = root.join("w");
    let writer = open_lingering(root.clone(), &dir);
    let barrier = Barrier::new(2);

    let (step_outcome, recipe_outcome) = std::thread::scope(|scope| {
        let step_task = scope.spawn(|| {
            barrier.wait();
            writer.commit(|batch| {
                batch.insert(STEP, [step_row(7, "mix")]);
                Ok(())
            })
        });
        let recipe_task = scope.spawn(|| {
            barrier.wait();
            writer.commit(|batch| {
                batch.insert(RECIPE, [recipe_row(7, "cake")]);
                Ok(())
            })
        });
        (
            step_task.join().expect("join").expect("commit"),
            recipe_task.join().expect("join").expect("commit"),
        )
    });

    // The step alone would reject (no recipe 7); the drain is one
    // transaction, so the engine judges the composite's final state.
    let Commit::Accepted { generation: g1, .. } = step_outcome else {
        panic!("the composite accepted what a solo run would reject");
    };
    let Commit::Accepted { generation: g2, .. } = recipe_outcome else {
        panic!("accepted expected");
    };
    assert_eq!(g1, 1);
    assert_eq!(g2, 1, "one batch, one generation, one object");

    let codec = codec();
    let braid = kitchen_braid(&codec);
    let store = FsStore::new(root);
    let slot = store
        .get(&log_key("", braid, 1))
        .expect("get")
        .expect("one slot");
    let batch = codec.decode(&slot.bytes).expect("decode");
    assert_eq!(batch.ops.len(), 2, "both callers packed into one batch");
    assert!(store.get(&log_key("", braid, 2)).expect("get").is_none());
}

#[test]
fn a_rejected_composite_falls_back_one_by_one() {
    let root = temp_dir("fallback");
    let dir = root.join("w");
    let writer = open_lingering(root.clone(), &dir);
    let barrier = Barrier::new(2);

    let (guilty, innocent) = std::thread::scope(|scope| {
        let guilty_task = scope.spawn(|| {
            barrier.wait();
            writer.commit(|batch| {
                batch.insert(STEP, [step_row(99, "orphan")]);
                Ok(())
            })
        });
        let innocent_task = scope.spawn(|| {
            barrier.wait();
            writer.commit(|batch| {
                batch.insert(RECIPE, [recipe_row(50, "pie")]);
                Ok(())
            })
        });
        (
            guilty_task.join().expect("join").expect("commit"),
            innocent_task.join().expect("join").expect("commit"),
        )
    });

    assert!(
        matches!(guilty, Commit::Rejected(_)),
        "the guilty write gets its own serial rejection"
    );
    assert!(
        matches!(innocent, Commit::Accepted { generation: 1, .. }),
        "an innocent write never fails for a neighbor's violation"
    );
    let codec = codec();
    let braid = kitchen_braid(&codec);
    let store = FsStore::new(root);
    let slot = store
        .get(&log_key("", braid, 1))
        .expect("get")
        .expect("the innocent write published alone");
    let batch = codec.decode(&slot.bytes).expect("decode");
    assert_eq!(batch.ops.len(), 1);
    assert_eq!(batch.ops[0].relation, RECIPE);
    writer.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(RECIPE, &recipe_row(50, "pie"))?);
            assert!(!instance.contains_dyn(STEP, &step_row(99, "orphan"))?);
            Ok(())
        })
        .expect("read");
    });
}
