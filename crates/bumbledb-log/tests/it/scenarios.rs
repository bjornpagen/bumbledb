//! Scripted protocol paths: each test drives the machine through one case
//! with chosen request fates, then checks the world against the replay.

use bumbledb::Schema;
use bumbledb_log::{
    Body, Bucket, Bundle, Command, Entry, Input, IoBody, Op, Outcome, Precondition, Refusal,
    RequestId, Revision, Seq, Settled, Ticket,
};

use crate::sim::{Ask, Fate, Faults, World, check, receipt_of};
use crate::support::{bundle, items, schema};

fn opened(seed: u64, clients: usize) -> World {
    let mut world = World::new(seed, clients, &bundle(), Faults::NONE);
    for client in 0..clients {
        world.start(client);
        world.drain();
    }
    world
}

fn command(schema: &Schema, request: u8, adds: &[(u64, u64)]) -> Command {
    Command::seal(
        RequestId([request; 16]),
        Precondition::None,
        items(schema, adds, &[]),
    )
}

fn submit(world: &mut World, client: usize, command: Command) -> Ticket {
    world.submit(client, command.request(), command)
}

fn settled(world: &World, ticket: Ticket) -> &Settled {
    &world.results[&ticket].settled
}

fn is_put(request: &bumbledb_log::IoRequest) -> bool {
    request.bucket == Bucket::Log && matches!(request.op, Op::PutIfAbsent(_))
}

fn put_body(world: &World, client: usize) -> Vec<u8> {
    world
        .issued()
        .find_map(|(owner, request)| match &request.op {
            Op::PutIfAbsent(IoBody::Bytes(bytes)) if owner == client => Some(bytes.clone()),
            _ => None,
        })
        .expect("an entry PUT is issued")
}

#[test]
fn a_lone_writer_records_every_kind_of_decision() {
    let schema = schema();
    let mut world = opened(1, 1);
    let opens: Vec<_> = world
        .results
        .values()
        .map(|result| result.settled.clone())
        .collect();
    assert_eq!(opens, [Settled::Opened { pending: 0 }]);

    let first = submit(&mut world, 0, command(&schema, 1, &[(1, 10)]));
    world.drain();
    let receipt = receipt_of(settled(&world, first)).clone();
    assert_eq!(receipt.seq, Seq::new(2).unwrap());
    assert_eq!(receipt.revision, Revision(1));
    assert!(matches!(receipt.outcome, Outcome::Committed(delta) if delta.added() == 1));

    let same = submit(&mut world, 0, command(&schema, 2, &[(1, 10)]));
    let clash = submit(&mut world, 0, command(&schema, 3, &[(1, 11)]));
    let stale = world.submit(
        0,
        RequestId([4; 16]),
        Command::seal(
            RequestId([4; 16]),
            Precondition::ExactRevision(Revision(0)),
            items(&schema, &[(2, 1)], &[]),
        ),
    );
    world.drain();
    assert_eq!(receipt_of(settled(&world, same)).outcome, Outcome::NoChange);
    assert!(matches!(
        receipt_of(settled(&world, clash)).outcome,
        Outcome::InvariantRejected(_)
    ));
    assert_eq!(
        receipt_of(settled(&world, stale)).outcome,
        Outcome::PreconditionFailed {
            expected: Revision(0),
            observed: Revision(1)
        }
    );

    // A decided request answers from its receipt without touching the store.
    let again = submit(&mut world, 0, command(&schema, 1, &[(1, 10)]));
    assert_eq!(world.issued().count(), 0);
    assert_eq!(settled(&world, again), &Settled::Decided(receipt));

    let reused = submit(&mut world, 0, command(&schema, 1, &[(9, 9)]));
    assert!(matches!(
        settled(&world, reused),
        Settled::Refused(Refusal::RequestReused(_))
    ));
    check(&world);
}

#[test]
fn commands_arriving_during_a_write_share_the_next_entry() {
    let schema = schema();
    let mut world = opened(2, 1);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    assert_eq!(
        world
            .issued()
            .filter(|(_, request)| is_put(request))
            .count(),
        1
    );
    let b = submit(&mut world, 0, command(&schema, 2, &[(2, 2)]));
    let c = submit(&mut world, 0, command(&schema, 3, &[(3, 3)]));
    assert_eq!(world.issued().count(), 1, "one write in flight, no timer");
    world.execute_as(0, Fate::Answered);
    assert_eq!(receipt_of(settled(&world, a)).seq, Seq::new(2).unwrap());
    world.drain();
    let (b, c) = (
        receipt_of(settled(&world, b)),
        receipt_of(settled(&world, c)),
    );
    assert_eq!(b.seq, Seq::new(3).unwrap());
    assert_eq!(c.seq, b.seq);
    assert_eq!((b.revision, c.revision), (Revision(2), Revision(3)));
    check(&world);
}

#[test]
fn a_lost_race_applies_the_winner_and_decides_again_at_the_next_slot() {
    let schema = schema();
    let mut world = opened(3, 2);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    let b = submit(&mut world, 1, command(&schema, 2, &[(1, 2)]));
    let b_put = world.find(1, is_put).unwrap();
    world.execute_as(b_put, Fate::Answered);
    let a_put = world.find(0, is_put).unwrap();
    world.execute_as(a_put, Fate::Answered);
    world.drain();
    let (a, b) = (
        receipt_of(settled(&world, a)),
        receipt_of(settled(&world, b)),
    );
    assert_eq!(b.seq, Seq::new(2).unwrap());
    assert_eq!(a.seq, Seq::new(3).unwrap());
    assert!(
        matches!(a.outcome, Outcome::InvariantRejected(_)),
        "decided again against the winner's state"
    );
    check(&world);
}

#[test]
fn an_unanswered_write_is_recognized_by_reading_it_back() {
    let schema = schema();
    let mut world = opened(4, 1);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    world.execute_as(0, Fate::Unanswered);
    assert!(!world.results.contains_key(&a));
    world.drain();
    assert_eq!(receipt_of(settled(&world, a)).seq, Seq::new(2).unwrap());
    assert_eq!(world.log.len(), 2, "no second copy of the entry");
    check(&world);
}

#[test]
fn a_failed_write_is_put_again_with_identical_bytes() {
    let schema = schema();
    let mut world = opened(5, 1);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    let first = put_body(&world, 0);
    world.execute_as(0, Fate::Lost);
    world.execute_as(0, Fate::Answered);
    assert_eq!(put_body(&world, 0), first, "the same nonce and bytes");
    world.drain();
    assert_eq!(receipt_of(settled(&world, a)).seq, Seq::new(2).unwrap());
    check(&world);
}

#[test]
fn a_stale_writer_catches_up_through_its_conflicts() {
    let schema = schema();
    let mut world = opened(6, 2);
    for request in 1..=5 {
        let ticket = submit(
            &mut world,
            0,
            command(&schema, request, &[(u64::from(request), 0)]),
        );
        world.drain();
        assert!(matches!(settled(&world, ticket), Settled::Decided(_)));
    }
    let late = submit(&mut world, 1, command(&schema, 9, &[(9, 9)]));
    world.drain();
    assert_eq!(receipt_of(settled(&world, late)).seq, Seq::new(7).unwrap());
    check(&world);
}

#[test]
fn closing_mid_write_leaves_the_outcome_to_resolve() {
    let schema = schema();
    let mut world = opened(7, 1);
    let a = submit(&mut world, 0, command(&schema, 1, &[(1, 1)]));
    let queued = submit(&mut world, 0, command(&schema, 2, &[(2, 2)]));
    world.execute_as(0, Fate::Unanswered);
    world.input(0, Input::Close);
    assert_eq!(settled(&world, a), &Settled::Refused(Refusal::Unknown));
    assert_eq!(
        settled(&world, queued),
        &Settled::Refused(Refusal::NotSubmitted)
    );
    world.drain();
    world.kill(0, false);
    world.start(0);
    world.drain();
    for (request, decided) in [(1u8, true), (2, false)] {
        let request = RequestId([request; 16]);
        let ticket = world.ticket(0, Ask::Resolve(request));
        let resolved = world.await_ticket(0, ticket, Input::Resolve(ticket, request));
        assert!(matches!(resolved, Settled::Resolved(found) if found.is_some() == decided));
    }
    check(&world);
}

#[test]
fn a_sync_sees_every_entry_written_before_it() {
    let schema = schema();
    let mut world = opened(8, 2);
    for request in 1..=3 {
        submit(
            &mut world,
            0,
            command(&schema, request, &[(u64::from(request), 1)]),
        );
        world.drain();
    }
    assert_eq!(world.sync(1), Settled::Synced(Seq::new(4).unwrap()));
    check(&world);
}

#[test]
fn a_cold_replica_installs_the_newest_checkpoint_and_replays_only_the_tail() {
    let schema = schema();
    let mut world = World::new(9, 2, &bundle(), Faults::NONE);
    world.clients[0].config.checkpoint.every = 4;
    world.clients[1].config.probe_window = 4;
    world.start(0);
    world.drain();
    for request in 1..=13 {
        submit(
            &mut world,
            0,
            command(&schema, request, &[(u64::from(request % 5), 0)]),
        );
        world.drain();
    }
    let checkpoints = crate::sim::checkpoint_keys(&world);
    let seqs: Vec<u64> = checkpoints.iter().map(|key| key.seq.get()).collect();
    assert_eq!(seqs, [12, 8], "every 4 entries, the newest 2 kept");
    world.start(1);
    let mut probed = Vec::new();
    while world.issued().count() > 0 {
        for (client, request) in world.issued() {
            if client == 1 && request.bucket == Bucket::Log {
                probed.push(request.key.clone());
            }
        }
        world.execute_as(0, Fate::Answered);
    }
    assert!(
        probed
            .iter()
            .all(|key| key.as_str() > bumbledb_log::log_key(Seq::new(12).unwrap()).as_str()),
        "nothing at or below the checkpoint is fetched: {probed:?}"
    );
    let head = world.machine(1).replica().state().unwrap().head.seq;
    assert_eq!(head, Seq::new(14).unwrap());
    check(&world);
}

#[test]
fn every_log_object_parses_and_carries_a_fresh_nonce() {
    let schema = schema();
    let mut world = opened(10, 2);
    for request in 1..=6 {
        submit(
            &mut world,
            usize::from(request % 2),
            command(&schema, request, &[(1, 1)]),
        );
        world.drain();
    }
    let mut nonces = std::collections::BTreeSet::new();
    for bytes in world.log_entries() {
        let entry = Entry::parse(&schema, &bytes).unwrap();
        assert!(nonces.insert(entry.nonce), "nonces are unique");
        if let Body::Commands(decided) = &entry.body {
            assert!(!decided.is_empty());
        }
    }
    check(&world);
}

#[test]
fn a_database_that_does_not_exist_is_not_created_without_an_identity() {
    let mut world = World::new(11, 1, &bundle(), Faults::NONE);
    world.clients[0].config.create = None;
    world.start(0);
    world.drain();
    let opens: Vec<_> = world
        .results
        .values()
        .map(|result| result.settled.clone())
        .collect();
    assert_eq!(opens, [Settled::Refused(Refusal::NotFound)]);
    assert!(world.log.is_empty());
}

#[test]
fn two_creators_agree_on_one_genesis() {
    let schema = schema();
    let mut world = World::new(12, 2, &bundle(), Faults::NONE);
    world.start(0);
    world.start(1);
    world.drain();
    assert_eq!(world.log.len(), 1);
    let databases: Vec<_> = (0..2)
        .map(|client| {
            world
                .machine(client)
                .replica()
                .state()
                .unwrap()
                .head
                .database
        })
        .collect();
    assert_eq!(databases[0], databases[1]);
    submit(&mut world, 1, command(&schema, 1, &[(1, 1)]));
    world.drain();
    check(&world);
}

#[test]
fn bundles_must_be_nonempty() {
    assert!(matches!(
        Bundle::new(Vec::new()),
        Err(bumbledb_log::BundleError::Empty)
    ));
}
