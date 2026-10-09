//! Migrations as log entries: the ledger and the bundle, optimistic and
//! frozen migrations, takeover, sticky rejection, and old code stopping at
//! a schema it does not ship.

use bumbledb_log::{
    Bundle, Command, Input, Millis, Mode, Precondition, Refusal, RequestId, Seq, Settled, Ticket,
};

use crate::sim::{Ask, Faults, World, check, population, receipt_of};
use crate::support::{
    bundle, bundle2, descriptor, descriptor2, items, migration_id, schema, schema2, tags,
};

fn open(world: &mut World, client: usize) -> Settled {
    world.start(client);
    world.drain();
    let ticket = world
        .asks
        .iter()
        .rev()
        .find(|(_, (owner, ask))| *owner == client && matches!(ask, Ask::Open))
        .map(|(ticket, _)| *ticket)
        .expect("an open ticket");
    world.results[&ticket].settled.clone()
}

fn head(world: &World, client: usize) -> Seq {
    world
        .machine(client)
        .replica()
        .state()
        .expect("a state")
        .head
        .seq
}

fn submit(world: &mut World, client: usize, request: u8, changes: bumbledb::ChangeSet) -> Settled {
    let command = Command::seal(RequestId([request; 16]), Precondition::None, changes);
    let ticket = world.submit(client, command.request(), command);
    world.drain();
    world.results[&ticket].settled.clone()
}

fn migrate(world: &mut World, client: usize, base: Seq, rows: bumbledb::ChangeSet) -> Settled {
    let ticket = world.ticket(client, Ask::Migrate);
    let population = population(migration_id("0001_tags"), base, rows);
    world.await_ticket(client, ticket, Input::Migrate(ticket, population))
}

fn freeze(world: &mut World, client: usize, lease: u64) -> Settled {
    let ticket: Ticket = world.ticket(client, Ask::Freeze);
    world.await_ticket(client, ticket, Input::Freeze(ticket, lease))
}

#[test]
fn a_new_database_starts_at_the_first_migration_and_migrates_forward() {
    let mut world = World::new(20, 2, &bundle2(), Faults::NONE);
    assert_eq!(open(&mut world, 0), Settled::Opened { pending: 1 });
    assert_eq!(
        submit(&mut world, 0, 1, items(&schema(), &[(1, 1)], &[])),
        Settled::Refused(Refusal::MigrationPending { next: 1 })
    );
    let base = head(&world, 0);
    assert_eq!(
        migrate(&mut world, 0, base, tags(&schema2(), &[(1, 7)])),
        Settled::Migrated(Seq::new(2).unwrap())
    );
    let decided = submit(&mut world, 0, 2, tags(&schema2(), &[(2, 8)]));
    assert_eq!(receipt_of(&decided).seq, Seq::new(3).unwrap());
    assert_eq!(open(&mut world, 1), Settled::Opened { pending: 0 });
    check(&world);
}

#[test]
fn old_code_stops_before_a_migration_it_does_not_ship() {
    let mut world = World::with_bundles(21, vec![bundle2(), bundle()], Faults::NONE);
    open(&mut world, 0);
    migrate(&mut world, 0, Seq::GENESIS, tags(&schema2(), &[]));
    assert_eq!(
        open(&mut world, 1),
        Settled::Refused(Refusal::SchemaAdvanced)
    );
    assert_eq!(
        world.machine(1).replica().state().unwrap().head.seq,
        Seq::GENESIS,
        "the old replica keeps the state before the migration"
    );
    let ticket = world.ticket(1, Ask::Sync);
    world.input(1, Input::Sync(ticket));
    assert_eq!(
        world.results[&ticket].settled,
        Settled::Refused(Refusal::SchemaAdvanced)
    );
    check(&world);
}

#[test]
fn a_rejected_migration_is_sticky_for_code_that_ships_it() {
    let mut world = World::with_bundles(22, vec![bundle2(), bundle2()], Faults::NONE);
    open(&mut world, 0);
    let rejected = migrate(
        &mut world,
        0,
        Seq::GENESIS,
        tags(&schema2(), &[(1, 1), (1, 2)]),
    );
    let Settled::Refused(Refusal::MigrationRejected(rejection)) = rejected else {
        panic!("expected a rejection, got {rejected:?}");
    };
    assert_eq!(rejection.migration, migration_id("0001_tags"));
    assert_eq!(
        open(&mut world, 1),
        Settled::Refused(Refusal::MigrationRejected(rejection))
    );
    check(&world);

    let fixed = Bundle::new(vec![
        (migration_id("0000_init"), descriptor()),
        (migration_id("0001_tags_fixed"), descriptor2()),
    ])
    .unwrap();
    let mut world = World::with_bundles(23, vec![bundle2(), fixed], Faults::NONE);
    open(&mut world, 0);
    migrate(
        &mut world,
        0,
        Seq::GENESIS,
        tags(&schema2(), &[(1, 1), (1, 2)]),
    );
    assert_eq!(open(&mut world, 1), Settled::Opened { pending: 1 });
}

#[test]
fn a_population_from_an_older_head_is_refused() {
    let mut world = World::new(24, 1, &bundle2(), Faults::NONE);
    open(&mut world, 0);
    let ticket = world.ticket(0, Ask::Migrate);
    let stale = population(
        migration_id("0001_tags"),
        Seq::new(5).unwrap(),
        tags(&schema2(), &[]),
    );
    assert_eq!(
        world.await_ticket(0, ticket, Input::Migrate(ticket, stale)),
        Settled::Refused(Refusal::Stale { head: Seq::GENESIS })
    );
    let ticket = world.ticket(0, Ask::Migrate);
    let other = population(
        migration_id("0009_other"),
        Seq::GENESIS,
        tags(&schema2(), &[]),
    );
    assert_eq!(
        world.await_ticket(0, ticket, Input::Migrate(ticket, other)),
        Settled::Refused(Refusal::NotPending)
    );
}

#[test]
fn a_freeze_holds_old_writers_until_its_deadline_on_store_time() {
    let mut world = World::with_bundles(25, vec![bundle2(), bundle()], Faults::NONE);
    open(&mut world, 0);
    open(&mut world, 1);
    assert_eq!(
        freeze(&mut world, 0, 1_000),
        Settled::Frozen(Seq::new(2).unwrap())
    );

    let held = submit(&mut world, 1, 1, items(&schema(), &[(1, 1)], &[]));
    let Settled::Refused(Refusal::Frozen { deadline }) = held else {
        panic!("expected a freeze, got {held:?}");
    };
    let Mode::Frozen { since, .. } = &world.machine(1).replica().state().unwrap().head.mode else {
        panic!("the old writer applied the freeze");
    };
    assert_eq!(deadline, Millis(since.0 + 1_000));

    world.now = deadline.0 + 1;
    let decided = submit(&mut world, 1, 2, items(&schema(), &[(1, 1)], &[]));
    assert_eq!(
        receipt_of(&decided).seq,
        Seq::new(4).unwrap(),
        "after the thaw at 3"
    );

    let base = head(&world, 0);
    assert_eq!(
        migrate(&mut world, 0, base, tags(&schema2(), &[])),
        Settled::Refused(Refusal::Stale {
            head: Seq::new(3).unwrap()
        })
    );
    world.sync(0);
    let base = head(&world, 0);
    assert_eq!(
        migrate(&mut world, 0, base, tags(&schema2(), &[])),
        Settled::Migrated(Seq::new(5).unwrap())
    );
    assert_eq!(
        submit(&mut world, 1, 3, items(&schema(), &[(2, 2)], &[])),
        Settled::Refused(Refusal::SchemaAdvanced)
    );
    check(&world);
}

#[test]
fn another_new_writer_may_finish_a_frozen_migration() {
    let mut world = World::with_bundles(26, vec![bundle2(), bundle2()], Faults::NONE);
    open(&mut world, 0);
    freeze(&mut world, 0, 60_000);
    world.kill(0, false);
    assert_eq!(open(&mut world, 1), Settled::Opened { pending: 1 });
    let base = head(&world, 1);
    assert_eq!(
        migrate(&mut world, 1, base, tags(&schema2(), &[(3, 3)])),
        Settled::Migrated(Seq::new(3).unwrap())
    );
    assert_eq!(
        freeze(&mut world, 1, 10),
        Settled::Refused(Refusal::NotPending)
    );
    check(&world);
}

#[test]
fn a_bundle_that_disagrees_with_the_ledger_refuses_to_open() {
    let mut world = World::new(27, 1, &bundle(), Faults::NONE);
    open(&mut world, 0);
    world.kill(0, true);
    let mut other = migration_id("0000_init");
    other.hash.0[0] ^= 1;
    let diverged = Bundle::new(vec![(other, descriptor())]).unwrap();
    let mut world2 = World::with_bundles(28, vec![diverged], Faults::NONE);
    world2.log = world.log.clone();
    assert_eq!(
        open(&mut world2, 0),
        Settled::Refused(Refusal::MigrationsDiverged { index: 0 })
    );
}

#[test]
fn a_new_migration_waits_for_a_lost_flights_image_upload() {
    let mut world = World::with_bundles(29, vec![bundle2(), bundle2()], Faults::NONE);
    open(&mut world, 0);
    open(&mut world, 1);
    let first = world.ticket(0, Ask::Migrate);
    let lost = population(
        migration_id("0001_tags"),
        Seq::GENESIS,
        tags(&schema2(), &[(1, 1)]),
    );
    world.input(0, Input::Migrate(first, lost));
    let is_upload = |request: &bumbledb_log::IoRequest| request.key.starts_with("mig/");
    assert!(world.find(0, is_upload).is_some());

    // Another writer freezes at the slot the first flight wanted.
    let frozen = world.ticket(1, Ask::Freeze);
    world.input(1, Input::Freeze(frozen, 60_000));
    while let Some(index) = world.find(1, |_| true) {
        world.execute_as(index, crate::sim::Fate::Answered);
    }
    let sync = world.ticket(0, Ask::Sync);
    world.input(0, Input::Sync(sync));
    while let Some(index) = world.find(0, |request| !request.key.starts_with("mig/")) {
        world.execute_as(index, crate::sim::Fate::Answered);
    }
    assert_eq!(
        world.results[&first].settled,
        Settled::Refused(Refusal::Stale {
            head: Seq::new(2).unwrap()
        })
    );

    let second = world.ticket(0, Ask::Migrate);
    let retry = population(
        migration_id("0001_tags"),
        Seq::new(2).unwrap(),
        tags(&schema2(), &[(2, 2)]),
    );
    world.input(0, Input::Migrate(second, retry));
    assert_eq!(
        world
            .issued()
            .filter(|(client, request)| *client == 0 && is_upload(request))
            .count(),
        1,
        "the new image waits for the old upload"
    );
    world.drain();
    assert_eq!(
        world.results[&second].settled,
        Settled::Migrated(Seq::new(3).unwrap())
    );
    check(&world);
}
