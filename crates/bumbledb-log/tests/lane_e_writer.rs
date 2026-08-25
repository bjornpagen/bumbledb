//! The commit discipline and the one loss path: the exact outcome
//! types, the publish law, spanning refusal, the split verb, the
//! ambiguous-PUT absorption, and the loss shapes — identical effects
//! and strict supersets landing Accepted at the current generation
//! through the re-judged net no-op, disjoint-shaped losses landing a
//! re-judged publish with a fresh header at tip+1, conflicts landing
//! the serial Rejected.

mod lane_e_support;

use bumbledb::SchemaDescriptor;
use bumbledb_log::braids::BraidId;
use bumbledb_log::manifest::log_key;
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{
    BraidOutcome, Commit, Durability, Error, Options, Writer, WriterOpened,
};
use lane_e_support::{
    AmbiguousOnce, NOTE, RECIPE, STEP, TestLog, codec, insert, kitchen_braid, note_braid, note_row,
    recipe_row, step_row, temp_dir, theory,
};

type FsWriter = Writer<SchemaDescriptor, FsStore>;

fn open_at(root: std::path::PathBuf, dir: &std::path::Path, writer_id: u64) -> FsWriter {
    match Writer::open(
        FsStore::new(root),
        "",
        dir,
        theory(),
        Options::new(writer_id),
    )
    .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

#[test]
fn accepted_publish_advances_the_chain() {
    let root = temp_dir("accept");
    let dir = root.join("w");
    let writer = open_at(root.clone(), &dir, 11);
    let codec = codec();
    let braid = kitchen_braid(&codec);

    let outcome = writer
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(1, "bread")]);
            Ok(42u32)
        })
        .expect("commit");
    let Commit::Accepted {
        value,
        braid: got,
        generation,
        durability,
    } = outcome
    else {
        panic!("accepted expected");
    };
    assert_eq!(value, 42);
    assert_eq!(got, braid);
    assert_eq!(generation, 1, "braid generation, never the sum");
    assert_eq!(durability, Durability::Published);

    let store = FsStore::new(root);
    let slot = store
        .get(&log_key("", braid, 1))
        .expect("get")
        .expect("slot published");
    let batch = codec.decode(&slot.bytes).expect("published batch decodes");
    assert_eq!(batch.header.writer, 11);
    assert_eq!(batch.header.braid_gen, 1);
    assert_eq!(writer.vector().at(braid), 1);
    assert_eq!(writer.backlog(), None, "pending cleared");
}

#[test]
fn rejection_is_free_of_network() {
    let root = temp_dir("reject");
    let dir = root.join("w");
    let writer = open_at(root.clone(), &dir, 11);
    let codec = codec();
    let braid = kitchen_braid(&codec);

    let outcome = writer
        .commit(|batch| {
            batch.insert(STEP, [step_row(9, "knead")]);
            Ok(())
        })
        .expect("commit");
    assert!(
        matches!(outcome, Commit::Rejected(_)),
        "a step without its recipe violates the containment"
    );
    let store = FsStore::new(root);
    assert!(
        store.get(&log_key("", braid, 1)).expect("get").is_none(),
        "rejections never reach the network"
    );
    assert_eq!(writer.backlog(), None, "pending cleared");
}

#[test]
fn publish_law_net_noop_creates_no_slot() {
    let root = temp_dir("noop");
    let dir = root.join("w");
    let writer = open_at(root.clone(), &dir, 11);
    let codec = codec();
    let braid = kitchen_braid(&codec);

    let first = writer
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(4, "soup")]);
            Ok(())
        })
        .expect("commit");
    assert!(matches!(first, Commit::Accepted { generation: 1, .. }));

    let again = writer
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(4, "soup")]);
            Ok(())
        })
        .expect("commit");
    let Commit::Accepted {
        generation,
        durability,
        ..
    } = again
    else {
        panic!("a net no-op is accepted");
    };
    assert_eq!(generation, 1, "accepted at the current generation");
    assert_eq!(durability, Durability::Published);
    let store = FsStore::new(root);
    assert!(
        store.get(&log_key("", braid, 2)).expect("get").is_none(),
        "the log never gains a no-op slot"
    );
}

#[test]
fn spanning_commit_refuses_naming_the_braids() {
    let root = temp_dir("span");
    let dir = root.join("w");
    let writer = open_at(root, &dir, 11);
    let codec = codec();

    let err = writer
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(1, "bread")]);
            batch.insert(NOTE, [note_row(1, "aside")]);
            Ok(())
        })
        .expect_err("spanning refuses");
    let Error::SpanningCommit { braids } = err else {
        panic!("SpanningCommit expected, got {err:?}");
    };
    let expected: Vec<BraidId> = vec![kitchen_braid(&codec), note_braid(&codec)];
    assert_eq!(braids.as_ref(), expected.as_slice());
}

#[test]
fn empty_body_refuses() {
    let root = temp_dir("empty");
    let dir = root.join("w");
    let writer = open_at(root, &dir, 11);
    let err = writer.commit(|_| Ok(())).expect_err("empty refuses");
    assert!(matches!(err, Error::EmptyCommit));
}

#[test]
fn commit_split_is_the_explicit_verb() {
    let root = temp_dir("split");
    let dir = root.join("w");
    let writer = open_at(root, &dir, 11);
    let codec = codec();

    let (value, outcomes) = writer
        .commit_split(|batch| {
            batch.insert(RECIPE, [recipe_row(1, "bread")]);
            batch.insert(NOTE, [note_row(1, "aside")]);
            Ok("both")
        })
        .expect("split commit");
    assert_eq!(value, "both");
    assert_eq!(outcomes.len(), 2);
    assert!(matches!(
        outcomes[0],
        BraidOutcome::Accepted {
            braid,
            generation: 1,
            durability: Durability::Published,
        } if braid == kitchen_braid(&codec)
    ));
    assert!(matches!(
        outcomes[1],
        BraidOutcome::Accepted {
            braid,
            generation: 1,
            durability: Durability::Published,
        } if braid == note_braid(&codec)
    ));
    assert_eq!(writer.vector().at(kitchen_braid(&codec)), 1);
    assert_eq!(writer.vector().at(note_braid(&codec)), 1);
}

#[test]
fn ambiguous_put_absorbed_by_fetch_and_compare() {
    let root = temp_dir("ambiguous");
    let dir = root.join("w");
    let store = AmbiguousOnce::new(root.clone());
    let opened = Writer::open(store, "", &dir, theory(), Options::new(11)).expect("open");
    let WriterOpened::Ready(writer) = opened else {
        panic!("ready expected");
    };
    let outcome = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(5, "ours")]);
            Ok(())
        })
        .expect("commit");
    assert!(
        matches!(
            outcome,
            Commit::Accepted {
                generation: 1,
                durability: Durability::Published,
                ..
            }
        ),
        "byte-equal Exists is our own earlier PUT, absorbed"
    );
    assert_eq!(writer.losses(), 0, "absorption is not a loss");
}

#[test]
fn identical_effects_race_lands_accepted_at_the_winners_generation() {
    let root = temp_dir("identical_race");
    let dir = root.join("w");
    let writer = open_at(root.clone(), &dir, 11);
    let codec = codec();
    let braid = kitchen_braid(&codec);

    let mut log = TestLog::attach(root.clone(), "");
    log.publish(braid, &[insert(RECIPE, recipe_row(5, "same"))], 10);

    let outcome = writer
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(5, "same")]);
            Ok(())
        })
        .expect("commit");
    let Commit::Accepted {
        generation,
        durability,
        ..
    } = outcome
    else {
        panic!("a loss whose effects the winner performed reports Accepted");
    };
    assert_eq!(
        generation, 1,
        "the re-judgment nets a no-op and the publish law answers at the winner's generation"
    );
    assert_eq!(durability, Durability::Published);
    assert_eq!(writer.losses(), 1, "one loss, one re-judgment");
    let store = FsStore::new(root);
    assert!(
        store.get(&log_key("", braid, 2)).expect("get").is_none(),
        "the log never gains a no-op slot"
    );
    assert_eq!(
        writer.vector().at(braid),
        1,
        "the winner's slot is accounted"
    );
}

#[test]
fn strict_superset_race_lands_accepted_with_the_residue_present() {
    let root = temp_dir("superset_race");
    let dir = root.join("w");
    let writer = open_at(root.clone(), &dir, 11);
    let codec = codec();
    let braid = kitchen_braid(&codec);

    let mut log = TestLog::attach(root.clone(), "");
    log.publish(
        braid,
        &[
            insert(RECIPE, recipe_row(5, "same")),
            insert(RECIPE, recipe_row(6, "extra")),
        ],
        10,
    );

    let outcome = writer
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(5, "same")]);
            Ok(())
        })
        .expect("commit");
    assert!(
        matches!(
            outcome,
            Commit::Accepted {
                generation: 1,
                durability: Durability::Published,
                ..
            }
        ),
        "the winner strictly contains us; the re-judgment nets a no-op"
    );
    assert_eq!(writer.losses(), 1);
    assert_eq!(writer.vector().at(braid), 1);
    writer
        .with_db(|db| {
            db.read(|instance| {
                assert!(instance.contains_dyn(RECIPE, &recipe_row(5, "same"))?);
                assert!(instance.contains_dyn(RECIPE, &recipe_row(6, "extra"))?);
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
    let store = FsStore::new(root);
    assert!(store.get(&log_key("", braid, 2)).expect("get").is_none());
}

#[test]
fn conflict_loss_rejudges_to_the_serial_rejection() {
    let root = temp_dir("conflict");
    let dir = root.join("w");
    let writer = open_at(root.clone(), &dir, 11);
    let codec = codec();
    let braid = kitchen_braid(&codec);

    let mut log = TestLog::attach(root.clone(), "");
    log.publish(braid, &[insert(RECIPE, recipe_row(1, "winner"))], 10);

    let outcome = writer
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(1, "loser")]);
            Ok(())
        })
        .expect("commit");
    assert!(
        matches!(outcome, Commit::Rejected(_)),
        "exactly the verdict serial execution would have produced"
    );
    assert_eq!(writer.losses(), 1, "one loss, one re-judgment");
    assert_eq!(
        writer.vector().at(braid),
        1,
        "winner-current after the loss"
    );
    writer
        .with_db(|db| {
            db.read(|instance| {
                assert!(instance.contains_dyn(RECIPE, &recipe_row(1, "winner"))?);
                assert!(!instance.contains_dyn(RECIPE, &recipe_row(1, "loser"))?);
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
    let store = FsStore::new(root);
    assert!(store.get(&log_key("", braid, 2)).expect("get").is_none());
}

#[test]
fn disjoint_loss_rejudges_and_publishes_at_tip_plus_one() {
    let root = temp_dir("disjoint");
    let dir = root.join("w");
    let writer = open_at(root.clone(), &dir, 11);
    let codec = codec();
    let braid = kitchen_braid(&codec);

    let winner_ops = vec![insert(RECIPE, recipe_row(2, "theirs"))];
    let mut log = TestLog::attach(root.clone(), "");
    log.publish(braid, &winner_ops, 10);

    let outcome = writer
        .commit(|batch| {
            batch.insert(RECIPE, [recipe_row(3, "ours")]);
            Ok(())
        })
        .expect("commit");
    let Commit::Accepted {
        generation,
        durability,
        ..
    } = outcome
    else {
        panic!("a disjoint-shaped loss lands");
    };
    assert_eq!(generation, 2, "the re-judged publish lands at tip+1");
    assert_eq!(durability, Durability::Published);
    assert_eq!(writer.losses(), 1, "one loss, one re-judgment");
    let store = FsStore::new(root);
    let slot2 = store
        .get(&log_key("", braid, 2))
        .expect("get")
        .expect("published");
    let batch = codec.decode(&slot2.bytes).expect("decode");
    assert_eq!(batch.header.writer, 11, "a fresh header of our own");
    assert_eq!(batch.header.braid_gen, 2);
    let slot1 = store
        .get(&log_key("", braid, 1))
        .expect("get")
        .expect("the winner's slot");
    assert_eq!(
        batch.header.prev,
        *blake3::hash(&slot1.bytes).as_bytes(),
        "the fresh header cites the winner as its base"
    );
    writer
        .with_db(|db| {
            db.read(|instance| {
                assert!(instance.contains_dyn(RECIPE, &recipe_row(2, "theirs"))?);
                assert!(instance.contains_dyn(RECIPE, &recipe_row(3, "ours"))?);
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
}

#[test]
fn adoption_catches_up_and_continues_the_chain() {
    let root = temp_dir("adopt");
    let dir_a = root.join("a");
    let dir_b = root.join("b");
    let writer_a = open_at(root.clone(), &dir_a, 1);
    let codec = codec();
    let braid = note_braid(&codec);

    assert!(matches!(
        writer_a
            .commit(|batch| {
                batch.insert(NOTE, [note_row(1, "first")]);
                Ok(())
            })
            .expect("commit"),
        Commit::Accepted { generation: 1, .. }
    ));
    drop(writer_a);

    let writer_b = open_at(root, &dir_b, 2);
    assert_eq!(writer_b.vector().at(braid), 1, "adopt = open as replica");
    assert!(matches!(
        writer_b
            .commit(|batch| {
                batch.insert(NOTE, [note_row(2, "second")]);
                Ok(())
            })
            .expect("commit"),
        Commit::Accepted { generation: 2, .. }
    ));
    writer_b
        .with_db(|db| {
            db.read(|instance| {
                assert!(instance.contains_dyn(NOTE, &note_row(1, "first"))?);
                assert!(instance.contains_dyn(NOTE, &note_row(2, "second"))?);
                Ok(())
            })
            .expect("read");
        })
        .expect("db");
}
