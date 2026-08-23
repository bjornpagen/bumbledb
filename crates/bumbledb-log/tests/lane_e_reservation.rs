//! Capacity reservations as a schema idiom: `reserve_capacity` is
//! sugar over an ordinary insert into the declared reservation
//! relation; mints are judged inserts, spends are ordinary commits,
//! nothing is special-cased.

mod lane_e_support;

use bumbledb::{SchemaDescriptor, Value};
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::writer::{Commit, Error, Options, Writer, WriterOpened};
use lane_e_support::{
    BOOKING, BOOKING_CAPACITY, HOLD, HOLD_CAPACITY, RECIPE_KEY, VENUE, temp_dir, theory,
};

fn open_at(root: std::path::PathBuf, dir: &std::path::Path) -> Writer<SchemaDescriptor, FsStore> {
    match Writer::open(FsStore::new(root), "", dir, theory(), Options::new(71))
        .expect("open writer")
    {
        WriterOpened::Ready(writer) => writer,
        WriterOpened::Refused(refusal) => panic!("open refused: {refusal:?}"),
    }
}

fn hold_row(venue: u64, units: u64, expiry: u64) -> Box<[Value]> {
    Box::from([Value::U64(venue), Value::U64(units), Value::U64(expiry)])
}

#[test]
fn mint_is_an_ordinary_judged_insert() {
    let root = temp_dir("mint");
    let dir = root.join("w");
    let writer = open_at(root, &dir);

    assert!(matches!(
        writer
            .commit(|batch| {
                batch.insert(VENUE, [Box::from([Value::U64(1)])]);
                Ok(())
            })
            .expect("venue"),
        Commit::Accepted { .. }
    ));
    let outcome = writer
        .commit(|batch| {
            batch.reserve_capacity(HOLD_CAPACITY, &[Value::U64(1)], 3, 999)?;
            Ok(())
        })
        .expect("mint");
    assert!(matches!(outcome, Commit::Accepted { .. }));
    writer.with_db(|db| {
        db.read(|instance| {
            assert!(
                instance.contains_dyn(HOLD, &hold_row(1, 3, 999))?,
                "the reservation is a row, nothing more"
            );
            Ok(())
        })
        .expect("read");
    });
}

#[test]
fn spend_deletes_the_reservation_and_inserts_children_in_one_commit() {
    let root = temp_dir("spend");
    let dir = root.join("w");
    let writer = open_at(root, &dir);

    writer
        .commit(|batch| {
            batch.insert(VENUE, [Box::from([Value::U64(1)])]);
            Ok(())
        })
        .expect("venue");
    writer
        .commit(|batch| {
            batch.reserve_capacity(HOLD_CAPACITY, &[Value::U64(1)], 3, 999)?;
            Ok(())
        })
        .expect("mint");
    let outcome = writer
        .commit(|batch| {
            batch.delete(HOLD, [hold_row(1, 3, 999)]);
            batch.insert(BOOKING, [Box::from([Value::U64(1), Value::U64(3)])]);
            Ok(())
        })
        .expect("spend");
    assert!(matches!(outcome, Commit::Accepted { .. }));
    writer.with_db(|db| {
        db.read(|instance| {
            assert!(!instance.contains_dyn(HOLD, &hold_row(1, 3, 999))?);
            assert!(instance.contains_dyn(BOOKING, &[Value::U64(1), Value::U64(3)] as &[Value])?);
            Ok(())
        })
        .expect("read");
    });
}

#[test]
fn mint_pays_the_capacity_judgment() {
    let root = temp_dir("priced");
    let dir = root.join("w");
    let writer = open_at(root, &dir);

    writer
        .commit(|batch| {
            batch.insert(VENUE, [Box::from([Value::U64(1)])]);
            Ok(())
        })
        .expect("venue");
    assert!(matches!(
        writer
            .commit(|batch| {
                batch.reserve_capacity(HOLD_CAPACITY, &[Value::U64(1)], 60_000, 10)?;
                Ok(())
            })
            .expect("first mint"),
        Commit::Accepted { .. }
    ));
    assert!(
        matches!(
            writer
                .commit(|batch| {
                    batch.reserve_capacity(HOLD_CAPACITY, &[Value::U64(1)], 50_000, 20)?;
                    Ok(())
                })
                .expect("second mint"),
            Commit::Rejected(_)
        ),
        "a mint is priced against real slack by the ordinary judgment"
    );
}

#[test]
fn statements_without_the_reservation_shape_refuse() {
    let root = temp_dir("shape");
    let dir = root.join("w");
    let writer = open_at(root, &dir);

    let err = writer
        .commit::<()>(|batch| {
            batch.reserve_capacity(BOOKING_CAPACITY, &[Value::U64(1)], 1, 1)?;
            Ok(())
        })
        .expect_err("booking has no expiry field");
    assert!(matches!(
        err,
        Error::ReservationShape { statement } if statement == BOOKING_CAPACITY
    ));

    let err = writer
        .commit::<()>(|batch| {
            batch.reserve_capacity(RECIPE_KEY, &[Value::U64(1)], 1, 1)?;
            Ok(())
        })
        .expect_err("a key statement is not a capacity");
    assert!(matches!(
        err,
        Error::ReservationShape { statement } if statement == RECIPE_KEY
    ));
}
