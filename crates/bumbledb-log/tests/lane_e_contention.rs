//! The contention bound: sixteen consecutive losses surface as
//! `Err::Contention`, and the terminal re-judgment sources the cause —
//! `SlotRace` with the applied batch retained in `pending` when the
//! final re-judgment accepted and the tip was simply busy, `HotKey`
//! with the statement and the offending fact's raw values from the
//! engine's own violation when the racers turned the re-judgment into
//! a rejection.

mod lane_e_support;

use bumbledb::{SchemaDescriptor, Value};
use bumbledb_log::manifest::log_key;
use bumbledb_log::store::ObjectStore;
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{
    Commit, ContentionCause, Error, LOSS_BOUND, Options, Writer, WriterOpened, WriterStep,
};
use lane_e_support::{
    BOOKING, BOOKING_CAPACITY, Competitor, CrashOnce, NOTE, RacingStore, VENUE, codec, note_braid,
    note_row, temp_dir, theory, venue_braid,
};

/// The venue capacity ceiling the shared fixture theory declares.
const CEILING: u64 = 100_000;

/// Racer booking units sized so the sixteenth loss is the first whose
/// re-judgment the ceiling convicts: fifteen racers plus our booking
/// stay under the ceiling, sixteen push it over, and the racers' own
/// replay stays admissible.
const RACER_UNITS: u64 = 6_240;

/// Our booking's units in the hot-key fixture.
const LOSER_UNITS: u64 = 100;

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
    assert_eq!(
        cause,
        ContentionCause::SlotRace { tip: 16 },
        "the terminal re-judgment accepted and the tip was busy"
    );
    assert_eq!(racer.plants(), 16);
    assert_eq!(writer.losses(), u64::from(LOSS_BOUND));
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
fn a_rejecting_terminal_rejudgment_surfaces_hot_key_from_the_violation() {
    let root = temp_dir("hotkey");
    let dir = root.join("w");
    let codec = codec();
    let braid = venue_braid(&codec);
    let (store, racer) = RacingStore::new(
        root.clone(),
        "",
        braid,
        0,
        Competitor::Bookings {
            venue: 1,
            base_units: RACER_UNITS,
        },
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
    // Fifteen racer bookings plus ours fit; the sixteenth crosses the
    // ceiling exactly where the bound is spent, so the terminal
    // re-judgment is the engine's own capacity rejection.
    let racer_fill: u64 = (0..16).map(|seq| RACER_UNITS + seq).sum();
    assert!(racer_fill <= CEILING, "the racers' own replay stays legal");
    assert!(racer_fill - (RACER_UNITS + 15) + LOSER_UNITS <= CEILING);
    assert!(racer_fill + LOSER_UNITS > CEILING);
    racer.seed_from(root.clone());
    racer.arm(16);

    let err = writer
        .commit(|batch| {
            batch.insert(
                BOOKING,
                [Box::from([Value::U64(1), Value::U64(LOSER_UNITS)])],
            );
            Ok(())
        })
        .expect_err("the racers turned the terminal re-judgment into a rejection");
    let Error::Contention { braid: got, cause } = err else {
        panic!("Contention expected, got {err:?}");
    };
    assert_eq!(got, braid);
    let ContentionCause::HotKey { statement, values } = cause else {
        panic!("HotKey expected, got {cause:?}");
    };
    assert_eq!(statement, BOOKING_CAPACITY, "the violation names itself");
    assert!(
        values.contains(&Value::U64(1)),
        "the offending fact's raw values carry the parent determinant: {values:?}"
    );
    assert_eq!(racer.plants(), 16);
    assert_eq!(writer.losses(), u64::from(LOSS_BOUND));
    assert_eq!(
        writer.backlog(),
        None,
        "a rejected terminal re-judgment clears the pending — nothing is owed"
    );
    let generation = writer.with_db(|db| db.generation().expect("generation").value());
    let sum: u64 = writer.vector().values().sum();
    assert_eq!(generation, sum, "whole with nothing pending");
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
