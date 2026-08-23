//! The contention bound: sixteen consecutive live losses surface as
//! `Err::Contention` with the cause that actually exhausted the bound —
//! `SlotRace` for fully-disjoint racers, `HotKey` with the raw
//! determinant values for conflicts — and the applied batch stays in
//! `pending`: the store is whole, reads serve, and publication retries
//! on the next commit.

mod lane_e_support;

use bumbledb::{SchemaDescriptor, Value};
use bumbledb_log::manifest::log_key;
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{
    Commit, ContentionCause, Error, Options, Writer, WriterOpened, WriterStep,
};
use lane_e_support::{
    BOOKING, BOOKING_CAPACITY, Competitor, CrashOnce, NOTE, RacingStore, VENUE, codec, note_braid,
    note_row, temp_dir, theory, venue_braid,
};

fn ready<S: bumbledb_log::store::ObjectStore + 'static>(
    opened: WriterOpened<SchemaDescriptor, S>,
) -> Writer<SchemaDescriptor, S> {
    match opened {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

#[test]
fn sixteen_disjoint_losses_surface_slot_race_and_keep_pending() {
    let root = temp_dir("slotrace");
    let dir = root.join("w");
    let codec = codec();
    let braid = note_braid(&codec);
    let (store, racer) = RacingStore::new(root.clone(), "", braid, 16, Competitor::Notes);
    let writer =
        ready(Writer::open(store, "", &dir, theory(), Options::new(21)).expect("open writer"));

    let err = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(1_000, "starved")]);
            Ok(())
        })
        .expect_err("the bound converts starvation into a typed signal");
    let Error::Contention { braid: got, cause } = err else {
        panic!("Contention expected, got {err:?}");
    };
    assert_eq!(got, braid);
    assert_eq!(cause, ContentionCause::SlotRace { tip: 16 });
    assert_eq!(racer.plants(), 16);
    assert_eq!(
        writer.backlog(),
        Some(braid),
        "the applied commit is never dropped because the tip was busy"
    );
    writer.with_db(|db| {
        db.read(|instance| {
            assert!(
                instance.contains_dyn(NOTE, &note_row(1_000, "starved"))?,
                "reads serve the applied pending"
            );
            Ok(())
        })
        .expect("read");
    });

    // The racer is spent; the next commit republishes the retained
    // batch first, then lands its own.
    let outcome = writer
        .commit(|batch| {
            batch.insert(NOTE, [note_row(2_000, "later")]);
            Ok(())
        })
        .expect("commit after the race");
    assert!(matches!(outcome, Commit::Accepted { generation: 18, .. }));
    assert_eq!(writer.backlog(), None);
    assert_eq!(writer.vector()[&braid], 18);
    let fs = FsStore::new(root);
    let retained = fs
        .get(&log_key("", braid, 17))
        .expect("get")
        .expect("the retained batch published at the tip");
    let batch = codec.decode(&retained.bytes).expect("decode");
    assert_eq!(batch.header.writer, 21);
}

#[test]
fn sixteen_conflicts_surface_hot_key_with_raw_determinants() {
    let root = temp_dir("hotkey");
    let dir = root.join("w");
    let codec = codec();
    let braid = venue_braid(&codec);
    let (store, racer) = RacingStore::new(
        root.clone(),
        "",
        braid,
        0,
        Competitor::Bookings { venue: 1 },
    );
    let writer =
        ready(Writer::open(store, "", &dir, theory(), Options::new(22)).expect("open writer"));

    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(VENUE, [Box::from([Value::U64(1)])]);
                Ok(())
            })
            .expect("venue setup"),
        Commit::Accepted { generation: 1, .. }
    ));
    racer.seed_from(root.clone());
    racer.arm(16);

    let err = writer
        .commit(|batch| {
            batch.insert(BOOKING, [Box::from([Value::U64(1), Value::U64(5)])]);
            Ok(())
        })
        .expect_err("conflicts exhausted the bound");
    let Error::Contention { braid: got, cause } = err else {
        panic!("Contention expected, got {err:?}");
    };
    assert_eq!(got, braid);
    let ContentionCause::HotKey { statement, values } = cause else {
        panic!("HotKey expected, got {cause:?}");
    };
    assert_eq!(statement, Some(BOOKING_CAPACITY));
    assert_eq!(
        values.as_ref(),
        &[Value::U64(1)],
        "the loser owns its raw determinant values"
    );
    assert_eq!(writer.backlog(), Some(braid));
}

#[test]
fn open_ending_in_contention_keeps_pending_and_serves() {
    let root = temp_dir("open_contended");
    let dir = root.join("w");
    let codec = codec();
    let braid = note_braid(&codec);

    // Crash after the local apply: the commit is real and unpublished.
    let crashed = match Writer::open_hooked(
        FsStore::new(root.clone()),
        "",
        &dir,
        theory(),
        Options::new(23),
        CrashOnce::new(WriterStep::ApplyLocal, 0),
    )
    .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    };
    let err = crashed
        .commit(|batch| {
            batch.insert(NOTE, [note_row(7, "pending")]);
            Ok(())
        })
        .expect_err("crash injected");
    assert!(matches!(err, Error::InjectedCrash { .. }));
    drop(crashed);

    let (store, racer) = RacingStore::new(root.clone(), "", braid, 16, Competitor::Notes);
    let reopened =
        ready(Writer::open(store, "", &dir, theory(), Options::new(23)).expect("reopen writer"));
    assert_eq!(
        reopened.backlog(),
        Some(braid),
        "an open ending in Contention keeps its pending"
    );
    assert_eq!(racer.plants(), 16);
    reopened.with_db(|db| {
        db.read(|instance| {
            assert!(instance.contains_dyn(NOTE, &note_row(7, "pending"))?);
            Ok(())
        })
        .expect("read");
    });

    let outcome = reopened
        .commit(|batch| {
            batch.insert(NOTE, [note_row(8, "after")]);
            Ok(())
        })
        .expect("publication retries on the next commit");
    assert!(matches!(outcome, Commit::Accepted { .. }));
    assert_eq!(reopened.backlog(), None);
}
