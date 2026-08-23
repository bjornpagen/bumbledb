//! Adoption and deposition: adopt = open as replica, catch up, lease
//! fresh ranges, begin — no registration anywhere. A resident writer
//! hitting a non-byte-equal `Exists` finishes the loss as an ordinary
//! loser, drops to published acks, and surfaces the operational signal
//! naming both writer ids from the headers.

mod lane_e_support;

use std::time::Duration;

use bumbledb::SchemaDescriptor;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{AckMode, Commit, Durability, Options, Writer, WriterOpened};
use lane_e_support::{NOTE, codec, note_braid, note_row, temp_dir, theory};

fn ready(opened: WriterOpened<SchemaDescriptor, FsStore>) -> Writer<SchemaDescriptor, FsStore> {
    match opened {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

#[test]
fn a_deposed_resident_finishes_the_loss_and_drops_to_published_acks() {
    let root = temp_dir("depose");
    let resident_options = Options {
        writer_id: 100,
        ack: AckMode::Local {
            max_pending_batches: 8,
            max_pending_bytes: 1024 * 1024,
        },
        linger: Duration::ZERO,
    };
    let resident = ready(
        Writer::open(
            FsStore::new(root.clone()),
            "",
            &root.join("resident"),
            theory(),
            resident_options,
        )
        .expect("open resident"),
    );

    // The usurper adopts: opens as a replica, catches up, begins. No
    // registration exists; the slot is the fence.
    let usurper = ready(
        Writer::open(
            FsStore::new(root.clone()),
            "",
            &root.join("usurper"),
            theory(),
            Options::new(200),
        )
        .expect("open usurper"),
    );
    assert!(matches!(
        usurper
            .commit(|batch| {
                batch.insert(NOTE, [note_row(10, "usurper")]);
                Ok(())
            })
            .expect("usurper commit"),
        Commit::Accepted { generation: 1, .. }
    ));

    // The resident's next commit acks LocalPending, then its publisher
    // hits the non-byte-equal Exists and learns it was deposed.
    let outcome = resident
        .commit(|batch| {
            batch.insert(NOTE, [note_row(20, "resident")]);
            Ok(())
        })
        .expect("resident commit");
    assert!(matches!(
        outcome,
        Commit::Accepted {
            durability: Durability::LocalPending,
            ..
        }
    ));
    resident.quiesce();

    let deposition = resident.deposition().expect("the operational signal");
    assert_eq!(deposition.resident, 100);
    assert_eq!(deposition.usurper, 200, "both writer ids, from the headers");
    let codec = codec();
    let braid = note_braid(&codec);
    assert_eq!(deposition.braid, braid);
    assert_eq!(deposition.slot, 1);
    assert_eq!(
        resident.backlog(),
        None,
        "the loss finished as an ordinary loser"
    );
    assert_eq!(
        resident.vector()[&braid],
        2,
        "republished behind the winner"
    );
    resident.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(NOTE, &note_row(10, "usurper"))?);
            assert!(instance.contains_dyn(NOTE, &note_row(20, "resident"))?);
            Ok(())
        })
        .expect("read");
    });

    // Acks dropped to published: never a corruption halt, because
    // nothing is corrupt.
    let after = resident
        .commit(|batch| {
            batch.insert(NOTE, [note_row(30, "after")]);
            Ok(())
        })
        .expect("commit after deposition");
    assert!(matches!(
        after,
        Commit::Accepted {
            generation: 3,
            durability: Durability::Published,
            ..
        }
    ));
}
